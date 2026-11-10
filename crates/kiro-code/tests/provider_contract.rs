//! Metric semantics of the adopted parse layer (code-constraints 12.1, 3.1, 4.1, 4.2).
//!
//! The values asserted here were counted by hand from the fixture sources
//! under `fixtures/metrics/`, following the definitions in `measure.rs`.
//! They are the oracle; `expected.json` is the full regression record.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_code::measure_into;
use kiro_core::code::{CodeFacts, Completeness, Scope};

const FIXTURES: &[(&str, &str)] = &[
    ("sample.rs", include_str!("../fixtures/metrics/sample.rs")),
    (
        "constructs.rs",
        include_str!("../fixtures/metrics/constructs.rs"),
    ),
    ("sample.py", include_str!("../fixtures/metrics/sample.py")),
    ("sample.ts", include_str!("../fixtures/metrics/sample.ts")),
    (
        "constructs.py",
        include_str!("../fixtures/metrics/constructs.py"),
    ),
    (
        "constructs.ts",
        include_str!("../fixtures/metrics/constructs.ts"),
    ),
];

#[test]
fn python_constructs_match_the_hand_counts() {
    let facts = facts();
    let path = "constructs.py";
    // Nested routines are their own entities: `retry` holds one statement.
    assert_eq!(routine(&facts, path, "retry"), [1.0, 1.0, 0.0, 2.0, 1.0]);
    // The decorator line of the inner routine is code of `decorator`, and
    // the decorated definition is one item of its block.
    assert_eq!(
        routine(&facts, path, "retry.decorator"),
        [1.0, 1.0, 0.0, 3.0, 2.0]
    );
    // *args and **kwargs are parameters; for, except and if are decisions;
    // for > try > if nests three deep.
    assert_eq!(
        routine(&facts, path, "retry.decorator.wrapper"),
        [2.0, 4.0, 3.0, 8.0, 6.0]
    );
    assert_eq!(
        routine(&facts, path, "Account.__init__"),
        [2.0, 1.0, 0.0, 3.0, 2.0]
    );
    // The `@property` line belongs to the class body, not to the routine.
    assert_eq!(
        routine(&facts, path, "Account.rich"),
        [0.0, 1.0, 0.0, 2.0, 1.0]
    );
    // An async routine is a routine; the bare `*` declares nothing; `or`
    // and the lambda's conditional expression are decisions of `pay`.
    assert_eq!(
        routine(&facts, path, "Account.pay"),
        [2.0, 3.0, 1.0, 5.0, 4.0]
    );

    let file = |metric: &str| value(&facts, path, Scope::File, path, metric).unwrap();
    assert_eq!(file("functions"), 6.0);
    assert_eq!(file("types"), 1.0);
    assert_eq!(file("max_cyclomatic"), 4.0);
    // 27 non-blank lines, no comment and no docstring.
    assert_eq!(file("code_lines"), 27.0);
    assert_eq!(
        value(&facts, path, Scope::Type, "Account", "methods"),
        Some(3.0)
    );
}

#[test]
fn typescript_constructs_match_the_hand_counts() {
    let facts = facts();
    let path = "constructs.ts";
    // An arrow function bound to a name is a routine. for..of, if, && and
    // the ternary in the inline callback are its decisions.
    assert_eq!(routine(&facts, path, "pick"), [2.0, 5.0, 2.0, 8.0, 4.0]);
    assert_eq!(
        routine(&facts, path, "Square.constructor"),
        [1.0, 1.0, 0.0, 3.0, 1.0]
    );
    // try opens a nesting level and catch is a decision.
    assert_eq!(
        routine(&facts, path, "Square.area"),
        [0.0, 2.0, 1.0, 7.0, 3.0]
    );
    // An arrow function held by a class field is a routine with an
    // expression body: no statement.
    assert_eq!(
        routine(&facts, path, "Square.scale"),
        [1.0, 1.0, 0.0, 1.0, 0.0]
    );
    // A single parameter without a list.
    assert_eq!(routine(&facts, path, "twice"), [1.0, 1.0, 0.0, 1.0, 0.0]);
    // The inline callback of `map` is not a routine of its own.
    assert_eq!(
        facts
            .observations
            .iter()
            .filter(|o| o.entity.path == path
                && o.entity.scope == Scope::Routine
                && o.metric.name == "cyclomatic")
            .count(),
        5
    );

    let file = |metric: &str| value(&facts, path, Scope::File, path, metric).unwrap();
    assert_eq!(file("functions"), 5.0);
    // Interface, enum and class.
    assert_eq!(file("types"), 3.0);
    assert_eq!(file("max_cyclomatic"), 5.0);
    assert_eq!(file("code_lines"), 32.0);
    let member = |name: &str, metric: &str| value(&facts, path, Scope::Type, name, metric);
    assert_eq!(member("Shape", "methods"), Some(1.0));
    assert_eq!(member("Shape", "fields"), Some(1.0));
    assert_eq!(member("Mode", "methods"), Some(0.0));
    // constructor, area and the arrow field `scale` are methods; `name`
    // and `side` are fields.
    assert_eq!(member("Square", "methods"), Some(3.0));
    assert_eq!(member("Square", "fields"), Some(2.0));
}

