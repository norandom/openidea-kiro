//! No-panic robustness tests (task 5.3, requirements 12.1 and 12.2): every
//! parser and the shared scanner are fed hostile inputs; each call must
//! return with the structure it recognised rather than panicking or
//! discarding the document.

use kiro_core::{design, md, requirements, roadmap, spec, tasks};

/// Runs the scanner and every parser over `text` and returns the scan for
/// recognisability assertions.
fn parse_everything(text: &str) -> md::Scan<'_> {
    let scanned = md::scan(text, "hostile.md");
    let (meta, spec_findings) = spec::parse(text, "hostile.json");
    let req = requirements::parse(text, "hostile-requirements.md");
    let task_doc = tasks::parse(text, "hostile-tasks.md");
    let design_doc = design::parse(text, "hostile-design.md");
    let roadmap_doc = roadmap::parse(text, "hostile-roadmap.md");
    // Touch the recognised structure so the calls are not optimised away
    // and partial data is observable (requirement 12.2).
    let _ = (
        meta.phase,
        spec_findings.len(),
        req.requirements.len(),
        task_doc.tasks.len(),
        design_doc.components.len(),
        roadmap_doc.entries.len(),
    );
    scanned
}

#[test]
fn empty_text_returns_documents() {
    let scanned = parse_everything("");
    assert!(scanned.lines.is_empty());
    assert!(scanned.outline.is_empty());
    assert!(scanned.fences.is_empty());
}

#[test]
fn lone_byte_order_mark_returns_documents() {
    // A leading BOM is tolerated by the scanner contract; the BOM-only
    // line is kept as one raw line when present.
    let scanned = parse_everything("\u{FEFF}");
    assert!(
        scanned.lines.len() <= 1,
        "the BOM line is either kept or empty"
    );
}

#[test]
fn ten_thousand_nested_headings_are_recognised() {
    // ATX headings are 1-6 `#` (md contract); cycle the levels across
    // ten thousand heading lines.
    let mut text = String::new();
    for i in 1..=10_000usize {
        for _ in 0..(i % 6) + 1 {
            text.push('#');
        }
        text.push_str(" heading ");
        text.push_str(&i.to_string());
        text.push('\n');
    }
    let scanned = parse_everything(&text);
    assert_eq!(scanned.outline.len(), 10_000, "every heading is recognised");
}

#[test]
fn ten_thousand_fence_markers_and_one_unclosed_fence() {
    // Ten thousand fence marker lines open and close alternately; the
    // scanner tracks every one without panicking.
    let mut alternating = String::new();
    for _ in 0..10_000 {
        alternating.push_str("```\n");
    }
    let scanned = parse_everything(&alternating);
    assert_eq!(scanned.fences.len(), 5_000, "pairs of markers form fences");

    // A single unclosed fence over ten thousand content lines runs to the
    // last line and is reported once (requirement 12.2).
    let mut unclosed = String::from("```\n");
    for i in 0..10_000 {
        unclosed.push_str("content ");
        unclosed.push_str(&i.to_string());
        unclosed.push('\n');
    }
    let scanned = parse_everything(&unclosed);
    assert!(
        scanned
            .findings
            .iter()
            .any(|f| f.code == "MD_FENCE_UNCLOSED"),
        "the unclosed fence is reported"
    );
}

#[test]
fn one_megabyte_lines_are_scanned() {
    let mut text = String::with_capacity(2 * 1024 * 1024 + 2);
    for line in 0..2 {
        for _ in 0..1024 * 1024 {
            text.push('a');
        }
        text.push('\n');
        let _ = line;
    }
    let scanned = parse_everything(&text);
    assert_eq!(scanned.lines.len(), 2, "both megabyte lines are typed");
}

/// Fixed-seed linear congruential generator (deterministic per 12.5).
struct Lcg(u64);

impl Lcg {
    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next_u64() >> 33) as usize % n.max(1)
    }
}

#[test]
fn fixed_seed_generated_documents_never_panic() {
    const ALPHABET: &[char] = &[
        '#', '-', '[', ']', '_', ':', '|', '`', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
        'a', 'b', 'c', 'X', 'Y', 'Z', ' ', '.',
    ];
    let mut rng = Lcg(0x5EED_1234_ABCD_0001);
    for doc in 0..256u32 {
        let target = 2_000 + rng.below(2_000);
        let mut text = String::new();
        while text.len() < target {
            let chunk = 1 + rng.below(60);
            for _ in 0..chunk {
                let ch = ALPHABET
                    .get(rng.below(ALPHABET.len()))
                    .copied()
                    .unwrap_or('x');
                text.push(ch);
            }
            text.push('\n');
        }
        let scanned = parse_everything(&text);
        // Every line is typed in one forward pass; content inside generated
        // fences is intentionally not outline material, so recognisability
        // is pinned by the dedicated corpora tests above.
        let line_count = text.lines().count();
        assert_eq!(
            scanned.lines.len(),
            line_count,
            "document {doc}: every line is typed"
        );
    }
}
