//! End-to-end dispatch tests over the fake host (task 4.1).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]

use openidea_plugin_kiro::commands::{
    activate, all_commands, run_command, run_command_with,
};
use openidea_plugin_kiro::host::{FakeHost, HostEvent};
use openidea_plugin_kiro::module::FeatureModule;
use openidea_plugin_kiro::present::Output;
use openidea_plugin_kiro::registry::{
    static_commands, CommandSpec, Group, Positional,
};

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
fn scenario_01_activation_registers_only_static_list() {
    let host = setup_minimal_host();
    activate_host(&host);

    let registered = host.registered();
    let expected = static_commands(&all_commands());

    assert_eq!(registered.len(), expected.len());
    for decl in expected {
        assert!(
            registered.contains_key(&decl.id),
            "expected static id {} in registrations",
            decl.id
        );
    }
}

#[test]
fn scenario_02_and_03_current_spec_from_active_file_and_variants_sync() {
    let mut host = setup_minimal_host();
    activate_host(&host);

    host.active = Some("C:\\proj\\.kiro\\specs\\demo\\design.md".to_owned());

    let out = run_command(&host, "ws.current", None)
        .expect("command must succeed")
        .expect("return text must be present");

    assert!(out.contains("Spec: demo (from the active file)"));
    assert!(out.contains("Phase: tasks-generated"));

    // Check per-spec variants are registered on host
    let registered = host.registered();
    assert!(registered.contains_key("ws.current.demo"));
    assert!(registered.contains_key("ws.current.old"));

    // Running the variant for the superseded spec reports it
    let old_out = run_command(&host, "ws.current.old", None)
        .expect("old variant must succeed")
        .expect("return text");
    assert!(old_out.contains("Spec: old (named explicitly)"));
    assert!(old_out.contains("Phase: superseded"));
}

#[test]
fn scenario_04_no_active_file_with_two_specs_fails_listing_candidates() {
    let mut host = setup_minimal_host();
    activate_host(&host);

    host.active = None;
    let err = run_command(&host, "ws.current", None).unwrap_err();
    assert_eq!(
        err,
        "no current spec: open a file under .kiro/specs/<spec>/ or run the per-spec command; candidates: demo, old"
    );
}

#[test]
fn scenario_05_unknown_spec_variant_fails_and_does_not_register_variant() {
    let host = setup_minimal_host();
    activate_host(&host);

    let err = run_command(&host, "ws.current.unknown-spec", None).unwrap_err();
    assert_eq!(
        err,
        "spec `unknown-spec` was not found in this project (no readable .kiro/specs/unknown-spec/spec.json)"
    );

    let registered = host.registered();
    assert!(!registered.contains_key("ws.current.unknown-spec"));
}

#[test]
fn scenario_06_listing_with_insert_produces_one_insert_and_one_notification() {
    let host = setup_minimal_host();
    activate_host(&host);

    let out = run_command(&host, "ws.specs", Some("insert"))
        .expect("must succeed")
        .expect("return text");

    let notifs = host.notifications();
    assert_eq!(notifs.len(), 1);
    assert_eq!(notifs[0].1, "Kiro: 2 specs known");

    let inserts: Vec<String> = host
        .events()
        .into_iter()
        .filter_map(|e| match e {
            HostEvent::Insert(txt) => Some(txt),
            _ => None,
        })
        .collect();

    assert_eq!(inserts.len(), 1);
    assert_eq!(inserts[0], out);
}

#[test]
fn scenario_07_insertion_on_non_insert_capable_command_fails() {
    let mut host = setup_minimal_host();
    activate_host(&host);
    host.active = Some("C:\\proj\\.kiro\\specs\\demo\\design.md".to_owned());

    let err = run_command(&host, "ws.current", Some("insert")).unwrap_err();
    assert_eq!(err, "`ws.current` does not support insertion");
}

#[test]
fn scenario_08_spec_named_twice_fails() {
    let host = setup_minimal_host();
    activate_host(&host);

    let err = run_command(&host, "ws.current.demo", Some("old")).unwrap_err();
    assert_eq!(
        err,
        "spec named twice: `demo` (command) and `old` (argument)"
    );
}

