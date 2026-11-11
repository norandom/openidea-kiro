//! Scattered definitions and scattered state (code-locality-data 1, 2).
//! Expected findings were decided by hand from the facts.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::Severity;
use kiro_core::code::Tag;
use kiro_core::code::locality::{
    Binding, Comparison, LocalityFacts, definition_findings, state_findings,
};

fn binding(path: &str, name: &str, initializer: &str) -> Binding {
    Binding {
        path: path.to_string(),
        line: 3,
        name: name.to_string(),
        initializer: initializer.to_string(),
    }
}

fn compare(path: &str, line: u32, subject: &str, literal: &str) -> Comparison {
    Comparison {
        path: path.to_string(),
        line,
        subject: subject.to_string(),
        literal: literal.to_string(),
    }
}

fn facts(bindings: Vec<Binding>, comparisons: Vec<Comparison>) -> LocalityFacts {
    LocalityFacts {
        bindings,
        comparisons,
        files: 0,
    }
}

fn files(paths: &[&str], name: &str, initializer: &str) -> Vec<Binding> {
    paths
        .iter()
        .map(|path| binding(path, name, initializer))
        .collect()
}

#[test]
fn the_same_definition_in_four_files_is_scattered_and_in_three_it_is_not() {
    let three = facts(files(&["a.py", "b.py", "c.py"], "TIMEOUT", "30"), vec![]);
    assert!(definition_findings(&three, &|_| true, 3).is_empty());

    let four = facts(
        files(&["a.py", "b.py", "c.py", "d.py"], "TIMEOUT", "30"),
        vec![],
    );
    let found = definition_findings(&four, &|_| true, 3);
    assert_eq!(found.len(), 4, "one finding per file");
    let first = &found[0];
    assert_eq!(first.finding.code, "CODE_SCATTERED_DEFINITION");
    assert_eq!(first.finding.severity, Severity::Warning);
    assert_eq!(first.finding.file, "a.py");
    assert_eq!(first.finding.line, Some(3));
    assert_eq!(
        first.finding.message,
        "`TIMEOUT = 30` is defined in 4 files; also in b.py, c.py, d.py"
    );
    assert_eq!(first.tag, Tag::Collect);
    assert_eq!(
        first.hint,
        "define `TIMEOUT` once in the module that owns it and import it in the other 3 files"
    );

    // Only the selected file is reported.
    let selected = definition_findings(&four, &|path| path == "c.py", 3);
    assert_eq!(selected.len(), 1);
    assert_eq!(
        selected[0].finding.message,
        "`TIMEOUT = 30` is defined in 4 files; also in a.py, b.py, d.py"
    );
}

#[test]
fn name_and_value_are_keyed_together() {
    // The same name with four different values is no repetition.
    let different = facts(
        vec![
            binding("a.py", "LIMIT", "1"),
            binding("b.py", "LIMIT", "2"),
            binding("c.py", "LIMIT", "3"),
            binding("d.py", "LIMIT", "4"),
        ],
        vec![],
    );
    assert!(definition_findings(&different, &|_| true, 3).is_empty());

    // Two definitions in one file count once.
    let twice = facts(
        vec![
            binding("a.py", "X", "1"),
            binding("a.py", "X", "1"),
            binding("b.py", "X", "1"),
            binding("c.py", "X", "1"),
        ],
        vec![],
    );
    assert!(definition_findings(&twice, &|_| true, 3).is_empty());
}

#[test]
fn the_per_module_idiom_is_ignored_by_name() {
    let mut bindings = Vec::new();
    for name in ["log", "logger", "pytestmark", "__all__"] {
        bindings.extend(files(&["a.py", "b.py", "c.py", "d.py", "e.py"], name, "x"));
    }
    assert!(definition_findings(&facts(bindings, vec![]), &|_| true, 3).is_empty());
}

#[test]
fn more_than_five_files_are_summarized() {
    let found = definition_findings(
        &facts(files(&["a", "b", "c", "d", "e", "f"], "K", "\"v\""), vec![]),
        &|path| path == "a",
        3,
    );
    assert_eq!(
        found[0].finding.message,
        "`K = \"v\"` is defined in 6 files; also in b, c, d and 2 more"
    );
}

