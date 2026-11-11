//! Pairing of entities across two measurements (code-constraints 5 and 11).
//!
//! The ratchet compares an entity with itself. This module decides which
//! entity of the before side is "itself", and how far that decision can be
//! trusted:
//!
//! - **Exact**: the same scope, path (after applying the renames Git
//!   reported), qualified name, parameter text and ordinal. An exact pair
//!   may excuse existing debt and may show regression.
//! - **Moved**: a routine that disappeared from one file and appeared in
//!   another with the same name, the same parameter text and an identical
//!   normalized body, with exactly one candidate on each side. A move keeps
//!   the debt status, so that splitting a file is not reported as new
//!   breaches.
//! - **Signature**: within one file and qualified name, exactly one routine
//!   disappeared and one appeared (the parameters changed). This pair can
//!   show regression only; it never excuses a breach as pre-existing.
//!
//! Anything else is unpaired: judged on the absolute limits alone.

use std::collections::{BTreeMap, BTreeSet};

use super::facts::{CodeFacts, Completeness, EntityKey, Scope};

/// How far a pairing can be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Trust {
    Exact,
    Moved,
    Signature,
}

impl Trust {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Moved => "moved",
            Self::Signature => "signature",
        }
    }

    /// Whether the pair may classify a breach as pre-existing.
    #[must_use]
    pub const fn excuses_debt(self) -> bool {
        matches!(self, Self::Exact | Self::Moved)
    }
}

/// The before side of a comparison.
#[derive(Debug, Clone, Copy)]
pub struct Before<'a> {
    pub facts: &'a CodeFacts,
    /// What the before side is, for the report: a revision or the baseline.
    pub identity: &'a str,
    /// `(old path, new path)` pairs Git reported as renames.
    pub renames: &'a [(String, String)],
}

/// The before value of one metric of an after entity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Earlier {
    pub value: f64,
    pub trust: Trust,
}

/// Before values by after entity and metric name.
#[derive(Debug, Clone, Default)]
pub struct Pairing {
    values: BTreeMap<(EntityKey, String), Earlier>,
    paired: BTreeMap<EntityKey, Trust>,
}

impl Pairing {
    /// The before value of `metric` for the after entity, when it is paired
    /// and the metric was measured on the before side.
    #[must_use]
    pub fn earlier(&self, entity: &EntityKey, metric: &str) -> Option<Earlier> {
        self.values
            .get(&(entity.clone(), metric.to_string()))
            .copied()
    }

    /// How the after entity is paired, if it is.
    #[must_use]
    pub fn trust(&self, entity: &EntityKey) -> Option<Trust> {
        self.paired.get(entity).copied()
    }

    /// Number of paired entities.
    #[must_use]
    pub fn len(&self) -> usize {
        self.paired.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.paired.is_empty()
    }
}

fn entities(facts: &CodeFacts) -> BTreeSet<&EntityKey> {
    facts
        .observations
        .iter()
        .map(|observation| &observation.entity)
        .collect()
}

fn hashes(facts: &CodeFacts) -> BTreeMap<&EntityKey, &str> {
    facts
        .fingerprints
        .iter()
        .map(|fingerprint| (&fingerprint.entity, fingerprint.hash.as_str()))
        .collect()
}

/// Paths whose measurement is complete; a file that failed to parse on
/// either side takes part in no comparison.
fn complete_paths(facts: &CodeFacts) -> BTreeSet<&str> {
    facts
        .coverage
        .iter()
        .filter(|cell| cell.metric.is_none() && cell.state == Completeness::Complete)
        .map(|cell| cell.path.as_str())
        .collect()
}

/// The last segment of a qualified name: `a::B::run` and `B.run` are `run`.
fn simple_name(qualified: &str) -> &str {
    qualified.rsplit([':', '.']).next().unwrap_or(qualified)
}

/// The entities of a side that take part in pairing, in key order.
fn comparable_entities(facts: &CodeFacts) -> Vec<&EntityKey> {
    let complete = complete_paths(facts);
    entities(facts)
        .into_iter()
        .filter(|entity| complete.contains(entity.path.as_str()))
        .collect()
}

/// The routines of `list` that no earlier step has paired.
fn open_routines<'a>(list: &[&'a EntityKey], used: &BTreeSet<&EntityKey>) -> Vec<&'a EntityKey> {
    list.iter()
        .copied()
        .filter(|entity| entity.scope == Scope::Routine && !used.contains(entity))
        .collect()
}

/// Normalized body hashes of both sides, to recognise a moved routine.
struct Bodies<'a> {
    before: BTreeMap<&'a EntityKey, &'a str>,
    after: BTreeMap<&'a EntityKey, &'a str>,
}

impl Bodies<'_> {
    /// Same name, parameter text and normalized body. A routine without a
    /// fingerprint equals nothing.
    fn same(&self, earlier: &EntityKey, later: &EntityKey) -> bool {
        simple_name(&earlier.qualified_name) == simple_name(&later.qualified_name)
            && earlier.parameters == later.parameters
            && match (self.before.get(earlier), self.after.get(later)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
    }
}

/// A before entity, the after entity it is paired with, and how.
type Match<'a> = (&'a EntityKey, &'a EntityKey, Trust);

/// (file, qualified name) -> (routines that went, routines that came).
type Groups<'a> = BTreeMap<(String, String), (Vec<&'a EntityKey>, Vec<&'a EntityKey>)>;

