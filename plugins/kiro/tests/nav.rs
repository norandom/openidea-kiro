//! Navigation integration tests (tasks 1.2, 4.1, 4.2).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use kiro_core::finding::codes::SPEC_DOC_MISSING;
use openidea_plugin_kiro::commands::{activate, run_command};
use openidea_plugin_kiro::context::CommandContext;
use openidea_plugin_kiro::host::{FakeHost, HostEvent, Level};

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

fn activate_host(host: &FakeHost) {
    activate(
        host,
        &[
            "project-read".to_owned(),
            "editor-read".to_owned(),
            "editor-write".to_owned(),
        ],
    )
    .expect("activation must succeed");
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

#[test]
fn nav_activation_registers_static_ids_and_per_spec_variants() {
    let host = setup_nav_host();
    activate_host(&host);

    let registered = host.registered();
    assert!(registered.contains_key("nav.context"));
    assert!(registered.contains_key("nav.context.insert"));
    assert!(registered.contains_key("nav.files"));
    assert!(registered.contains_key("nav.task-files"));
    assert!(registered.contains_key("nav.to-requirements"));
    assert!(registered.contains_key("nav.to-design"));
    assert!(registered.contains_key("nav.to-tasks"));
}

#[test]
fn nav_context_over_shared_file_and_per_spec() {
    let host = setup_nav_host().with_active("C:\\proj\\src\\shared.rs");
    activate_host(&host);

    // 1. Cross-spec context check over shared file
    let out = run_command(&host, "nav.context", None).unwrap().unwrap();
    assert!(out.contains("File: src/shared.rs"));
    assert!(out.contains("Checked: alpha, beta, gamma (3 specs)"));
    assert!(out.contains("Covered by: alpha, beta"));
    assert!(out.contains("No design: gamma"));

    // Alpha details
    assert!(out.contains("alpha\n  Components: Alpha\n  Tasks:\n    2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:6)\n  Requirements:\n    1.1 (.kiro/specs/alpha/requirements.md:11)\n    1.2 (.kiro/specs/alpha/requirements.md:12)\n    9.9 (not found)"));

    // Beta details
    assert!(out.contains("beta\n  Components: Beta\n  Tasks:\n    1.1 [ ] Beta work (.kiro/specs/beta/tasks.md:4)\n  Requirements:\n    1.1 (.kiro/specs/beta/requirements.md:11)"));

    // Verify notification event: Warning level (because gamma has no design) and title
    let notifies: Vec<_> = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Notify { .. }))
        .collect();
    let last_notify = notifies.last().unwrap();
    if let HostEvent::Notify { level, title, body } = last_notify {
        assert_eq!(*level, Level::Warning);
        assert_eq!(title, "Kiro: shared.rs — alpha, beta");
        assert_eq!(body, &out);
    }

    // Verify active file was NEVER read
    assert!(
        !host
            .events()
            .iter()
            .any(|e| matches!(e, HostEvent::Read(p) if p.contains("shared.rs"))),
        "code files must never be read by nav.context"
    );

    // 2. Per-spec variant restricts to one spec
    let alpha_out = run_command(&host, "nav.context.alpha", None)
        .unwrap()
        .unwrap();
    assert!(alpha_out.contains("Checked: alpha (1 spec)"));
    assert!(alpha_out.contains("Covered by: alpha"));
    assert!(!alpha_out.contains("beta"));
    assert!(!alpha_out.contains("gamma"));

    let notifies_after: Vec<_> = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Notify { .. }))
        .collect();
    if let HostEvent::Notify { level, title, .. } = notifies_after.last().unwrap() {
        assert_eq!(*level, Level::Info);
        assert_eq!(title, "Kiro: shared.rs — alpha");
    }
}

