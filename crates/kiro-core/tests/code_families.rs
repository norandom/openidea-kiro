//! Similar-routine families (clone-constraints 1.3, 1.4, 3.2). The
//! expected families were decided by hand from the routines and edges.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::Severity;
use kiro_core::code::Tag;
use kiro_core::code::families::{
    Edge, Family, FamilyFacts, IGNORED_NAMES, Routine, families, family_findings,
};

fn routine(path: &str, line: u32, end_line: u32, name: &str) -> Routine {
    Routine {
        path: path.to_string(),
        line,
        end_line,
        name: name.to_string(),
        language: "rust".to_string(),
        statements: 8,
        tokens: 60,
    }
}

fn facts(routines: Vec<Routine>, edges: &[(usize, usize, f64)]) -> FamilyFacts {
    FamilyFacts {
        routines,
        edges: edges
            .iter()
            .map(|(a, b, ratio)| Edge {
                a: *a,
                b: *b,
                ratio: *ratio,
            })
            .collect(),
        ..FamilyFacts::default()
    }
}

fn members(found: &[Family]) -> Vec<Vec<usize>> {
    found.iter().map(|family| family.members.clone()).collect()
}

#[test]
fn a_chain_of_similar_routines_is_one_family_held_by_its_weakest_pair() {
    // a is like b and b is like c; a and c were never above the threshold.
    let facts = facts(
        vec![
            routine("src/c.rs", 5, 20, "load_c"),
            routine("src/a.rs", 9, 24, "load_a"),
            routine("src/b.rs", 1, 16, "load_b"),
            routine("src/z.rs", 1, 16, "other"),
        ],
        &[(1, 2, 0.95), (0, 2, 0.92)],
    );
    let found = families(&facts, IGNORED_NAMES);
    assert_eq!(members(&found), [[1, 2, 0]], "in path order");
    assert!((found[0].weakest - 0.92).abs() < 1e-12);

    let all = family_findings(&facts, &found, &|_| true);
    assert_eq!(all.len(), 1, "one finding per family");
    let finding = &all[0];
    assert_eq!(finding.finding.code, "CODE_SIMILAR_FAMILY");
    assert_eq!(finding.finding.severity, Severity::Warning);
    assert_eq!(finding.finding.file, "src/a.rs");
    assert_eq!(finding.finding.line, Some(9));
    assert_eq!(
        finding.finding.message,
        "routine `load_a` is one of 3 similar routines (weakest similarity 0.92); also `load_b` at src/b.rs:1, `load_c` at src/c.rs:5"
    );
    assert_eq!(finding.tag, Tag::Delete);
    assert_eq!(
        finding.hint,
        "keep one routine in the module that owns it and call it from the other 2 places"
    );
}

#[test]
fn the_finding_stands_at_the_first_selected_member_or_nowhere() {
    let facts = facts(
        vec![
            routine("src/a.rs", 9, 24, "load_a"),
            routine("src/b.rs", 1, 16, "load_b"),
        ],
        &[(0, 1, 0.95)],
    );
    let found = families(&facts, IGNORED_NAMES);
    let at_b = family_findings(&facts, &found, &|path| path == "src/b.rs");
    assert_eq!(at_b.len(), 1);
    assert_eq!(at_b[0].finding.file, "src/b.rs");
    assert_eq!(
        at_b[0].finding.message,
        "routine `load_b` is one of 2 similar routines (weakest similarity 0.95); also `load_a` at src/a.rs:9"
    );
    assert!(family_findings(&facts, &found, &|path| path == "src/z.rs").is_empty());
}

