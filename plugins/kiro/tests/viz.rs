//! End-to-end integration tests for `viz` commands over `FakeHost` (task 3.3).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use openidea_plugin_kiro::commands::{activate, run_command};
use openidea_plugin_kiro::host::FakeHost;

fn setup_minimal_host() -> FakeHost {
    let host = FakeHost::new().with_root("C:\\proj");

    host.with_file(
        ".kiro/steering/roadmap.md",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/steering/roadmap.md"),
    )
    .with_file(
        ".kiro/specs/demo/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/specs/demo/spec.json"),
    )
    .with_file(
        ".kiro/specs/demo/requirements.md",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/specs/demo/requirements.md"),
    )
    .with_file(
        ".kiro/specs/demo/design.md",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/specs/demo/design.md"),
    )
    .with_file(
        ".kiro/specs/demo/tasks.md",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/specs/demo/tasks.md"),
    )
    .with_file(
        ".kiro/specs/old/spec.json",
        include_str!("../../../crates/kiro-core/fixtures/minimal/.kiro/specs/old/spec.json"),
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
fn viz_end_to_end_scenarios() {
    let mut host = setup_minimal_host();
    activate_host(&host);

    // 1. Activation registers the six static viz ids with their titles
    let registered = host.registered();
    assert_eq!(
        registered.get("viz.status").map(|(t, _)| t.as_str()),
        Some("Kiro: Spec Status")
    );
    assert_eq!(
        registered.get("viz.status.insert").map(|(t, _)| t.as_str()),
        Some("Kiro: Spec Status — Insert at Cursor")
    );
    assert_eq!(
        registered.get("viz.graph").map(|(t, _)| t.as_str()),
        Some("Kiro: Spec Graph")
    );
    assert_eq!(
        registered.get("viz.graph.insert").map(|(t, _)| t.as_str()),
        Some("Kiro: Spec Graph — Insert at Cursor")
    );
    assert_eq!(
        registered.get("viz.tasks").map(|(t, _)| t.as_str()),
        Some("Kiro: Task Graph")
    );
    assert_eq!(
        registered.get("viz.tasks.insert").map(|(t, _)| t.as_str()),
        Some("Kiro: Task Graph — Insert at Cursor")
    );

    // 2. viz.graph with active design file notifies documented title with body starting with summary and returns Mermaid fence
    host.active = Some("C:\\proj\\.kiro\\specs\\demo\\design.md".to_owned());
    let notify_count_before = host.notifications().len();

    let graph_out = run_command(&host, "viz.graph", None)
        .expect("viz.graph should succeed")
        .expect("should return text");
    assert!(graph_out.contains("```mermaid"));

    let notifs = host.notifications();
    assert_eq!(notifs.len(), notify_count_before + 1);
    let last_notify = notifs.last().unwrap();
    assert_eq!(last_notify.1, "Kiro: demo graph (13 nodes)");
    assert!(
        last_notify.2.starts_with("Graph for demo:"),
        "notification message should start with summary, got: {}",
        last_notify.2
    );

    // 3. Afterwards the per-spec variants of all three commands exist for both fixture specs
    let reg_after = host.registered();
    assert!(reg_after.contains_key("viz.status.demo"));
    assert!(reg_after.contains_key("viz.status.old"));
    assert!(reg_after.contains_key("viz.graph.demo"));
    assert!(reg_after.contains_key("viz.graph.old"));
    assert!(reg_after.contains_key("viz.tasks.demo"));
    assert!(reg_after.contains_key("viz.tasks.old"));

    // 4. viz.tasks.insert produces exactly one insertion equal to returned text and one notification
    let inserts_before = host.inserts().len();
    let notifs_before = host.notifications().len();

    let tasks_ins_out = run_command(&host, "viz.tasks.insert", None)
        .expect("viz.tasks.insert should succeed")
        .expect("should return text");

    let inserts_after = host.inserts();
    let notifs_after = host.notifications();

    assert_eq!(inserts_after.len(), inserts_before + 1);
    assert_eq!(inserts_after.last().unwrap(), &tasks_ins_out);
    assert_eq!(notifs_after.len(), notifs_before + 1);

    // 5. The `text` option returns the node table without a fence
    let table_out = run_command(&host, "viz.graph", Some("demo text"))
        .expect("viz.graph demo text should succeed")
        .expect("should return text");
    assert!(
        table_out.contains("| Kind | Node | Links to |"),
        "expected table headers in table output"
    );
    assert!(
        !table_out.contains("```mermaid"),
        "table output should not contain Mermaid fence"
    );

    // 6. Unknown option fails with documented message and no notification
    let notifs_before_unknown = host.notifications().len();
    let unknown_err =
        run_command(&host, "viz.graph", Some("demo txt")).expect_err("unknown option should fail");
    assert!(
        unknown_err.contains("unknown option `txt` for viz.graph (accepted: text)"),
        "unexpected error message: {unknown_err}"
    );
    assert_eq!(
        host.notifications().len(),
        notifs_before_unknown,
        "failed command must produce no notification"
    );

    // 7. viz.graph text without explicit spec and viz.graph.demo with argument text fail with plugin-core's documented errors
    let no_spec_err = run_command(&host, "viz.graph", Some("text"))
        .expect_err("viz.graph text should fail because 'text' is not a spec");
    assert!(
        no_spec_err.contains("spec `text` was not found"),
        "unexpected no-spec error: {no_spec_err}"
    );

    let conflict_err = run_command(&host, "viz.graph.demo", Some("text"))
        .expect_err("viz.graph.demo text should fail because text conflicts with demo");
    assert!(
        conflict_err.contains("spec named twice: `demo` (command) and `text` (argument)"),
        "unexpected conflict error: {conflict_err}"
    );

    // 8. No active file with two known specs fails with the candidates message
    host.active = None;
    let candidate_err = run_command(&host, "viz.status", None)
        .expect_err("viz.status without active file should fail");
    assert!(
        candidate_err.contains("candidates: demo, old"),
        "unexpected candidates error: {candidate_err}"
    );

    // 9. The superseded fixture without tasks.md fails with the missing-document message
    let missing_tasks_err = run_command(&host, "viz.tasks.old", None)
        .expect_err("viz.tasks.old should fail due to missing tasks.md");
    assert!(
        missing_tasks_err.contains("spec `old` has no readable .kiro/specs/old/tasks.md"),
        "unexpected missing tasks error: {missing_tasks_err}"
    );

    // 10. Status for superseded spec shows successor chain
    let status_old_out = run_command(&host, "viz.status.old", None)
        .expect("viz.status.old should succeed")
        .expect("should return text");
    assert!(
        status_old_out.contains("Successor chain: old -> demo (terminal)"),
        "status output should contain successor chain: {status_old_out}"
    );
    let last_old_notif = host.notifications().last().unwrap().clone();
    assert!(
        last_old_notif
            .2
            .contains("Successor chain: old -> demo (terminal)"),
        "notification message should contain successor chain: {}",
        last_old_notif.2
    );

    // 11. The Phase: line of viz.status.demo equals that of ws.current.demo
    let status_demo_out = run_command(&host, "viz.status.demo", None)
        .expect("viz.status.demo should succeed")
        .expect("should return text");
    let ws_demo_out = run_command(&host, "ws.current.demo", None)
        .expect("ws.current.demo should succeed")
        .expect("should return text");

    let status_phase = status_demo_out
        .lines()
        .find(|l| l.starts_with("Phase:"))
        .expect("status output should have Phase line");
    let ws_phase = ws_demo_out
        .lines()
        .find(|l| l.starts_with("Phase:"))
        .expect("ws.current output should have Phase line");

    assert_eq!(status_phase, ws_phase);
}