#[test]
fn nav_context_error_and_notice_paths() {
    let host = setup_nav_host();
    activate_host(&host);

    // 1. No active file
    let err = run_command(&host, "nav.context", None).unwrap_err();
    assert_eq!(
        err,
        "no active file: open a file in the editor and run the command again"
    );

    // 2. File outside project
    let host_outside = setup_nav_host().with_active("D:\\other\\file.rs");
    activate_host(&host_outside);
    let err_outside = run_command(&host_outside, "nav.context", None).unwrap_err();
    assert_eq!(
        err_outside,
        "active file `D:/other/file.rs` is outside the project"
    );

    // 3. Uncovered file in project
    let host_uncovered = setup_nav_host().with_active("C:\\proj\\src\\uncovered.rs");
    activate_host(&host_uncovered);
    let unc_out = run_command(&host_uncovered, "nav.context", None)
        .unwrap()
        .unwrap();
    assert!(unc_out.contains("Covered by: none"));
    assert!(unc_out.contains("Not covered by: alpha, beta"));
    assert!(unc_out.contains("No design: gamma"));

    // 4. Spec document notice
    let host_spec = setup_nav_host().with_active("C:\\proj\\.kiro\\specs\\alpha\\design.md");
    activate_host(&host_spec);
    let spec_out = run_command(&host_spec, "nav.context", None)
        .unwrap()
        .unwrap();
    assert_eq!(
        spec_out,
        "File: .kiro/specs/alpha/design.md\n.kiro/specs/alpha/design.md is the design document of spec `alpha`. Run `nav.files` (Kiro: Files by Component) for its components and files."
    );

    // 5. Unexpected token
    let host_arg = setup_nav_host().with_active("C:\\proj\\src\\shared.rs");
    activate_host(&host_arg);
    let err_arg = run_command(&host_arg, "nav.context", Some("alpha extra")).unwrap_err();
    assert!(err_arg.contains("`nav.context` does not take `extra`"));

    // 6. No known spec (roadmap removed)
    let host_no_specs = FakeHost::new()
        .with_root("C:\\proj")
        .with_active("C:\\proj\\src\\shared.rs");
    activate_host(&host_no_specs);
    let err_no_specs = run_command(&host_no_specs, "nav.context", None).unwrap_err();
    assert!(err_no_specs.contains("no spec is known"));
}

#[test]
fn nav_context_insert_variant() {
    // 1. Rust file insertion
    let host = setup_nav_host().with_active("C:\\proj\\src\\shared.rs");
    activate_host(&host);

    let out = run_command(&host, "nav.context.insert", None)
        .unwrap()
        .unwrap();
    let expected_insert = "\
// kiro-spec: alpha; components: Alpha; tasks: 2.1; requirements: 1.1, 1.2, 9.9
// kiro-spec: beta; components: Beta; tasks: 1.1; requirements: 1.1
";
    assert_eq!(out, expected_insert);

    let insert_events: Vec<_> = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Insert(_)))
        .collect();
    assert_eq!(insert_events.len(), 1);
    if let HostEvent::Insert(text) = &insert_events[0] {
        assert_eq!(text, expected_insert);
    }

    // 2. Markdown file insertion
    let host_md = setup_nav_host()
        .with_file(
            ".kiro/specs/alpha/design.md",
            "# Design Document: alpha\n\n## File Structure Plan\n\n| Component | Files |\n|---|---|\n| Alpha | `docs/guide.md` |\n",
        )
        .with_active("C:\\proj\\docs\\guide.md");
    activate_host(&host_md);

    let out_md = run_command(&host_md, "nav.context.alpha", Some("insert"))
        .unwrap()
        .unwrap();
    assert!(out_md.starts_with("<!-- kiro-spec: alpha; components: Alpha;"));
    assert!(out_md.ends_with(" -->\n"));

    // 3. Uncovered file nothing-to-insert error
    let host_uncovered = setup_nav_host().with_active("C:\\proj\\src\\uncovered.rs");
    activate_host(&host_uncovered);
    let err_unc = run_command(&host_uncovered, "nav.context.insert", None).unwrap_err();
    assert_eq!(
        err_unc,
        "nothing to insert: `src/uncovered.rs` is not covered by alpha, beta, gamma"
    );
    assert!(
        !host_uncovered
            .events()
            .iter()
            .any(|e| matches!(e, HostEvent::Insert(_)))
    );

    // 4. Spec document nothing-to-insert error
    let host_spec = setup_nav_host().with_active("C:\\proj\\.kiro\\specs\\alpha\\design.md");
    activate_host(&host_spec);
    let err_spec = run_command(&host_spec, "nav.context.insert", None).unwrap_err();
    assert_eq!(
        err_spec,
        "nothing to insert: `.kiro/specs/alpha/design.md` is a spec document"
    );
}

