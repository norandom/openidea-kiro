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

/// Pairs the entities of `after` with those of the before side and
/// collects the before values of every paired metric.
#[must_use]
pub fn pair(before: &Before<'_>, after: &CodeFacts) -> Pairing {
    let renamed: BTreeMap<&str, &str> = before
        .renames
        .iter()
        .map(|(old, new)| (old.as_str(), new.as_str()))
        .collect();
    let new_path = |old: &str| renamed.get(old).copied().unwrap_or(old).to_string();
    let before_complete = complete_paths(before.facts);
    let after_complete = complete_paths(after);

    let before_entities: Vec<&EntityKey> = entities(before.facts)
        .into_iter()
        .filter(|entity| before_complete.contains(entity.path.as_str()))
        .collect();
    let after_entities: Vec<&EntityKey> = entities(after)
        .into_iter()
        .filter(|entity| after_complete.contains(entity.path.as_str()))
        .collect();

    // The before entity as it would be named on the after side.
    let mapped = |entity: &EntityKey| {
        let path = new_path(&entity.path);
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
    };

    let mut matches: Vec<(&EntityKey, &EntityKey, Trust)> = Vec::new();
    let mut used_before: BTreeSet<&EntityKey> = BTreeSet::new();
    let mut used_after: BTreeSet<&EntityKey> = BTreeSet::new();

    // 1. Exact identity.
    let after_index: BTreeSet<&EntityKey> = after_entities.iter().copied().collect();
    for earlier in &before_entities {
        let target = mapped(earlier);
        if let Some(found) = after_index.get(&target).copied() {
            matches.push((earlier, found, Trust::Exact));
            used_before.insert(earlier);
            used_after.insert(found);
        }
    }

    // 2. Moved routines: same name, parameters and normalized body, one
    //    candidate on each side.
    let before_hash = hashes(before.facts);
    let after_hash = hashes(after);
    let unmatched = |list: &[&'_ EntityKey], used: &BTreeSet<&EntityKey>| -> Vec<usize> {
        list.iter()
            .enumerate()
            .filter(|(_, entity)| entity.scope == Scope::Routine && !used.contains(*entity))
            .map(|(index, _)| index)
            .collect()
    };
    let same_body = |earlier: &EntityKey, later: &EntityKey| {
        simple_name(&earlier.qualified_name) == simple_name(&later.qualified_name)
            && earlier.parameters == later.parameters
            && match (before_hash.get(earlier), after_hash.get(later)) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
    };
    let open_before = unmatched(&before_entities, &used_before);
    let open_after = unmatched(&after_entities, &used_after);
    let mut moves: Vec<(usize, usize)> = Vec::new();
    for later_index in &open_after {
        let Some(later) = after_entities.get(*later_index).copied() else {
            continue;
        };
        let candidates: Vec<usize> = open_before
            .iter()
            .copied()
            .filter(|index| {
                before_entities
                    .get(*index)
                    .is_some_and(|earlier| same_body(earlier, later))
            })
            .collect();
        if let [only] = candidates.as_slice() {
            let Some(earlier) = before_entities.get(*only).copied() else {
                continue;
            };
            let rivals = open_after
                .iter()
                .filter(|index| {
                    after_entities
                        .get(**index)
                        .is_some_and(|other| same_body(earlier, other))
                })
                .count();
            if rivals == 1 {
                moves.push((*only, *later_index));
            }
        }
    }
    for (earlier_index, later_index) in moves {
        if let (Some(earlier), Some(later)) = (
            before_entities.get(earlier_index).copied(),
            after_entities.get(later_index).copied(),
        ) {
            matches.push((earlier, later, Trust::Moved));
            used_before.insert(earlier);
            used_after.insert(later);
        }
    }

    // 3. One disappeared and one appeared under the same file and name.
    // (file, qualified name) -> (routines that went, routines that came).
    type Group<'k> = (Vec<&'k EntityKey>, Vec<&'k EntityKey>);
    let mut groups: BTreeMap<(String, String), Group<'_>> = BTreeMap::new();
    for earlier in &before_entities {
        if earlier.scope == Scope::Routine && !used_before.contains(earlier) {
            groups
                .entry((new_path(&earlier.path), earlier.qualified_name.clone()))
                .or_default()
                .0
                .push(earlier);
        }
    }
    for later in &after_entities {
        if later.scope == Scope::Routine && !used_after.contains(later) {
            groups
                .entry((later.path.clone(), later.qualified_name.clone()))
                .or_default()
                .1
                .push(later);
        }
    }
    for (gone, came) in groups.values() {
        if let ([earlier], [later]) = (gone.as_slice(), came.as_slice()) {
            matches.push((earlier, later, Trust::Signature));
        }
    }

    // Before values of every paired entity, by metric name. Only the same
    // producer and metric definition are comparable.
    let mut pairing = Pairing::default();
    for (earlier, later, trust) in matches {
        pairing.paired.insert(later.clone(), trust);
        for was in before
            .facts
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
