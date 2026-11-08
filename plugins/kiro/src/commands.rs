//! Command dispatch, activation, deactivation, and module table (task 3.2).

use std::sync::Mutex;

use crate::args;
use crate::context::CommandContext;
use crate::host::{HostApi, LogLevel, REQUIRED_PERMISSIONS};
use crate::module::FeatureModule;
use crate::present;
use crate::registry::{CommandSpec, Group, parse_id, registry, static_commands, sync_variants};

static GRANTED_PERMISSIONS: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// The module table in canonical order `ws, nav, viz, vocab, check`.
///
/// plugin-core ships `[Box::new(crate::ws::Ws)]`; each sibling spec inserts its module at its canonical position.
#[must_use]
pub fn modules() -> Vec<Box<dyn FeatureModule>> {
    vec![
        Box::new(crate::ws::Ws),
        Box::new(crate::nav::Nav),
        Box::new(crate::vocab::Vocab),
    ]
}

/// Concatenation of all base command specifications across modules.
#[must_use]
pub fn all_commands() -> Vec<CommandSpec> {
    all_commands_from(&modules())
}

/// Concatenates base command specifications from an explicit slice of modules.
#[must_use]
pub fn all_commands_from(modules: &[Box<dyn FeatureModule>]) -> Vec<CommandSpec> {
    let mut cmds = Vec::new();
    for m in modules {
        cmds.extend_from_slice(m.commands());
    }
    cmds
}