#[test]
fn nav_files_by_component_and_per_spec() {
    let host = setup_nav_host();
    activate_host(&host);

    // Alpha via argument
    let out_alpha = run_command(&host, "nav.files", Some("alpha"))
        .unwrap()
        .unwrap();
    // Alpha via per-spec variant
    let out_alpha_var = run_command(&host, "nav.files.alpha", None)
        .unwrap()
        .unwrap();
    assert_eq!(out_alpha, out_alpha_var);

    assert!(out_alpha.contains("Spec: alpha"));
    assert!(out_alpha.contains("Design: .kiro/specs/alpha/design.md"));
    assert!(out_alpha.contains("Tasks: .kiro/specs/alpha/tasks.md"));
    assert!(out_alpha.contains(
        "Components: 2 (files: 3, unassigned: 1); tasks: 7 (unscoped: 4, unknown boundary: 1)"
    ));
    assert!(out_alpha.contains("Alpha (2 files, 1 tasks)"));
    assert!(out_alpha.contains("  src/alpha.rs — Alpha logic"));
    assert!(out_alpha.contains("  src/shared.rs"));
    assert!(out_alpha.contains("  task 2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:6)"));
    assert!(out_alpha.contains("Empty (no files, 1 tasks)"));
    assert!(out_alpha.contains("  task 2.2 [ ] Empty work (.kiro/specs/alpha/tasks.md:9)"));
    assert!(out_alpha.contains("Unassigned files (1)"));
    assert!(out_alpha.contains("  src/util.rs"));
    assert!(out_alpha.contains("Unscoped tasks (4)"));
    assert!(out_alpha.contains("Unknown boundary (1)"));
    assert!(out_alpha.contains("  task 3.1 [ ] Glue work (.kiro/specs/alpha/tasks.md:12) — Glue"));

    // Gamma (missing design)
    let out_gamma = run_command(&host, "nav.files.gamma", None)
        .unwrap()
        .unwrap();
    assert!(out_gamma.contains("Design: .kiro/specs/gamma/design.md (missing)"));
    assert!(out_gamma.contains("Components: missing; tasks: 2 (unscoped: 1, unknown boundary: 1)"));
    assert!(out_gamma.contains("Unknown boundary (1)"));
    assert!(out_gamma.contains("  task 1.1 [ ] Gamma work (.kiro/specs/gamma/tasks.md:4) — Gamma"));
}

