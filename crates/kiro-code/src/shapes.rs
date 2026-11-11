//! Token shapes of routines and their pairwise similarity
//! (clone-constraints 1.2 and 1.5).
//!
//! A shape is the token sequence of a routine with every identifier
//! replaced by one placeholder and every string or number literal by
//! another; keywords, operators and punctuation stay as written. Comments
//! and routines nested inside are left out. Two routines that differ only
//! in names and constants have the same shape.
//!
//! The similarity of two shapes is `2 * M / (len(a) + len(b))`, where `M`
//! is the matched length that the `difflib-fast` library computes (the
//! Ratcliff-Obershelp matching of Python's `difflib`, without its junk
//! heuristic). No scorer is written here. The ratio depends on the order
//! of its arguments, so every pair is scored in one canonical order: the
//! shorter shape first, and on equal length the smaller one.
//!
//! Before a pair is scored, two exact ceilings are checked. The matched
//! length can exceed neither the shorter shape nor the number of tokens
//! the two shapes have in common, counted with multiplicity. A pair whose
//! ceiling is below the threshold cannot reach it and is refused; no pair
//! at or above the threshold is ever refused.

use std::collections::BTreeMap;

use kiro_core::code::families::{Edge, FamilyCoverage, FamilyFacts, Routine};

use crate::measure::{
    Profile, SgNode, document_order, has, is_routine, qualified, statement_count,
};

/// Most pairs scored in one run. Beyond it the comparison stops and the
/// coverage says that it is incomplete.
pub const MAX_SCORED_PAIRS: u64 = 2_000_000;

const IDENTIFIER: &str = "$I";
const LITERAL: &str = "$L";
/// First character of the token alphabet.
const FIRST_CODE: u32 = 0x21;

fn is_literal(kind: &str) -> bool {
    (kind.ends_with("_literal") && kind != "boolean_literal")
        || matches!(
            kind,
            "string"
                | "concatenated_string"
                | "template_string"
                | "regex"
                | "integer"
                | "float"
                | "number"
        )
}

fn is_identifier(kind: &str) -> bool {
    kind == "identifier"
        || kind.ends_with("_identifier")
        || kind.starts_with("shorthand_property_identifier")
}

/// The token a node stands for, or `None` when its children do.
fn token_of(node: &SgNode<'_>) -> Option<String> {
    let kind = node.kind();
    if is_literal(&kind) {
        return Some(LITERAL.to_string());
    }
    if node.children().next().is_some() {
        return None;
    }
    if is_identifier(&kind) {
        return Some(IDENTIFIER.to_string());
    }
    let text = node.text();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// The normalized tokens of one routine in document order.
#[must_use]
pub(crate) fn shape_tokens(
    function: &SgNode<'_>,
    profile: &Profile,
    comments: &[&str],
) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack: Vec<SgNode<'_>> = function.children().collect();
    stack.reverse();
    while let Some(node) = stack.pop() {
        if is_routine(&node, profile) || has(comments, &node.kind()) {
            continue;
        }
        if let Some(token) = token_of(&node) {
            out.push(token);
        } else {
            let mut children: Vec<SgNode<'_>> = node.children().collect();
            children.reverse();
            stack.extend(children);
        }
    }
    out
}

/// One character per distinct token, so that the ratio library, which
/// compares characters, compares tokens.
#[derive(Debug, Default)]
struct Alphabet {
    characters: BTreeMap<String, char>,
    next: u32,
}

impl Alphabet {
    fn character(&mut self, token: &str) -> char {
        if let Some(known) = self.characters.get(token) {
            return *known;
        }
        // Printable ASCII first: the ratio library tunes itself on ASCII
        // frequencies. The surrogate range is skipped.
        let mut code = FIRST_CODE.saturating_add(self.next);
        while char::from_u32(code).is_none() {
            code = code.saturating_add(1);
        }
        self.next = code.saturating_sub(FIRST_CODE).saturating_add(1);
        let character = char::from_u32(code).unwrap_or('\u{E000}');
        self.characters.insert(token.to_string(), character);
        character
    }

    fn encode(&mut self, tokens: &[String]) -> String {
        tokens.iter().map(|token| self.character(token)).collect()
    }
}

/// A routine's shape, ready to be compared.
#[derive(Debug, Clone)]
struct Shape {
    /// Index into the routine list.
    routine: usize,
    text: String,
    length: usize,
    /// Token counts, sorted by token.
    counts: Vec<(char, usize)>,
}

fn counts_of(text: &str) -> Vec<(char, usize)> {
    let mut counts: BTreeMap<char, usize> = BTreeMap::new();
    for character in text.chars() {
        *counts.entry(character).or_default() += 1;
    }
    counts.into_iter().collect()
}

