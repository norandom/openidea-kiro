//! Navigation commands and view dispatch (`nav.*`, task 1.1).

pub mod comment;
pub mod context;
pub mod docs;
pub mod files;

use crate::args::Request;
use crate::context::CommandContext;
use crate::module::FeatureModule;
use crate::present::Output;
use crate::registry::{CommandSpec, Group, Positional};
use kiro_core::spec::Doc;

pub struct Nav;

pub static COMMANDS: [CommandSpec; 6] = [
    CommandSpec {
        group: Group::Nav,
        name: "context",
        title: "Kiro: Spec Context",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Nav,
        name: "files",
        title: "Kiro: Files by Component",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Nav,
        name: "task-files",
        title: "Kiro: Files by Task",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Nav,
        name: "to-requirements",
        title: "Kiro: Requirements Outline",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Nav,
        name: "to-design",
        title: "Kiro: Design Outline",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Nav,
        name: "to-tasks",
        title: "Kiro: Tasks Outline",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
];

/// Total static commands derived from `COMMANDS` (6 base + 1 insert variant).
pub const STATIC_IDS: usize = 7;
/// Number of spec-addressable commands generating per-spec variants.
pub const IDS_PER_SPEC: usize = 6;

impl FeatureModule for Nav {
    fn group(&self) -> Group {
        Group::Nav
    }

    fn commands(&self) -> &'static [CommandSpec] {
        &COMMANDS
    }

    fn run(
        &self,
        ctx: &mut CommandContext<'_>,
        cmd: &CommandSpec,
        req: &Request,
    ) -> Result<Output, String> {
        match cmd.name {
            "context" => context::run(ctx, req),
            "files" => files::by_component(ctx, req),
            "task-files" => files::by_task(ctx, req),
            "to-requirements" => docs::run(ctx, req, Doc::Requirements),
            "to-design" => docs::run(ctx, req, Doc::Design),
            "to-tasks" => docs::run(ctx, req, Doc::Tasks),
            _ => Err(format!("unknown nav command `{}`", cmd.name)),
        }
    }
}

/// Rejects extra positional arguments beyond `allowed`.
#[allow(dead_code)]
pub(crate) fn reject_extra(cmd: &CommandSpec, req: &Request, allowed: usize) -> Result<(), String> {
    if req.rest.len() > allowed {
        let extra = req.rest.get(allowed).map(String::as_str).unwrap_or("");
        return Err(format!("`{}` does not take `{}`", cmd.id(), extra));
    }
    Ok(())
}

/// Formats a boolean completion state as a Markdown checkbox.
#[allow(dead_code)]
#[must_use]
pub(crate) fn checkbox(checked: bool) -> &'static str {
    if checked { "[x]" } else { "[ ]" }
}

/// Formats a `TaskRef` into standard navigation task line shape:
/// `<id> <checkbox> <description> (<tasks_path>:<line>)`.
#[allow(dead_code)]
#[must_use]
pub(crate) fn task_ref_line(t: &kiro_core::join::TaskRef, tasks_path: &str) -> String {
    format!(
        "{} {} {} ({tasks_path}:{})",
        t.id,
        checkbox(t.checked),
        t.description,
        t.line
    )
}

/// Formats a `Task` into standard navigation task line shape:
/// `<id> <checkbox> <description> (<tasks_path>:<line>)`.
#[allow(dead_code)]
#[must_use]
pub(crate) fn task_line(t: &kiro_core::tasks::Task, tasks_path: &str) -> String {
    format!(
        "{} {} {} ({tasks_path}:{})",
        t.id,
        checkbox(t.checked),
        t.description,
        t.line
    )
}

/// Extracts the file name component after the last forward slash.
#[allow(dead_code)]
#[must_use]
pub(crate) fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
    use crate::registry::static_commands;
    use kiro_core::join::TaskRef;
    use kiro_core::tasks::Task;

    #[test]
    fn commands_validate() {
        for cmd in &COMMANDS {
            assert!(
                cmd.validate().is_ok(),
                "command `{}` failed validation",
                cmd.name
            );
        }
    }

    #[test]
    fn static_ids_and_per_spec_counts() {
        let static_list = static_commands(&COMMANDS);
        assert_eq!(
            static_list.len(),
            STATIC_IDS,
            "static_commands must produce exactly 7 declarations"
        );
        let spec_addressable_count = COMMANDS.iter().filter(|c| c.spec_addressable).count();
        assert_eq!(
            spec_addressable_count, IDS_PER_SPEC,
            "exactly 6 commands must be spec-addressable"
        );
    }

    #[test]
    fn reject_extra_helper() {
        let cmd = &COMMANDS[1]; // nav.files
        let req_ok = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        assert!(reject_extra(cmd, &req_ok, 0).is_ok());

        let req_extra = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["unexpected".into()],
            raw: None,
        };
        let err = reject_extra(cmd, &req_extra, 0).unwrap_err();
        assert_eq!(err, "`nav.files` does not take `unexpected`");

        // Allowed 1
        assert!(reject_extra(cmd, &req_extra, 1).is_ok());
    }

    #[test]
    fn checkbox_helper() {
        assert_eq!(checkbox(true), "[x]");
        assert_eq!(checkbox(false), "[ ]");
    }

    #[test]
    fn task_ref_line_and_task_line_helpers() {
        let tref = TaskRef {
            id: "2.1".into(),
            description: "Build alpha".into(),
            line: 42,
            checked: true,
        };
        assert_eq!(
            task_ref_line(&tref, ".kiro/specs/demo/tasks.md"),
            "2.1 [x] Build alpha (.kiro/specs/demo/tasks.md:42)"
        );

        let task = Task {
            id: "1.2".into(),
            description: "Implement parser".into(),
            checked: false,
            line: 18,
            ..Default::default()
        };
        assert_eq!(
            task_line(&task, ".kiro/specs/alpha/tasks.md"),
            "1.2 [ ] Implement parser (.kiro/specs/alpha/tasks.md:18)"
        );
    }

    #[test]
    fn file_name_helper() {
        assert_eq!(file_name("src/lib.rs"), "lib.rs");
        assert_eq!(file_name("deep/nested/path/to/File.kt"), "File.kt");
        assert_eq!(file_name("Makefile"), "Makefile");
        assert_eq!(file_name(""), "");
    }
}