#[test]
fn nav_files_by_task_and_filters() {
    let host = setup_nav_host();
    activate_host(&host);

    // Full task tree
    let out_full = run_command(&host, "nav.task-files.alpha", None)
        .unwrap()
        .unwrap();
    assert!(out_full.contains("Count: 7 tasks (completed 2, unscoped 4)"));
    assert!(out_full.contains("1 [ ] Foundation (.kiro/specs/alpha/tasks.md:3) — unscoped"));
    assert!(out_full.contains("  1.1 [x] Unscoped task (.kiro/specs/alpha/tasks.md:4) — unscoped"));
    assert!(out_full.contains("2 [ ] Core (.kiro/specs/alpha/tasks.md:5) — unscoped"));
    assert!(out_full.contains("  2.1 [x] Build alpha (.kiro/specs/alpha/tasks.md:6) — Alpha"));
    assert!(out_full.contains("      src/alpha.rs"));
    assert!(out_full.contains("      src/shared.rs"));
    assert!(
        out_full.contains("  2.2 [ ] Empty work (.kiro/specs/alpha/tasks.md:9) — Empty (no files)")
    );
    assert!(
        out_full.contains("  3.1 [ ] Glue work (.kiro/specs/alpha/tasks.md:12) — Glue (unknown)")
    );

    // Major filter
    let out_major = run_command(&host, "nav.task-files.alpha", Some("alpha 2"))
        .unwrap()
        .unwrap();
    assert!(out_major.contains("filter: task 2"));
    assert!(out_major.contains("2 [ ] Core"));
    assert!(out_major.contains("  2.1 [x] Build alpha"));
    assert!(out_major.contains("  2.2 [ ] Empty work"));
    assert!(!out_major.contains("1 [ ] Foundation"));

    // Subtask filter
    let out_sub = run_command(&host, "nav.task-files.alpha", Some("alpha 2.1"))
        .unwrap()
        .unwrap();
    assert!(out_sub.contains("filter: task 2.1"));
    assert!(out_sub.contains("  2.1 [x] Build alpha"));
    assert!(out_sub.contains("      src/alpha.rs"));
    assert!(!out_sub.contains("2 [ ] Core"));

    // Unknown id error
    let err = run_command(&host, "nav.task-files.alpha", Some("alpha 99.9")).unwrap_err();
    assert_eq!(err, "task `99.9` not found in .kiro/specs/alpha/tasks.md");

    // Gamma (missing design)
    let out_gamma = run_command(&host, "nav.task-files.gamma", None)
        .unwrap()
        .unwrap();
    assert!(out_gamma.contains("Design: .kiro/specs/gamma/design.md (missing)"));
    assert!(
        out_gamma.contains("1.1 [ ] Gamma work (.kiro/specs/gamma/tasks.md:4) — Gamma (unknown)")
    );
}

#[test]
fn nav_outlines_and_per_spec() {
    let host = setup_nav_host();
    activate_host(&host);

    // Requirements outline
    let req_out = run_command(&host, "nav.to-requirements.alpha", None)
        .unwrap()
        .unwrap();
    assert!(req_out.contains("Spec: alpha"));
    assert!(req_out.contains("Document: .kiro/specs/alpha/requirements.md"));
    assert!(req_out.contains("1: # Requirements Document: alpha"));
    assert!(req_out.contains("3: ## Requirements"));

    // Design outline
    let des_out = run_command(&host, "nav.to-design.alpha", None)
        .unwrap()
        .unwrap();
    assert!(des_out.contains("Document: .kiro/specs/alpha/design.md"));
    assert!(des_out.contains("1: # Design Document: alpha"));
    assert!(des_out.contains("3: ## File Structure Plan"));

    // Tasks outline with merged task items
    let tasks_out = run_command(&host, "nav.to-tasks.alpha", None)
        .unwrap()
        .unwrap();
    assert!(tasks_out.contains("Document: .kiro/specs/alpha/tasks.md"));
    assert!(tasks_out.contains("1: # Implementation Plan: alpha"));
    assert!(tasks_out.contains("3: - [ ] 1 Foundation"));
    assert!(tasks_out.contains("4:   - [x] 1.1 Unscoped task"));
    assert!(tasks_out.contains("5: - [ ] 2 Core"));
    assert!(tasks_out.contains("6:   - [x] 2.1 Build alpha"));

    // Missing design (gamma)
    let gamma_des = run_command(&host, "nav.to-design.gamma", None)
        .unwrap()
        .unwrap();
    assert!(gamma_des.contains("Document: .kiro/specs/gamma/design.md (missing)"));
    assert!(gamma_des.contains("Outline: empty"));

    let notifies: Vec<_> = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Notify { .. }))
        .collect();
    if let HostEvent::Notify { level, title, .. } = notifies.last().unwrap() {
        assert_eq!(*level, Level::Warning);
        assert_eq!(title, "Kiro: gamma design.md");
    }
}