/// Tokens two shapes have in common, counted with multiplicity. Both
/// lists are sorted by token, so one pass over each is enough.
fn common_tokens(a: &[(char, usize)], b: &[(char, usize)]) -> usize {
    let mut common = 0_usize;
    let mut rest = b.iter().peekable();
    for (character, count) in a {
        while rest.next_if(|(other, _)| other < character).is_some() {}
        if let Some((_, found)) = rest.next_if(|(other, _)| other == character) {
            common = common.saturating_add(*count.min(found));
        }
    }
    common
}

fn ratio_of(matched: usize, total: usize) -> f64 {
    if total == 0 {
        return 1.0;
    }
    #[allow(clippy::cast_precision_loss)]
    let ratio = (2 * matched) as f64 / total as f64;
    ratio
}

/// The highest ratio two shapes of these lengths can have.
#[must_use]
pub fn length_ceiling(a: usize, b: usize) -> f64 {
    ratio_of(a.min(b), a.saturating_add(b))
}

/// The highest ratio two shapes with these token counts can have.
#[must_use]
pub fn multiset_ceiling(a: &str, b: &str) -> f64 {
    let total = a.chars().count().saturating_add(b.chars().count());
    ratio_of(common_tokens(&counts_of(a), &counts_of(b)), total)
}

/// The similarity of two shapes in the canonical argument order.
#[must_use]
pub fn similarity(a: &str, b: &str) -> f64 {
    if a == b {
        return 1.0;
    }
    let (la, lb) = (a.chars().count(), b.chars().count());
    if (la, a) <= (lb, b) {
        difflib_fast::ratio(a, b)
    } else {
        difflib_fast::ratio(b, a)
    }
}

/// Collects the routines of a project and scores them.
#[derive(Debug, Default)]
pub(crate) struct ShapeCollector {
    alphabet: Alphabet,
    routines: Vec<Routine>,
    /// Shapes by language: only routines of one language are compared.
    shapes: BTreeMap<String, Vec<Shape>>,
    seen: usize,
}

/// TypeScript with and without JSX is one language.
fn language_group(profile: &Profile) -> &'static str {
    if profile.name == "tsx" {
        "typescript"
    } else {
        profile.name
    }
}

fn line_of(index: usize) -> u32 {
    u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX)
}

impl ShapeCollector {
    /// Adds the routines of one parsed file that have enough statements.
    pub(crate) fn add_file(
        &mut self,
        root: &SgNode<'_>,
        profile: &Profile,
        comments: &[&str],
        path: &str,
        min_statements: u32,
    ) {
        for node in document_order(root) {
            if !is_routine(&node, profile) {
                continue;
            }
            self.seen = self.seen.saturating_add(1);
            let statements = u32::try_from(statement_count(&node, profile)).unwrap_or(u32::MAX);
            if statements < min_statements {
                continue;
            }
            let text = self
                .alphabet
                .encode(&shape_tokens(&node, profile, comments));
            let length = text.chars().count();
            let language = language_group(profile);
            self.shapes
                .entry(language.to_string())
                .or_default()
                .push(Shape {
                    routine: self.routines.len(),
                    counts: counts_of(&text),
                    text,
                    length,
                });
            self.routines.push(Routine {
                path: path.to_string(),
                line: line_of(node.start_pos().line()),
                end_line: line_of(node.end_pos().line()),
                name: qualified(&node, profile),
                language: language.to_string(),
                statements,
                tokens: u32::try_from(length).unwrap_or(u32::MAX),
            });
        }
    }

    /// Scores every pair that can reach the threshold.
    pub(crate) fn finish(self, threshold: f64) -> FamilyFacts {
        self.finish_within(threshold, MAX_SCORED_PAIRS)
    }

    /// As `finish`, with the most pairs that may be scored.
    fn finish_within(mut self, threshold: f64, budget: u64) -> FamilyFacts {
        let mut tally = Tally {
            threshold,
            budget,
            scored: 0,
            pruned: 0,
            complete: true,
            edges: Vec::new(),
        };
        for shapes in self.shapes.values_mut() {
            shapes.sort_by(|a, b| {
                (a.length, &a.text, a.routine).cmp(&(b.length, &b.text, b.routine))
            });
            tally.score_language(shapes);
        }
        tally.edges.sort_by_key(|edge| (edge.a, edge.b));
        FamilyFacts {
            coverage: FamilyCoverage {
                routines: self.seen,
                eligible: self.routines.len(),
                scored_pairs: tally.scored,
                pruned_pairs: tally.pruned,
                complete: tally.complete,
            },
            routines: self.routines,
            edges: tally.edges,
        }
    }
}

struct Tally {
    threshold: f64,
    /// Most pairs that may be scored.
    budget: u64,
    scored: u64,
    pruned: u64,
    complete: bool,
    edges: Vec<Edge>,
}