#[test]
fn one_name_with_two_scattered_values_is_reported_and_never_merged() {
    let mut bindings = files(&["a", "b", "c", "d"], "MODE", "\"fast\"");
    bindings.extend(files(&["e", "f", "g", "h"], "MODE", "\"slow\""));
    let found = definition_findings(&facts(bindings, vec![]), &|_| true, 3);
    let scattered = found
        .iter()
        .filter(|f| f.finding.code == "CODE_SCATTERED_DEFINITION")
        .count();
    let ambiguous: Vec<_> = found
        .iter()
        .filter(|f| f.finding.code == "CODE_DEFINITION_AMBIGUOUS")
        .collect();
    assert_eq!(
        scattered, 8,
        "each value group is reported on its own files"
    );
    assert_eq!(ambiguous.len(), 8);
    assert_eq!(ambiguous[0].finding.severity, Severity::Info);
    assert_eq!(ambiguous[0].tag, Tag::Name);
    assert!(
        ambiguous[0]
            .finding
            .message
            .contains("`MODE` is defined with 2 different values")
    );
}

#[test]
fn a_subject_compared_with_two_literals_across_four_files_is_scattered_state() {
    let comparisons = vec![
        compare("a.rs", 10, "order.status", "\"paid\""),
        compare("a.rs", 4, "order.status", "\"open\""),
        compare("b.rs", 7, "order.status", "\"paid\""),
        compare("c.rs", 2, "order.status", "\"cancelled\""),
        compare("d.rs", 9, "order.status", "\"open\""),
    ];
    let found = state_findings(&facts(vec![], comparisons), &|_| true, 3, 2);
    assert_eq!(found.len(), 4, "once per file, not once per comparison");
    let first = &found[0];
    assert_eq!(first.finding.code, "CODE_SCATTERED_STATE");
    assert_eq!(first.finding.severity, Severity::Info);
    assert_eq!(first.finding.file, "a.rs");
    assert_eq!(
        first.finding.line,
        Some(4),
        "the first comparison in the file"
    );
    assert_eq!(
        first.finding.message,
        "`order.status` is compared with 3 literal values (\"cancelled\", \"open\", \"paid\") in 4 files; also in b.rs, c.rs, d.rs"
    );
    assert_eq!(first.tag, Tag::Collect);
    assert!(
        first
            .hint
            .starts_with("declare the values of `order.status` once")
    );
}

#[test]
fn state_needs_enough_values_and_enough_files() {
    // Four files, one literal: a flag, not a state.
    let one_value: Vec<Comparison> = ["a", "b", "c", "d"]
        .iter()
        .map(|path| compare(path, 1, "kind", "\"x\""))
        .collect();
    assert!(state_findings(&facts(vec![], one_value), &|_| true, 3, 2).is_empty());

    // Many literals, three files: within the allowance.
    let three_files = vec![
        compare("a", 1, "kind", "\"x\""),
        compare("b", 1, "kind", "\"y\""),
        compare("c", 1, "kind", "\"z\""),
    ];
    assert!(state_findings(&facts(vec![], three_files), &|_| true, 3, 2).is_empty());

    // All comparisons in one file: the state has one owner already.
    let one_file: Vec<Comparison> = (0..9)
        .map(|index| compare("only.rs", index, "kind", &format!("\"v{index}\"")))
        .collect();
    assert!(state_findings(&facts(vec![], one_file), &|_| true, 3, 2).is_empty());
}

#[test]
fn subjects_are_kept_apart_by_spelling_and_many_values_are_summarized() {
    let mut comparisons = Vec::new();
    for (index, path) in ["a", "b", "c", "d"].iter().enumerate() {
        comparisons.push(compare(path, 1, "mode", &format!("\"m{index}\"")));
        comparisons.push(compare(path, 2, "mode", "\"shared\""));
        // A second subject in two files only.
        if index < 2 {
            comparisons.push(compare(path, 3, "other", &format!("\"o{index}\"")));
        }
    }
    let found = state_findings(&facts(vec![], comparisons), &|path| path == "a", 3, 2);
    assert_eq!(found.len(), 1, "`other` stands in two files only");
    assert_eq!(
        found[0].finding.message,
        "`mode` is compared with 5 literal values (\"m0\", \"m1\", \"m2\", \"m3\" and 1 more) in 4 files; also in b, c, d"
    );
}
