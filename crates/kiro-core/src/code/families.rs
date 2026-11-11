//! Similar-routine families (clone-constraints 1.2 to 1.4).
//!
//! A family is a set of routines connected by similarity: each member is
//! at least `DEFAULT_THRESHOLD` similar to some other member. The
//! similarity is a ratio over token shapes (identifiers and literals
//! replaced by placeholders), computed by an adopted library in the native
//! crate. This module is pure: it receives the routines and the scored
//! pairs and decides which families exist and how they are reported.
//!
//! What it finds: a routine that was copied and renamed. What it does not
//! find: the same behaviour written in another shape.

use std::collections::BTreeMap;

use super::report::{CodeFinding, Tag};
use crate::finding::{Finding, Severity, codes};

/// Lowest similarity that connects two routines.
pub const DEFAULT_THRESHOLD: f64 = 0.9;
/// Fewest statements of a routine that is compared.
pub const DEFAULT_MIN_STATEMENTS: u32 = 6;
/// Routine names that are not compared: every type or test class has its
/// own, and their likeness is the convention, not a copy. A Python name
/// with two underscores on each side is never compared either.
pub const IGNORED_NAMES: &[&str] = &[
    "main",
    "new",
    "default",
    "fmt",
    "setUp",
    "tearDown",
    "setup",
    "teardown",
    "setup_method",
    "teardown_method",
    "setup_class",
    "teardown_class",
];
/// Most other members named in one finding.
const MAX_OTHERS: usize = 3;

/// A routine that takes part in the comparison.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Routine {
    pub path: String,
    /// 1-based first and last line.
    pub line: u32,
    pub end_line: u32,
    /// Nesting-qualified name.
    pub name: String,
    /// Only routines of one language are compared.
    pub language: String,
    pub statements: u32,
    /// Length of the token shape.
    pub tokens: u32,
}

/// Two routines and their similarity. `a` and `b` index `FamilyFacts::routines`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub a: usize,
    pub b: usize,
    pub ratio: f64,
}

/// How much of the project the comparison covered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FamilyCoverage {
    /// Routines seen, of any size.
    pub routines: usize,
    /// Routines with enough statements.
    pub eligible: usize,
    /// Pairs whose similarity was computed.
    pub scored_pairs: u64,
    /// Pairs refused before scoring because the threshold was out of reach.
    pub pruned_pairs: u64,
    /// `false` when the pair budget ran out: families may be missing.
    pub complete: bool,
}

/// The routines of a project and every pair at or above the threshold.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FamilyFacts {
    pub routines: Vec<Routine>,
    pub edges: Vec<Edge>,
    pub coverage: FamilyCoverage,
}

/// A family: its members in path and line order, and the lowest
/// similarity among the pairs that hold it together.
#[derive(Debug, Clone, PartialEq)]
pub struct Family {
    pub members: Vec<usize>,
    pub weakest: f64,
}

fn is_dunder(name: &str) -> bool {
    name.len() > 4 && name.starts_with("__") && name.ends_with("__")
}

fn last_segment(name: &str) -> &str {
    name.rsplit(['.', ':']).next().unwrap_or(name)
}

fn find(parent: &mut [usize], start: usize) -> usize {
    let mut root = start;
    while let Some(next) = parent.get(root).copied().filter(|next| *next != root) {
        root = next;
    }
    // Path compression keeps later lookups short.
    let mut node = start;
    while let Some(next) = parent.get(node).copied().filter(|next| *next != root) {
        if let Some(slot) = parent.get_mut(node) {
            *slot = root;
        }
        node = next;
    }
    root
}

fn is_alive(alive: &[bool], index: usize) -> bool {
    alive.get(index).copied().unwrap_or(false)
}

fn retained<'a>(facts: &'a FamilyFacts, alive: &'a [bool]) -> impl Iterator<Item = &'a Edge> {
    facts
        .edges
        .iter()
        .filter(|edge| edge.a != edge.b && is_alive(alive, edge.a) && is_alive(alive, edge.b))
}

/// The connected groups of at least two routines over the retained edges.
fn components(facts: &FamilyFacts, alive: &[bool]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..facts.routines.len()).collect();
    for edge in retained(facts, alive) {
        let (a, b) = (find(&mut parent, edge.a), find(&mut parent, edge.b));
        if let Some(slot) = parent.get_mut(a.max(b)) {
            *slot = a.min(b);
        }
    }
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..facts.routines.len() {
        if is_alive(alive, index) {
            groups
                .entry(find(&mut parent, index))
                .or_default()
                .push(index);
        }
    }
    groups
        .into_values()
        .filter(|members| members.len() > 1)
        .collect()
}