impl Tally {
    /// Scores the pairs of one language. `shapes` is sorted by length, so
    /// the first partner that is too long ends the search for a shape.
    fn score_language(&mut self, shapes: &[Shape]) {
        for (position, shorter) in shapes.iter().enumerate() {
            let partners = shapes.get(position.saturating_add(1)..).unwrap_or(&[]);
            let reachable = partners
                .iter()
                .take_while(|longer| {
                    length_ceiling(shorter.length, longer.length) >= self.threshold
                })
                .count();
            let beyond = partners.len().saturating_sub(reachable);
            self.pruned = self.pruned.saturating_add(beyond as u64);
            for longer in partners.iter().take(reachable) {
                if !self.score_pair(shorter, longer) {
                    return;
                }
            }
        }
    }

    /// Scores one pair. Returns `false` when the budget is spent.
    fn score_pair(&mut self, shorter: &Shape, longer: &Shape) -> bool {
        let total = shorter.length.saturating_add(longer.length);
        let common = common_tokens(&shorter.counts, &longer.counts);
        if ratio_of(common, total) < self.threshold {
            self.pruned = self.pruned.saturating_add(1);
            return true;
        }
        if self.scored >= self.budget {
            self.complete = false;
            return false;
        }
        self.scored = self.scored.saturating_add(1);
        let ratio = similarity(&shorter.text, &longer.text);
        if ratio >= self.threshold {
            self.edges.push(Edge {
                a: shorter.routine.min(longer.routine),
                b: shorter.routine.max(longer.routine),
                ratio,
            });
        }
        true
    }
}

/// The shape of every routine in one source text with at least
/// `min_statements` statements, as readable tokens. For tests and for
/// looking at what the comparison sees.
#[must_use]
pub fn shapes_of(path: &str, text: &str, min_statements: u32) -> Vec<(String, Vec<String>)> {
    let (Some(profile), Some(comments)) = (
        crate::measure::profile_for(path),
        crate::measure::comment_kinds(path),
    ) else {
        return Vec::new();
    };
    let Some(lang) = crate::measure::language_of(path) else {
        return Vec::new();
    };
    let normalized = text.replace("\r\n", "\n");
    let Ok(tree) = ast_grep_core::tree_sitter::StrDoc::try_new(&normalized, lang)
        .map(ast_grep_core::AstGrep::doc)
    else {
        return Vec::new();
    };
    document_order(&tree.root())
        .into_iter()
        .filter(|node| is_routine(node, profile))
        .filter(|node| {
            u32::try_from(statement_count(node, profile)).unwrap_or(u32::MAX) >= min_statements
        })
        .map(|node| {
            (
                qualified(&node, profile),
                shape_tokens(&node, profile, comments),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use super::*;

    fn collector(texts: &[&str]) -> ShapeCollector {
        let mut collector = ShapeCollector::default();
        for (index, text) in texts.iter().enumerate() {
            collector
                .shapes
                .entry("rust".to_string())
                .or_default()
                .push(Shape {
                    routine: index,
                    text: (*text).to_string(),
                    length: text.chars().count(),
                    counts: counts_of(text),
                });
            collector.routines.push(Routine {
                path: format!("src/{index}.rs"),
                line: 1,
                end_line: 9,
                name: format!("routine_{index}"),
                language: "rust".to_string(),
                statements: 6,
                tokens: u32::try_from(text.chars().count()).unwrap(),
            });
        }
        collector
    }

    #[test]
    fn pairs_are_scored_refused_or_left_when_the_budget_is_spent() {
        // Three alike (ratios 0.9 each, by hand: nine of ten match), one
        // of other tokens, one far too long.
        let texts = [
            "abcdefghij",
            "abcdefghix",
            "abcdefghiy",
            "qrstuvwxyz",
            "abcdefghijabcdefghij",
        ];
        let facts = collector(&texts).finish(0.9);
        let pairs: Vec<(usize, usize)> = facts.edges.iter().map(|edge| (edge.a, edge.b)).collect();
        assert_eq!(pairs, [(0, 1), (0, 2), (1, 2)]);
        assert!(
            facts
                .edges
                .iter()
                .all(|edge| (edge.ratio - 0.9).abs() < 1e-9)
        );
        // Ten pairs: three scored, three refused on tokens (the fourth
        // text against the alike ones), four refused on length.
        assert_eq!(facts.coverage.scored_pairs, 3);
        assert_eq!(facts.coverage.pruned_pairs, 7);
        assert!(facts.coverage.complete);

        // With a budget of two pairs the third is left and that is said.
        let short = collector(&texts).finish_within(0.9, 2);
        assert_eq!(short.coverage.scored_pairs, 2);
        assert!(!short.coverage.complete);
        assert_eq!(short.edges.len(), 2);
    }
}
