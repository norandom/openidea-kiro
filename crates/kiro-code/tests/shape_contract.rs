//! Token shapes and their similarity (clone-constraints 1.2, 1.5, 1.6,
//! 3.2). Expected shapes and ratios were written by hand; the scores of
//! the labeled rewrites are measurements and are marked as such.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use std::collections::BTreeMap;

use kiro_code::scan::scan;
use kiro_code::shapes::{length_ceiling, multiset_ceiling, shapes_of, similarity};
use kiro_code::{SourceBundle, SourceText};

fn close(actual: f64, expected: f64) -> bool {
    (actual - expected).abs() < 1e-9
}

/// One character per distinct token, as the scan does it.
fn encode(pair: [&[String]; 2]) -> [String; 2] {
    let mut characters: BTreeMap<&str, char> = BTreeMap::new();
    pair.map(|tokens| {
        tokens
            .iter()
            .map(|token| {
                let next = char::from_u32(0xE000 + characters.len() as u32).unwrap();
                *characters.entry(token.as_str()).or_insert(next)
            })
            .collect()
    })
}

fn shape(path: &str, text: &str) -> Vec<String> {
    let mut shapes = shapes_of(path, text, 0);
    assert_eq!(shapes.len(), 1, "one routine in {path}");
    shapes.remove(0).1
}

fn score(path: &str, a: &str, b: &str) -> f64 {
    let (a, b) = (shape(path, a), shape(path, b));
    let [a, b] = encode([&a, &b]);
    similarity(&a, &b)
}

#[test]
fn a_rust_shape_keeps_structure_and_drops_names_literals_and_comments() {
    let tokens = shape(
        "a.rs",
        "fn add(a: u32, b: u32) -> u32 {\n    // sum first\n    let total = a + b;\n    total * 2\n}\n",
    );
    let expected = [
        "fn", "$I", "(", "$I", ":", "u32", ",", "$I", ":", "u32", ")", "->", "u32", "{", "let",
        "$I", "=", "$I", "+", "$I", ";", "$I", "*", "$L", "}",
    ];
    assert_eq!(tokens, expected);
}

#[test]
fn a_python_shape_counts_a_string_as_one_literal() {
    let tokens = shape(
        "a.py",
        "def area(w, h):\n    \"\"\"Width times height.\"\"\"\n    return w * h + 1\n",
    );
    let expected = [
        "def", "$I", "(", "$I", ",", "$I", ")", ":", "$L", "return", "$I", "*", "$I", "+", "$L",
    ];
    assert_eq!(tokens, expected);
}

#[test]
fn a_nested_routine_is_not_part_of_the_shape_around_it() {
    let shapes = shapes_of(
        "a.py",
        "def outer(x):\n    def inner(y):\n        return y + 1\n    return inner(x)\n",
        0,
    );
    let names: Vec<&str> = shapes.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["outer", "outer.inner"]);
    assert_eq!(
        shapes[0].1,
        [
            "def", "$I", "(", "$I", ")", ":", "return", "$I", "(", "$I", ")"
        ]
    );
}

#[test]
fn the_ratio_is_twice_the_matched_length_over_the_total_length() {
    // Nine of ten characters match: 2 * 9 / 20.
    assert!(close(similarity("abcdefghij", "abcdefghix"), 0.9));
    // Three of four private-use characters match: 2 * 3 / 8.
    assert!(close(
        similarity(
            "\u{e000}\u{e001}\u{e002}\u{e003}",
            "\u{e000}\u{e001}\u{e002}\u{e004}"
        ),
        0.75
    ));
    assert!(close(similarity("", ""), 1.0));
    assert!(close(similarity("same", "same"), 1.0));
}

#[test]
fn the_argument_order_is_canonical() {
    // The library's ratio is asymmetric here: (tide, diet) is 0.25 and
    // (diet, tide) is 0.5. Equal length: the smaller text goes first.
    assert!(close(similarity("tide", "diet"), 0.5));
    assert!(close(similarity("diet", "tide"), 0.5));
    // Different length: the shorter text goes first, whatever the order.
    assert!(close(
        similarity("abcabc", "abc"),
        similarity("abc", "abcabc")
    ));
}

#[test]
fn the_ceilings_are_what_the_arithmetic_says() {
    assert!(close(length_ceiling(10, 12), 20.0 / 22.0));
    assert!(close(length_ceiling(9, 12), 18.0 / 21.0));
    assert!(close(length_ceiling(0, 0), 1.0));
    // In common: one `a` and one `b`. 2 * 2 / 8.
    assert!(close(multiset_ceiling("aabb", "abcc"), 0.5));
    assert!(close(multiset_ceiling("abc", "xyz"), 0.0));
}

