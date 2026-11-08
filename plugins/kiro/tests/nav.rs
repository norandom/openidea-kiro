//! Navigation integration tests (tasks 1.2, 4.1, 4.2).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use kiro_core::finding::codes::SPEC_DOC_MISSING;
use openidea_plugin_kiro::context::CommandContext;
use openidea_plugin_kiro::host::FakeHost;

pub fn setup_nav_host() -> FakeHost {
    let host = FakeHost::new().with_root("C:\\proj");

    host.with_file(
        ".kiro/steering/roadmap.md",
        include_str!("fixtures/nav/.kiro/steering/roadmap.md"),
    )
    .with_file(
        ".kiro/specs/alpha/spec.json",
        include_str!("fixtures/nav/.kiro/specs/alpha/spec.json"),
    )
    .with_file(
        ".kiro/specs/alpha/requirements.md",
        include_str!("fixtures/nav/.kiro/specs/alpha/requirements.md"),
    )
    .with_file(
        ".kiro/specs/alpha/design.md",
        include_str!("fixtures/nav/.kiro/specs/alpha/design.md"),
    )
    .with_file(
        ".kiro/specs/alpha/tasks.md",
        include_str!("fixtures/nav/.kiro/specs/alpha/tasks.md"),
    )
    .with_file(
        ".kiro/specs/beta/spec.json",
        include_str!("fixtures/nav/.kiro/specs/beta/spec.json"),
    )
    .with_file(
        ".kiro/specs/beta/requirements.md",
        include_str!("fixtures/nav/.kiro/specs/beta/requirements.md"),
    )
    .with_file(
        ".kiro/specs/beta/design.md",
        include_str!("fixtures/nav/.kiro/specs/beta/design.md"),
    )
    .with_file(
        ".kiro/specs/beta/tasks.md",
        include_str!("fixtures/nav/.kiro/specs/beta/tasks.md"),
    )
    .with_file(
        ".kiro/specs/gamma/spec.json",
        include_str!("fixtures/nav/.kiro/specs/gamma/spec.json"),
    )
    .with_file(
        ".kiro/specs/gamma/requirements.md",
        include_str!("fixtures/nav/.kiro/specs/gamma/requirements.md"),
    )
    .with_file(
        ".kiro/specs/gamma/tasks.md",
        include_str!("fixtures/nav/.kiro/specs/gamma/tasks.md"),
    )
}

#[test]
fn fixture_tree_smoke_test() {
    let host = setup_nav_host();
    let mut ctx = CommandContext::new(&host, None).expect("context build");

    let alpha = ctx.load("alpha");
    assert!(
        !alpha.findings.iter().any(|f| f.code == SPEC_DOC_MISSING),
        "alpha should have no missing documents"
    );
    assert!(alpha.design.is_some());
    assert!(alpha.requirements.is_some());
    assert!(alpha.tasks.is_some());

    let beta = ctx.load("beta");
    assert!(
        !beta.findings.iter().any(|f| f.code == SPEC_DOC_MISSING),
        "beta should have no missing documents"
    );
    assert!(beta.design.is_some());
    assert!(beta.requirements.is_some());
    assert!(beta.tasks.is_some());

    let gamma = ctx.load("gamma");
    let gamma_missing: Vec<_> = gamma
        .findings
        .iter()
        .filter(|f| f.code == SPEC_DOC_MISSING)
        .collect();
    assert_eq!(gamma_missing.len(), 1);
    assert!(
        gamma_missing[0].file.ends_with("design.md"),
        "gamma should report exactly its design as missing"
    );
    assert!(gamma.design.is_none());
    assert!(gamma.requirements.is_some());
    assert!(gamma.tasks.is_some());
}