fn facts() -> CodeFacts {
    let mut facts = CodeFacts::default();
    for (path, text) in FIXTURES {
        measure_into(path, text, &mut facts);
    }
    facts.sort();
    facts
}

/// The value of `metric` for the entity `name` of `scope` in `path`.
fn value(facts: &CodeFacts, path: &str, scope: Scope, name: &str, metric: &str) -> Option<f64> {
    facts
        .observations
        .iter()
        .find(|observation| {
            observation.entity.path == path
                && observation.entity.scope == scope
                && observation.entity.qualified_name == name
                && observation.metric.name == metric
        })
        .map(|observation| observation.value)
}

/// (parameters, cyclomatic, nesting, code lines, statements) of a routine.
fn routine(facts: &CodeFacts, path: &str, name: &str) -> [f64; 5] {
    [
        "parameters",
        "cyclomatic",
        "nesting",
        "code_lines",
        "statements",
    ]
    .map(|metric| {
        value(facts, path, Scope::Routine, name, metric)
            .unwrap_or_else(|| panic!("{path} {name} {metric} was not measured"))
    })
}

#[test]
fn rust_routines_match_the_hand_counts() {
    let facts = facts();
    assert_eq!(
        routine(&facts, "sample.rs", "Config::new"),
        [1.0, 1.0, 0.0, 3.0, 1.0]
    );
    // if, if, ||, for, if, &&, while, and two match arms beyond the first.
    assert_eq!(
        routine(&facts, "sample.rs", "Config::classify"),
        [2.0, 10.0, 3.0, 20.0, 9.0]
    );
    // The comment line and the blank line are not code.
    assert_eq!(
        routine(&facts, "sample.rs", "free"),
        [3.0, 1.0, 0.0, 4.0, 2.0]
    );
}

#[test]
fn rust_constructs_match_the_hand_counts() {
    let facts = facts();
    let path = "constructs.rs";
    // A default method of a trait is a routine; the bare signature is not.
    assert_eq!(
        routine(&facts, path, "outer::Shape::name"),
        [0.0, 1.0, 0.0, 3.0, 1.0]
    );
    assert!(
        value(
            &facts,
            path,
            Scope::Routine,
            "outer::Shape::area",
            "cyclomatic"
        )
        .is_none()
    );
    // A closure belongs to the routine around it: its `if` counts there.
    // Statements are the items of every block: the `let`, the call, and
    // the tail expression in each of the closure's two branch blocks.
    assert_eq!(
        routine(&facts, path, "outer::Square::scaled"),
        [1.0, 2.0, 1.0, 4.0, 4.0]
    );
    // A nested routine is its own entity and adds nothing to its parent.
    assert_eq!(
        routine(&facts, path, "outer::Square::area"),
        [0.0, 1.0, 0.0, 3.0, 1.0]
    );
    assert_eq!(
        routine(&facts, path, "outer::Square::area::square"),
        [1.0, 1.0, 0.0, 3.0, 1.0]
    );
    // `?`, `loop` and a macro are no decisions; the else-if chain stays on
    // one nesting level below the loop.
    assert_eq!(routine(&facts, path, "pick"), [2.0, 3.0, 2.0, 13.0, 7.0]);
}