/// Whether `inner` stands inside `outer` in the same file.
fn inside(inner: &Routine, outer: &Routine) -> bool {
    inner.path == outer.path
        && outer.line <= inner.line
        && inner.end_line <= outer.end_line
        && (inner.line, inner.end_line) != (outer.line, outer.end_line)
}

/// Members that stand inside another member of their own group.
fn nested_members(facts: &FamilyFacts, groups: &[Vec<usize>]) -> Vec<usize> {
    let mut nested = Vec::new();
    for group in groups {
        let members: Vec<(usize, &Routine)> = group
            .iter()
            .filter_map(|index| facts.routines.get(*index).map(|routine| (*index, routine)))
            .collect();
        for (index, routine) in &members {
            if members.iter().any(|(_, other)| inside(routine, other)) {
                nested.push(*index);
            }
        }
    }
    nested
}

fn weakest(facts: &FamilyFacts, alive: &[bool], members: &[usize]) -> f64 {
    retained(facts, alive)
        .filter(|edge| members.contains(&edge.a) && members.contains(&edge.b))
        .map(|edge| edge.ratio)
        .fold(1.0, f64::min)
}

/// The families of a project.
///
/// A routine with an ignored name takes no part, and neither does a
/// routine nested in another member of its family. Reachability is
/// computed again after each removal, so a removed routine never holds
/// two others together.
#[must_use]
pub fn families(facts: &FamilyFacts, ignored: &[&str]) -> Vec<Family> {
    let mut alive: Vec<bool> = facts
        .routines
        .iter()
        .map(|routine| {
            let name = last_segment(&routine.name);
            !is_dunder(name) && !ignored.contains(&name)
        })
        .collect();
    let groups = loop {
        let groups = components(facts, &alive);
        let nested = nested_members(facts, &groups);
        if nested.is_empty() {
            break groups;
        }
        for index in nested {
            if let Some(slot) = alive.get_mut(index) {
                *slot = false;
            }
        }
    };
    let mut out: Vec<Family> = groups
        .into_iter()
        .map(|mut members| {
            members.sort_by_key(|index| facts.routines.get(*index));
            let weakest = weakest(facts, &alive, &members);
            Family { members, weakest }
        })
        .collect();
    out.sort_by_key(|family| {
        family
            .members
            .first()
            .and_then(|index| facts.routines.get(*index))
    });
    out
}

fn place(routine: &Routine) -> String {
    format!("`{}` at {}:{}", routine.name, routine.path, routine.line)
}

fn others_text(others: &[&Routine]) -> String {
    let named: Vec<String> = others.iter().take(MAX_OTHERS).map(|r| place(r)).collect();
    let more = others.len().saturating_sub(named.len());
    if more > 0 {
        format!("{} and {more} more", named.join(", "))
    } else {
        named.join(", ")
    }
}

fn family_finding(members: &[&Routine], anchor: &Routine, weakest: f64) -> CodeFinding {
    let others: Vec<&Routine> = members
        .iter()
        .copied()
        .filter(|member| *member != anchor)
        .collect();
    let size = members.len();
    let mut finding = Finding::new(
        codes::CODE_SIMILAR_FAMILY,
        anchor.path.clone(),
        format!(
            "routine `{}` is one of {size} similar routines (weakest similarity {weakest:.2}); also {}",
            anchor.name,
            others_text(&others)
        ),
    )
    .severity(Severity::Warning);
    finding.line = usize::try_from(anchor.line).ok();
    if members.iter().all(|member| member.path == anchor.path) {
        let hint =
            format!("merge the {size} routines into one and pass what differs as a parameter");
        CodeFinding::across_files(finding, Tag::Shrink, hint)
    } else {
        let hint = format!(
            "keep one routine in the module that owns it and call it from the other {} places",
            size.saturating_sub(1)
        );
        CodeFinding::across_files(finding, Tag::Delete, hint)
    }
}

/// One finding per family that holds a selected routine, placed at the
/// first selected member.
#[must_use]
pub fn family_findings(
    facts: &FamilyFacts,
    families: &[Family],
    selected: &dyn Fn(&str) -> bool,
) -> Vec<CodeFinding> {
    let mut out = Vec::new();
    for family in families {
        let members: Vec<&Routine> = family
            .members
            .iter()
            .filter_map(|index| facts.routines.get(*index))
            .collect();
        if let Some(anchor) = members
            .iter()
            .copied()
            .find(|member| selected(&member.path))
        {
            out.push(family_finding(&members, anchor, family.weakest));
        }
    }
    out
}