#[test]
fn no_pair_scores_above_a_ceiling_so_refusing_below_the_threshold_loses_nothing() {
    // Every string over three letters up to length 5: 364 strings.
    let mut all = vec![String::new()];
    let mut last = vec![String::new()];
    for _ in 0..5 {
        last = last
            .iter()
            .flat_map(|prefix| ["a", "b", "c"].map(|letter| format!("{prefix}{letter}")))
            .collect();
        all.extend(last.iter().cloned());
    }
    assert_eq!(all.len(), 364);
    let mut pairs = 0_u32;
    for (index, a) in all.iter().enumerate() {
        for b in &all[index..] {
            let ratio = similarity(a, b);
            let by_length = length_ceiling(a.len(), b.len());
            let by_tokens = multiset_ceiling(a, b);
            assert!(
                ratio <= by_length + 1e-12,
                "{a:?} {b:?} {ratio} > {by_length}"
            );
            assert!(
                ratio <= by_tokens + 1e-12,
                "{a:?} {b:?} {ratio} > {by_tokens}"
            );
            pairs += 1;
        }
    }
    assert_eq!(pairs, 364 * 365 / 2);
}

const LOAD_USERS: &str = "pub fn load_users(path: &str) -> Vec<String> {
    let text = read(path);
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out.sort();
    out
}
";

#[test]
fn a_renamed_copy_is_found_across_files_and_other_routines_are_not() {
    // The copy: other names and another literal-free body, same structure.
    let copy = LOAD_USERS
        .replace("load_users", "read_orders")
        .replace("text", "content")
        .replace("trimmed", "entry")
        .replace("out", "orders");
    let small = "pub fn tiny(a: u32) -> u32 {\n    let b = a + 1;\n    b * 2\n}\n";
    let python = "def load_users(path):\n    text = read(path)\n    out = []\n    for line in text.lines():\n        trimmed = line.trim()\n        if trimmed.is_empty():\n            continue\n        out.push(trimmed.to_string())\n    out.sort()\n    return out\n";
    let bundle = SourceBundle {
        files: vec![
            SourceText {
                path: "src/a.rs".to_string(),
                text: format!("{LOAD_USERS}\n{small}"),
            },
            SourceText {
                path: "src/b.rs".to_string(),
                text: copy,
            },
            SourceText {
                path: "tools/c.py".to_string(),
                text: python.to_string(),
            },
        ],
        unreadable: Vec::new(),
    };
    let facts = scan(&bundle).families;
    let names: Vec<(&str, &str, u32)> = facts
        .routines
        .iter()
        .map(|routine| (routine.path.as_str(), routine.name.as_str(), routine.line))
        .collect();
    // `tiny` has two statements and is not compared.
    assert_eq!(
        names,
        [
            ("src/a.rs", "load_users", 1),
            ("src/b.rs", "read_orders", 1),
            ("tools/c.py", "load_users", 1),
        ]
    );
    assert_eq!(facts.routines[0].end_line, 13);
    assert_eq!(facts.coverage.routines, 4);
    assert_eq!(facts.coverage.eligible, 3);
    assert!(facts.coverage.complete);
    // One pair of one language; the Python routine is never a partner.
    assert_eq!(facts.edges.len(), 1);
    assert_eq!((facts.edges[0].a, facts.edges[0].b), (0, 1));
    assert!(close(facts.edges[0].ratio, 1.0));
    assert_eq!(facts.coverage.scored_pairs, 1);
}

const GUARDS: &str = "def price(order):
    if order is None:
        return 0
    if order.cancelled:
        return 0
    if order.total < 0:
        return 0
    base = order.total
    tax = base * 0.2
    return base + tax
";

const LADDER: &str = "def price(order):
    if order is not None:
        if not order.cancelled:
            if order.total >= 0:
                base = order.total
                tax = base * 0.2
                return base + tax
    return 0
";

const CHAIN: &str = "def label(code):
    if code == 1:
        return \"one\"
    if code == 2:
        return \"two\"
    if code == 3:
        return \"three\"
    if code == 4:
        return \"four\"
    return \"many\"
";

const TABLE: &str = "def label(code):
    names = {1: \"one\", 2: \"two\", 3: \"three\", 4: \"four\"}
    return names.get(code, \"many\")
";

#[test]
fn the_rule_finds_a_rename_and_not_a_rewrite() {
    // A rename changes no token of the shape.
    let renamed = GUARDS
        .replace("price", "cost")
        .replace("order", "basket")
        .replace("0.2", "0.19");
    assert!(close(score("a.py", GUARDS, &renamed), 1.0));

    // Measured, not derived by hand: the same behaviour in another shape
    // stays below the threshold of 0.9.
    let ladder = score("a.py", GUARDS, LADDER);
    let table = score("a.py", CHAIN, TABLE);
    println!("guards to ladder: {ladder:.4}; chain to table: {table:.4}");
    assert!(ladder < 0.9, "{ladder}");
    assert!(table < 0.9, "{table}");
}
