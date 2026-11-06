//! Grammar matrix: every fixture in `fixtures/grammars` parsed with the
//! requirements parser and checked against the expected criterion ids and
//! finding codes (task 5.2). The expectations are the fixtures' reason to
//! exist: each row pins one grammar variant from the cc-sdd corpus.

use kiro_core::requirements;

struct Case {
    name: &'static str,
    text: &'static str,
    criteria: &'static [&'static str],
    codes: &'static [&'static str],
}

const G1_CANONICAL: &str = include_str!("../fixtures/grammars/g1-canonical.md");
const G1_DOTTED: &str = include_str!("../fixtures/grammars/g1-dotted.md");
const G2_RSTYLE: &str = include_str!("../fixtures/grammars/g2-rstyle.md");
const G3_AMENDMENT: &str = include_str!("../fixtures/grammars/g3-amendment.md");
const G4_EMPTY: &str = include_str!("../fixtures/grammars/g4-empty.md");
const FENCED_EXAMPLE: &str = include_str!("../fixtures/grammars/fenced-example.md");
const MALFORMED: &str = include_str!("../fixtures/grammars/malformed.md");

const CASES: &[Case] = &[
    Case {
        name: "g1-canonical.md",
        text: G1_CANONICAL,
        criteria: &["1.1", "1.2", "2.1"],
        codes: &[],
    },
    Case {
        name: "g1-dotted.md",
        text: G1_DOTTED,
        criteria: &["3.1", "3.2"],
        codes: &[],
    },
    Case {
        name: "g2-rstyle.md",
        text: G2_RSTYLE,
        // R-style bullets canonicalise to plain numeric ids (R1.1 -> 1.1).
        criteria: &["1.1", "1.2", "2.1"],
        codes: &[],
    },
    Case {
        name: "g3-amendment.md",
        text: G3_AMENDMENT,
        criteria: &["4.1", "A1.1"],
        codes: &[],
    },
    Case {
        name: "g4-empty.md",
        text: G4_EMPTY,
        criteria: &[],
        codes: &[],
    },
    Case {
        name: "fenced-example.md",
        text: FENCED_EXAMPLE,
        criteria: &["1.1", "2.1"],
        codes: &[],
    },
    Case {
        name: "malformed.md",
        text: MALFORMED,
        criteria: &["1.1"],
        // The duplicate requirement is rejected at its heading (one finding);
        // its criteria never parse. The unclosed fence is reported by the
        // scanner and the fenced content is ignored.
        codes: &[
            "MD_FENCE_UNCLOSED",
            "REQ_DUPLICATE_ID",
            "REQ_NON_NUMERIC_ID",
        ],
    },
];

#[test]
fn grammar_matrix_matches_expected_ids_and_codes() {
    for case in CASES {
        let doc = requirements::parse(case.text, case.name);
        let mut ids: Vec<String> = doc.criteria().map(|c| c.id.clone()).collect();
        ids.sort();
        assert_eq!(ids, case.criteria, "criterion ids for {}", case.name);
        let mut codes: Vec<&str> = doc.findings.iter().map(|f| f.code).collect();
        codes.sort();
        assert_eq!(codes, case.codes, "finding codes for {}", case.name);
    }
}

#[test]
fn the_matrix_covers_every_grammar_fixture_file() {
    // One row per file in fixtures/grammars; catches removed or renamed
    // rows. A brand-new fixture file cannot be detected without directory
    // enumeration (std::fs), which the no-I/O test harness forbids — new
    // fixtures must add their row here by hand.
    assert_eq!(CASES.len(), 7);
    let mut names: Vec<&str> = CASES.iter().map(|c| c.name).collect();
    names.sort();
    assert_eq!(
        names,
        vec![
            "fenced-example.md",
            "g1-canonical.md",
            "g1-dotted.md",
            "g2-rstyle.md",
            "g3-amendment.md",
            "g4-empty.md",
            "malformed.md",
        ]
    );
}