#[test]
fn a_family_in_one_file_is_to_be_shrunk_and_a_large_one_is_summarized() {
    let routines: Vec<Routine> = (0..6)
        .map(|index| {
            routine(
                "src/a.rs",
                10 + index * 20,
                25 + index * 20,
                &format!("step_{index}"),
            )
        })
        .collect();
    // A star around the first routine.
    let edges: Vec<(usize, usize, f64)> = (1..6).map(|index| (0, index, 0.96)).collect();
    let facts = facts(routines, &edges);
    let found = families(&facts, IGNORED_NAMES);
    assert_eq!(members(&found), [[0, 1, 2, 3, 4, 5]]);
    let finding = &family_findings(&facts, &found, &|_| true)[0];
    assert_eq!(
        finding.finding.message,
        "routine `step_0` is one of 6 similar routines (weakest similarity 0.96); also `step_1` at src/a.rs:30, `step_2` at src/a.rs:50, `step_3` at src/a.rs:70 and 2 more"
    );
    assert_eq!(finding.tag, Tag::Shrink);
    assert_eq!(
        finding.hint,
        "merge the 6 routines into one and pass what differs as a parameter"
    );
}

#[test]
fn an_ignored_name_takes_no_part_and_cannot_bridge() {
    // `a` and `c` are each like a constructor, not like each other.
    let facts = facts(
        vec![
            routine("src/a.rs", 1, 16, "build_a"),
            routine("src/b.rs", 1, 16, "Thing::new"),
            routine("src/c.rs", 1, 16, "build_c"),
            routine("tools/d.py", 1, 16, "Thing.__init__"),
            routine("tools/e.py", 1, 16, "setup"),
        ],
        &[(0, 1, 0.95), (1, 2, 0.95), (3, 4, 0.99)],
    );
    assert!(families(&facts, IGNORED_NAMES).is_empty());
    // With nothing ignored the bridge holds.
    assert_eq!(members(&families(&facts, &[])), [vec![0, 1, 2], vec![3, 4]]);
}

#[test]
fn a_member_nested_in_another_member_is_removed() {
    // The inner routine stands inside the outer one; both are like a third.
    let facts = facts(
        vec![
            routine("src/a.rs", 1, 30, "outer"),
            routine("src/a.rs", 10, 20, "outer::inner"),
            routine("src/b.rs", 1, 30, "third"),
        ],
        &[(0, 2, 0.95), (1, 2, 0.91)],
    );
    let found = families(&facts, IGNORED_NAMES);
    assert_eq!(members(&found), [[0, 2]]);
    assert!(
        (found[0].weakest - 0.95).abs() < 1e-12,
        "the removed member's pair does not count"
    );
}

#[test]
fn a_removed_nested_member_cannot_hold_two_others_together() {
    // outer ~ inner and inner ~ third; outer and third are not alike.
    let facts = facts(
        vec![
            routine("src/a.rs", 1, 30, "outer"),
            routine("src/a.rs", 10, 20, "outer::inner"),
            routine("src/b.rs", 1, 30, "third"),
        ],
        &[(0, 1, 0.95), (1, 2, 0.93)],
    );
    assert!(families(&facts, IGNORED_NAMES).is_empty());

    // Two routines side by side in one file are not nested.
    let side_by_side = self::facts(
        vec![
            routine("src/a.rs", 1, 30, "first"),
            routine("src/a.rs", 31, 60, "second"),
        ],
        &[(0, 1, 0.95)],
    );
    assert_eq!(members(&families(&side_by_side, IGNORED_NAMES)), [[0, 1]]);
}

#[test]
fn separate_groups_are_separate_families_in_a_stable_order() {
    let facts = facts(
        vec![
            routine("src/d.rs", 1, 16, "d"),
            routine("src/c.rs", 1, 16, "c"),
            routine("src/b.rs", 1, 16, "b"),
            routine("src/a.rs", 1, 16, "a"),
        ],
        &[(0, 1, 0.91), (2, 3, 0.99)],
    );
    let found = families(&facts, IGNORED_NAMES);
    assert_eq!(members(&found), [[3, 2], [1, 0]]);
    // The same facts with the edges in another order give the same result.
    let mut reversed = facts.clone();
    reversed.edges.reverse();
    assert_eq!(families(&reversed, IGNORED_NAMES), found);
}