#[test]
fn scenario_09_invalid_spec_name_causes_no_read() {
    let host = setup_minimal_host();
    activate_host(&host);

    let reads_before = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(_)))
        .count();

    let err = run_command(&host, "ws.current", Some("invalid/name")).unwrap_err();
    assert!(err.contains("invalid spec name `invalid/name`"));

    let reads_after = host
        .events()
        .into_iter()
        .filter(|e| matches!(e, HostEvent::Read(_)))
        .count();

    assert_eq!(reads_before, reads_after, "no read should be performed for invalid spec");
}

#[test]
fn scenario_10_group_without_module_fails() {
    let host = setup_minimal_host();
    activate_host(&host);

    let err = run_command_with(&host, &[], "ws.specs", None).unwrap_err();
    assert_eq!(err, "`ws` commands are not available in this build");
}

#[test]
fn scenario_11_clipping_budget_and_trailers() {
    struct LongOutputModule;

    static LONG_COMMANDS: [CommandSpec; 2] = [
        CommandSpec {
            group: Group::Ws,
            name: "long-insert",
            title: "Kiro: Long Insert",
            spec_addressable: false,
            insert_capable: true,
            positional: Positional::PassThrough,
        },
        CommandSpec {
            group: Group::Ws,
            name: "long-no-insert",
            title: "Kiro: Long No Insert",
            spec_addressable: false,
            insert_capable: false,
            positional: Positional::PassThrough,
        },
    ];

    impl FeatureModule for LongOutputModule {
        fn group(&self) -> Group {
            Group::Ws
        }

        fn commands(&self) -> &'static [CommandSpec] {
            &LONG_COMMANDS
        }

        fn run(
            &self,
            _ctx: &mut openidea_plugin_kiro::context::CommandContext<'_>,
            cmd: &CommandSpec,
            _req: &openidea_plugin_kiro::args::Request,
        ) -> Result<Output, String> {
            let line = "A".repeat(80);
            let text = format!("{line}\n").repeat(350); // ~28 KiB
            Ok(Output::info(cmd.title, text))
        }
    }

    let host = setup_minimal_host();
    activate_host(&host);

    let mods: Vec<Box<dyn FeatureModule>> = vec![Box::new(LongOutputModule)];

    // 1. Insert-capable command clipped with variant trailer
    let out1 = run_command_with(&host, &mods, "ws.long-insert", None)
        .unwrap()
        .unwrap();
    assert!(out1.len() >= 20 * 1024, "return value must be complete");

    let notifs = host.notifications();
    let notif1 = notifs.last().unwrap();
    assert!(notif1.2.len() <= 16 * 1024);
    assert!(
        notif1.2.contains("— run ws.long-insert.insert for the full text"),
        "expected variant trailer in notification"
    );

    // 2. Non-insert-capable command clipped with plain trailer
    let out2 = run_command_with(&host, &mods, "ws.long-no-insert", None)
        .unwrap()
        .unwrap();
    assert!(out2.len() >= 20 * 1024, "return value must be complete");

    let notifs2 = host.notifications();
    let notif2 = notifs2.last().unwrap();
    assert!(notif2.2.len() <= 16 * 1024);
    assert!(notif2.2.contains("more lines"));
    assert!(
        !notif2.2.contains("for the full text"),
        "plain trailer should not contain run hint"
    );
}

#[test]
fn scenario_12_each_path_read_at_most_once_per_command() {
    let host = setup_minimal_host();
    activate_host(&host);

    let events_before = host.events().len();

    let _ = run_command(&host, "ws.specs", None).unwrap().unwrap();

    let mut reads: Vec<String> = Vec::new();
    for ev in &host.events()[events_before..] {
        if let HostEvent::Read(path) = ev {
            reads.push(path.clone());
        }
    }

    let mut seen = std::collections::BTreeSet::new();
    for path in reads {
        assert!(
            seen.insert(path.clone()),
            "path '{path}' was read more than once during command execution"
        );
    }
}

#[test]
fn scenario_13_no_project_open_without_root() {
    let host = FakeHost::new(); // root is None
    activate_host(&host);

    let err = run_command(&host, "ws.specs", None).unwrap_err();
    assert_eq!(err, "no project is open");
}