#[test]
fn python_and_typescript_routines_match_the_hand_counts() {
    let facts = facts();
    // if, elif, or, for, if, and, while; the docstring is no code line.
    assert_eq!(
        routine(&facts, "sample.py", "classify"),
        [3.0, 8.0, 3.0, 10.0, 8.0]
    );
    // `self` is the receiver; the conditional expression is a decision.
    assert_eq!(
        routine(&facts, "sample.py", "Box.size"),
        [1.0, 2.0, 0.0, 2.0, 1.0]
    );
    // if, else-if, ||, for, if, &&, while, one case, one ternary.
    assert_eq!(
        routine(&facts, "sample.ts", "classify"),
        [2.0, 10.0, 3.0, 20.0, 11.0]
    );
    assert_eq!(
        routine(&facts, "sample.ts", "Box.size"),
        [1.0, 2.0, 0.0, 3.0, 1.0]
    );
}

#[test]
fn file_metrics_match_the_hand_counts() {
    let facts = facts();
    let file = |path: &str, metric: &str| value(&facts, path, Scope::File, path, metric).unwrap();
    // 36 lines, 4 blank, 2 comment lines.
    assert_eq!(file("sample.rs", "code_lines"), 30.0);
    assert_eq!(file("sample.rs", "functions"), 3.0);
    assert_eq!(file("sample.rs", "types"), 1.0);
    assert_eq!(file("sample.rs", "max_cyclomatic"), 10.0);
    // 44 non-blank lines, no comments; the alias is no type.
    assert_eq!(file("constructs.rs", "code_lines"), 44.0);
    assert_eq!(file("constructs.rs", "functions"), 5.0);
    assert_eq!(file("constructs.rs", "types"), 3.0);
    assert_eq!(file("constructs.rs", "max_cyclomatic"), 3.0);
    // 17 lines, 2 blank, 1 docstring.
    assert_eq!(file("sample.py", "code_lines"), 14.0);
    assert_eq!(file("sample.py", "functions"), 2.0);
    assert_eq!(file("sample.py", "types"), 1.0);
    // 26 lines, 1 blank.
    assert_eq!(file("sample.ts", "code_lines"), 25.0);
    assert_eq!(file("sample.ts", "functions"), 2.0);
    assert_eq!(file("sample.ts", "types"), 1.0);
}

#[test]
fn type_metrics_match_the_hand_counts() {
    let facts = facts();
    let member =
        |path: &str, name: &str, metric: &str| value(&facts, path, Scope::Type, name, metric);
    // Inherent impl methods in the same file belong to the type.
    assert_eq!(member("sample.rs", "Config", "methods"), Some(2.0));
    assert_eq!(member("sample.rs", "Config", "fields"), Some(2.0));
    // A trait counts its signatures and default methods.
    assert_eq!(
        member("constructs.rs", "outer::Shape", "methods"),
        Some(2.0)
    );
    // `impl Shape for Square` is not counted for Square; the nested routine is no method.
    assert_eq!(
        member("constructs.rs", "outer::Square", "methods"),
        Some(1.0)
    );
    assert_eq!(
        member("constructs.rs", "outer::Square", "fields"),
        Some(2.0)
    );
    assert_eq!(member("constructs.rs", "Mode", "methods"), Some(0.0));
    assert_eq!(member("sample.py", "Box", "methods"), Some(1.0));
    // No field profile for Python: the value is absent, not zero.
    assert_eq!(member("sample.py", "Box", "fields"), None);
    assert_eq!(member("sample.ts", "Box", "methods"), Some(1.0));
    assert_eq!(member("sample.ts", "Box", "fields"), Some(0.0));
}

#[test]
fn every_fixture_is_measured_completely_and_the_facts_are_valid() {
    let facts = facts();
    assert!(facts.problems().is_empty(), "{:?}", facts.problems());
    for (path, _) in FIXTURES {
        assert_eq!(
            facts.file_state(path).unwrap().state,
            Completeness::Complete
        );
    }
    for observation in &facts.observations {
        assert_eq!(observation.producer.name, "kiro-code");
        assert_eq!(observation.metric.definition_version, "1");
        assert!(observation.line >= 1);
    }
}

