//! Performance test (task 5.3, requirement 12.3): a synthetic 16 MiB
//! tasks-shaped document timed through the scanner and the tasks parser.
//! The two-second bound is asserted only in release builds; debug builds
//! run the same document for coverage without the timing gate.

use std::time::Instant;

use kiro_core::md;
use kiro_core::tasks;

/// Builds a tasks-shaped document of at least `target_bytes` with major
/// tasks, sub-tasks and requirement annotations.
fn build_tasks_document(target_bytes: usize) -> String {
    let mut text = String::with_capacity(target_bytes + 1024);
    let mut major = 0usize;
    while text.len() < target_bytes {
        major += 1;
        text.push_str(&format!(
            "- [ ] {major}. Major task {major} of the synthetic corpus\n"
        ));
        for sub in 1..=3 {
            text.push_str(&format!(
                "  - [ ] {major}.{sub} Sub task with annotation payload\n    - _Requirements: {major}.1_\n"
            ));
        }
    }
    text
}

#[test]
fn sixteen_mib_tasks_document_parses_within_two_seconds_in_release() {
    let text = build_tasks_document(16 * 1024 * 1024);
    assert!(text.len() >= 16 * 1024 * 1024, "the corpus reaches 16 MiB");

    let scan_start = Instant::now();
    let scanned = md::scan(&text, "perf-tasks.md");
    let scan_elapsed = scan_start.elapsed();

    let parse_start = Instant::now();
    let doc = tasks::parse(&text, "perf-tasks.md");
    let parse_elapsed = parse_start.elapsed();

    // The document is recognised, not discarded (requirement 12.2): one
    // major with three sub-tasks per group pins the attachment shape.
    assert!(!doc.tasks.is_empty(), "majors are recognised");
    assert_eq!(
        doc.all().count(),
        4 * doc.tasks.len(),
        "every major carries exactly three sub-tasks"
    );
    assert!(
        !scanned.lines.is_empty(),
        "the scanner typed every line in one forward pass"
    );

    if !cfg!(debug_assertions) {
        assert!(
            scan_elapsed.as_secs() < 2,
            "scan of 16 MiB took {scan_elapsed:?}"
        );
        assert!(
            parse_elapsed.as_secs() < 2,
            "parse of 16 MiB took {parse_elapsed:?}"
        );
    } else {
        // Keep the timings observable in debug runs without gating on them.
        let _ = (scan_elapsed, parse_elapsed);
    }
}