/// The pairing steps and what they have paired so far. The steps run in a
/// fixed order, each on what the earlier ones left open.
struct Matcher<'a> {
    /// New path by old path, as Git reported the renames.
    renamed: BTreeMap<&'a str, &'a str>,
    before: Vec<&'a EntityKey>,
    after: Vec<&'a EntityKey>,
    matches: Vec<Match<'a>>,
    used_before: BTreeSet<&'a EntityKey>,
    used_after: BTreeSet<&'a EntityKey>,
}

impl<'a> Matcher<'a> {
    fn new(before: &Before<'a>, after: &'a CodeFacts) -> Self {
        Self {
            renamed: before
                .renames
                .iter()
                .map(|(old, new)| (old.as_str(), new.as_str()))
                .collect(),
            before: comparable_entities(before.facts),
            after: comparable_entities(after),
            matches: Vec::new(),
            used_before: BTreeSet::new(),
            used_after: BTreeSet::new(),
        }
    }

    fn new_path(&self, old: &str) -> String {
        self.renamed.get(old).copied().unwrap_or(old).to_string()
    }

    /// The before entity as it would be named on the after side.
    fn mapped(&self, entity: &EntityKey) -> EntityKey {
        let path = self.new_path(&entity.path);
        EntityKey {
            // A file entity is named by its path.
            qualified_name: if entity.scope == Scope::File {
                path.clone()
            } else {
                entity.qualified_name.clone()
            },
            path,
            ..entity.clone()
        }
    }

    /// Records a pair and takes both entities out of the later steps.
    fn record(&mut self, earlier: &'a EntityKey, later: &'a EntityKey, trust: Trust) {
        self.matches.push((earlier, later, trust));
        self.used_before.insert(earlier);
        self.used_after.insert(later);
    }

    /// 1. Exact identity.
    fn pair_exact(&mut self) {
        let after_index: BTreeSet<&'a EntityKey> = self.after.iter().copied().collect();
        let found: Vec<(&'a EntityKey, &'a EntityKey)> = self
            .before
            .iter()
            .filter_map(|earlier| {
                let later = after_index.get(&self.mapped(earlier))?;
                Some((*earlier, *later))
            })
            .collect();
        for (earlier, later) in found {
            self.record(earlier, later, Trust::Exact);
        }
    }

    /// 2. Moved routines: same name, parameters and normalized body, one
    ///    candidate on each side.
    fn pair_moved(&mut self, bodies: &Bodies<'_>) {
        let open_before = open_routines(&self.before, &self.used_before);
        let open_after = open_routines(&self.after, &self.used_after);
        let mut moves: Vec<(&'a EntityKey, &'a EntityKey)> = Vec::new();
        for later in &open_after {
            let candidates: Vec<&'a EntityKey> = open_before
                .iter()
                .copied()
                .filter(|earlier| bodies.same(earlier, later))
                .collect();
            let [earlier] = candidates.as_slice() else {
                continue;
            };
            let rivals = open_after
                .iter()
                .filter(|other| bodies.same(earlier, other))
                .count();
            if rivals == 1 {
                moves.push((*earlier, *later));
            }
        }
        for (earlier, later) in moves {
            self.record(earlier, later, Trust::Moved);
        }
    }

    /// 3. One disappeared and one appeared under the same file and name.
    fn pair_signature(&mut self) {
        let mut groups: Groups<'a> = BTreeMap::new();
        for earlier in open_routines(&self.before, &self.used_before) {
            groups
                .entry((self.new_path(&earlier.path), earlier.qualified_name.clone()))
                .or_default()
                .0
                .push(earlier);
        }
        for later in open_routines(&self.after, &self.used_after) {
            groups
                .entry((later.path.clone(), later.qualified_name.clone()))
                .or_default()
                .1
                .push(later);
        }
        for (gone, came) in groups.values() {
            if let ([earlier], [later]) = (gone.as_slice(), came.as_slice()) {
                self.matches.push((earlier, later, Trust::Signature));
            }
        }
    }
}

/// Before values of every paired entity, by metric name. Only the same
/// producer and metric definition are comparable.
fn before_values(before: &CodeFacts, after: &CodeFacts, matches: &[Match<'_>]) -> Pairing {
    let mut pairing = Pairing::default();
    for (earlier, later, trust) in matches.iter().copied() {
        pairing.paired.insert(later.clone(), trust);
        for was in before
            .observations
            .iter()
            .filter(|observation| &observation.entity == earlier)
        {
            let comparable = after.observations.iter().any(|now| {
                &now.entity == later && now.metric == was.metric && now.producer == was.producer
            });
            if comparable {
                pairing.values.insert(
                    (later.clone(), was.metric.name.clone()),
                    Earlier {
                        value: was.value,
                        trust,
                    },
                );
            }
        }
    }
    pairing
}

/// Pairs the entities of `after` with those of the before side and
/// collects the before values of every paired metric.
#[must_use]
pub fn pair(before: &Before<'_>, after: &CodeFacts) -> Pairing {
    let mut matcher = Matcher::new(before, after);
    matcher.pair_exact();
    matcher.pair_moved(&Bodies {
        before: hashes(before.facts),
        after: hashes(after),
    });
    matcher.pair_signature();
    before_values(before.facts, after, &matcher.matches)
}