#[test]
fn a_file_that_does_not_parse_has_no_observation() {
    let mut facts = CodeFacts::default();
    measure_into("broken.rs", "fn ok() {}\nfn broken( {\n", &mut facts);
    assert!(facts.observations.is_empty(), "no rule may be judged on it");
    let cell = facts.file_state("broken.rs").unwrap();
    assert_eq!(cell.state, Completeness::Failed);
    assert_eq!(cell.line, Some(2));
}

#[test]
fn a_file_without_a_profile_is_unavailable_not_empty() {
    let mut facts = CodeFacts::default();
    measure_into("notes.txt", "hello", &mut facts);
    measure_into("Main.java", "class Main {}", &mut facts);
    assert!(facts.observations.is_empty());
    assert!(
        facts
            .coverage
            .iter()
            .all(|cell| cell.state == Completeness::Unavailable)
    );
}

#[test]
fn line_endings_do_not_change_a_measurement() {
    let lf = include_str!("../fixtures/metrics/sample.rs").replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    let mut a = CodeFacts::default();
    let mut b = CodeFacts::default();
    measure_into("sample.rs", &lf, &mut a);
    measure_into("sample.rs", &crlf, &mut b);
    assert_eq!(a, b);
}

#[test]
fn equal_routines_get_distinct_ordinals() {
    let mut facts = CodeFacts::default();
    let text = "#[cfg(unix)]\nfn same(a: u8) {}\n#[cfg(windows)]\nfn same(a: u8) {}\n";
    measure_into("twins.rs", text, &mut facts);
    let ordinals: Vec<u32> = facts
        .observations
        .iter()
        .filter(|o| o.entity.scope == Scope::Routine && o.metric.name == "cyclomatic")
        .map(|o| o.entity.ordinal)
        .collect();
    assert_eq!(ordinals, [0, 1]);
}

#[test]
fn arbitrary_text_never_panics() {
    // A small deterministic generator; no random-number dependency.
    let alphabet: Vec<char> = "fn {}()[];:=<>&|!?.,'\"\\ \n\tabcxyz019/*#@-+%_"
        .chars()
        .collect();
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    for round in 0..300 {
        let mut text = String::new();
        for _ in 0..(round % 97 + 1) {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            text.push(alphabet[(state % alphabet.len() as u64) as usize]);
        }
        for path in ["a.rs", "a.py", "a.ts", "a.tsx", "a.js"] {
            let mut facts = CodeFacts::default();
            measure_into(path, &text, &mut facts);
            assert_eq!(facts.coverage.len(), 1);
        }
    }
}

#[test]
fn full_measurement_matches_the_regression_record() {
    let facts = facts();
    let actual: Vec<String> = facts
        .observations
        .iter()
        .map(|o| {
            format!(
                "{} {} {}#{} {} = {}",
                o.entity.path,
                o.entity.scope.label(),
                o.entity.qualified_name,
                o.entity.ordinal,
                o.metric.name,
                o.value
            )
        })
        .collect();
    let actual = serde_json::to_string_pretty(&actual).unwrap() + "\n";
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/fixtures/metrics/expected.json"
    );
    if std::env::var_os("KIRO_UPDATE_METRICS").is_some() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
    assert_eq!(actual, expected);
}

#[test]
fn javascript_uses_the_typescript_tables_with_its_own_field_kind() {
    let text = "const add = (a, b) => a + b;\nclass K {\n  field = 1;\n  m() { if (this.field || add(1, 2)) { return 1; } }\n}\n";
    let mut facts = CodeFacts::default();
    measure_into("k.js", text, &mut facts);
    assert_eq!(
        facts.file_state("k.js").unwrap().state,
        Completeness::Complete
    );
    assert_eq!(routine(&facts, "k.js", "add"), [2.0, 1.0, 0.0, 1.0, 0.0]);
    // if and || are decisions; the if opens one level; if and return are statements.
    assert_eq!(routine(&facts, "k.js", "K.m"), [0.0, 3.0, 1.0, 1.0, 2.0]);
    assert_eq!(
        value(&facts, "k.js", Scope::Type, "K", "methods"),
        Some(1.0)
    );
    assert_eq!(value(&facts, "k.js", Scope::Type, "K", "fields"), Some(1.0));
}