/// Returns the granted permissions saved during activation.
#[must_use]
pub fn granted_permissions() -> Vec<String> {
    GRANTED_PERMISSIONS
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

/// Activates the plugin: verifies required permissions, registers static commands, and logs.
pub fn activate(host: &dyn HostApi, granted: &[String]) -> Result<(), String> {
    let missing: Vec<&str> = REQUIRED_PERMISSIONS
        .iter()
        .copied()
        .filter(|req| !granted.iter().any(|g| g == req))
        .collect();
    if !missing.is_empty() {
        return Err(present::error_line(&format!(
            "missing permissions: {} (declare them in plugin.toml)",
            missing.join(", ")
        )));
    }

    if let Ok(mut g) = GRANTED_PERMISSIONS.lock() {
        *g = granted.to_vec();
    } else if let Err(p) = GRANTED_PERMISSIONS.lock() {
        *p.into_inner() = granted.to_vec();
    }

    let static_cmds = static_commands(&all_commands());
    let count = static_cmds.len();
    let mut reg = registry();
    for decl in &static_cmds {
        if let Err(e) = reg.register(host, decl) {
            return Err(present::error_line(&e.to_string()));
        }
    }

    host.log(LogLevel::Info, &format!("kiro activated: {count} commands"));
    Ok(())
}

/// Deactivates the plugin: logs info and performs no other host call.
pub fn deactivate(host: &dyn HostApi) {
    host.log(LogLevel::Info, "kiro deactivated");
}

/// Dispatches a command execution against the default module table.
pub fn run_command(
    host: &dyn HostApi,
    id: &str,
    arg: Option<&str>,
) -> Result<Option<String>, String> {
    run_command_with(host, &modules(), id, arg)
}

/// Dispatches a command execution against an explicit module table (test seam).
pub fn run_command_with(
    host: &dyn HostApi,
    modules: &[Box<dyn FeatureModule>],
    id: &str,
    arg: Option<&str>,
) -> Result<Option<String>, String> {
    let parsed = match parse_id(id) {
        Some(p) => p,
        None => return Err(present::error_line(&format!("unknown command `{id}`"))),
    };

    let (group_prefix, cmd_name) = match parsed.base.split_once('.') {
        Some(pair) => pair,
        None => return Err(present::error_line(&format!("unknown command `{id}`"))),
    };

    let group = match Group::parse(group_prefix) {
        Some(g) => g,
        None => return Err(present::error_line(&format!("unknown command `{id}`"))),
    };

    let module = match modules.iter().find(|m| m.group() == group) {
        Some(m) => m,
        None => {
            return Err(present::error_line(&format!(
                "`{group_prefix}` commands are not available in this build"
            )));
        }
    };

    let cmd = match module.commands().iter().find(|c| c.name == cmd_name) {
        Some(c) => c,
        None => return Err(present::error_line(&format!("unknown command `{id}`"))),
    };

    if parsed.spec.is_some() && !cmd.spec_addressable {
        return Err(present::error_line(&format!("unknown command `{id}`")));
    }

    let req = match args::parse(cmd, &parsed, arg) {
        Ok(r) => r,
        Err(e) => return Err(present::error_line(&e)),
    };

    let mut ctx = match CommandContext::new(host, req.spec.as_deref()) {
        Ok(c) => c,
        Err(e) => return Err(present::error_line(&e)),
    };

    let all_cmds = all_commands_from(modules);

    if let Some(explicit_spec) = req.spec.as_deref() {
        let feature = ctx.load(explicit_spec);
        if feature.meta.is_none() {
            let valid_names: Vec<String> = ctx
                .known
                .iter()
                .filter(|(n, _)| n != explicit_spec)
                .map(|(n, _)| n.clone())
                .collect();
            let sync_report = sync_variants(host, &all_cmds, &valid_names);
            for skipped in &sync_report.skipped_budget {
                host.log(
                    LogLevel::Warn,
                    &format!("skipped variant for {skipped}: command budget reached"),
                );
            }
            for skipped in &sync_report.skipped_invalid {
                host.log(
                    LogLevel::Warn,
                    &format!("skipped variant for {skipped}: invalid spec segment"),
                );
            }
            return Err(present::error_line(&format!(
                "spec `{explicit_spec}` was not found in this project (no readable .kiro/specs/{explicit_spec}/spec.json)"
            )));
        }
    }

    let sync_report = sync_variants(host, &all_cmds, &ctx.known_names());
    for skipped in &sync_report.skipped_budget {
        host.log(
            LogLevel::Warn,
            &format!("skipped variant for {skipped}: command budget reached"),
        );
    }
    for skipped in &sync_report.skipped_invalid {
        host.log(
            LogLevel::Warn,
            &format!("skipped variant for {skipped}: invalid spec segment"),
        );
    }
    ctx.sync = sync_report;

    let out = match module.run(&mut ctx, cmd, &req) {
        Ok(o) => o,
        Err(e) => return Err(present::error_line(&e)),
    };

    let full_text = present::deliver(host, &out, req.insert, cmd.insert_id().as_deref());
    Ok(Some(full_text))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]
mod tests {
    use super::*;
    use crate::args::Request;
    use crate::host::{FakeHost, HostEvent};
    use crate::present::Output;
    use crate::registry::{Group, Positional};

    struct TestModule {
        commands: &'static [CommandSpec],
    }

    static TEST_COMMANDS: [CommandSpec; 2] = [
        CommandSpec {
            group: Group::Ws,
            name: "list",
            title: "Kiro: List",
            spec_addressable: false,
            insert_capable: true,
            positional: Positional::PassThrough,
        },
        CommandSpec {
            group: Group::Ws,
            name: "current",
            title: "Kiro: Current",
            spec_addressable: true,
            insert_capable: false,
            positional: Positional::Spec,
        },
    ];

    impl FeatureModule for TestModule {
        fn group(&self) -> Group {
            Group::Ws
        }

        fn commands(&self) -> &'static [CommandSpec] {
            self.commands
        }

        fn run(
            &self,
            _ctx: &mut CommandContext<'_>,
            cmd: &CommandSpec,
            req: &Request,
        ) -> Result<Output, String> {
            Ok(Output::info(
                cmd.title,
                format!("Ran {} with spec {:?}", cmd.name, req.spec),
            ))
        }
    }

    #[test]
    fn activate_validates_permissions_and_registers_static_list() {
        let host = FakeHost::new();

        // 1. None granted
        let err1 = activate(&host, &[]).unwrap_err();
        assert_eq!(
            err1,
            "missing permissions: project-read, editor-read, editor-write (declare them in plugin.toml)"
        );

        // 2. Partial granted
        let err2 = activate(&host, &["project-read".to_owned()]).unwrap_err();
        assert_eq!(
            err2,
            "missing permissions: editor-read, editor-write (declare them in plugin.toml)"
        );

        // 3. All granted
        let ok = activate(
            &host,
            &[
                "project-read".to_owned(),
                "editor-read".to_owned(),
                "editor-write".to_owned(),
            ],
        );
        assert!(ok.is_ok());

        assert_eq!(
            granted_permissions(),
            vec![
                "project-read".to_owned(),
                "editor-read".to_owned(),
                "editor-write".to_owned(),
            ]
        );

        let logs: Vec<String> = host
            .events()
            .into_iter()
            .filter_map(|e| match e {
                HostEvent::Log(LogLevel::Info, msg) => Some(msg),
                _ => None,
            })
            .collect();
        assert_eq!(logs, vec!["kiro activated: 14 commands"]);
    }

    #[test]
    fn deactivate_only_logs_and_performs_no_other_host_call() {
        let host = FakeHost::new();
        deactivate(&host);

        let events = host.events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            HostEvent::Log(LogLevel::Info, "kiro deactivated".to_owned())
        );
    }

    #[test]
    fn run_command_with_unknown_id_and_group_without_module() {
        let host = FakeHost::new().with_root("C:\\proj");

        // Completely invalid id
        let err1 = run_command_with(&host, &[], "not-an-id", None).unwrap_err();
        assert_eq!(err1, "unknown command `not-an-id`");

        // Group not in module table
        let err2 = run_command_with(&host, &[], "ws.list", None).unwrap_err();
        assert_eq!(err2, "`ws` commands are not available in this build");

        // Module table present, but group nav not in table
        let mods: Vec<Box<dyn FeatureModule>> = vec![Box::new(TestModule {
            commands: &TEST_COMMANDS,
        })];
        let err3 = run_command_with(&host, &mods, "nav.next", None).unwrap_err();
        assert_eq!(err3, "`nav` commands are not available in this build");

        // Module in table, but command unknown
        let err4 = run_command_with(&host, &mods, "ws.unknown", None).unwrap_err();
        assert_eq!(err4, "unknown command `ws.unknown`");

        // Per-spec variant on non-spec-addressable command
        let err5 = run_command_with(&host, &mods, "ws.list.spec-a", None).unwrap_err();
        assert_eq!(err5, "unknown command `ws.list.spec-a`");
    }

    #[test]
    fn run_command_with_unreadable_explicit_spec_syncs_other_variants_and_fails() {
        let rm = "# Roadmap\n## Specs (dependency order)\n- [ ] valid-spec\n";
        let host = FakeHost::new()
            .with_root("C:\\proj")
            .with_file(".kiro/steering/roadmap.md", rm);

        let mods: Vec<Box<dyn FeatureModule>> = vec![Box::new(TestModule {
            commands: &TEST_COMMANDS,
        })];

        // Call ws.current with explicit unreadable-spec
        let err = run_command_with(&host, &mods, "ws.current", Some("missing-spec")).unwrap_err();
        assert_eq!(
            err,
            "spec `missing-spec` was not found in this project (no readable .kiro/specs/missing-spec/spec.json)"
        );

        // Check that valid-spec variant was synchronised on host, but missing-spec was not
        let registered = host.registered();
        assert!(registered.contains_key("ws.current.valid-spec"));
        assert!(!registered.contains_key("ws.current.missing-spec"));

        // No notification was shown for the error
        assert_eq!(host.notifications().len(), 0);
    }
}
