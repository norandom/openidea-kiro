//! Workspace commands: `ws.specs` and `ws.current` (task 3.3).

use crate::args::Request;
use crate::context::{CommandContext, KnownVia, RoadmapState};
use crate::host::Level;
use crate::module::FeatureModule;
use crate::present::Output;
use crate::registry::{CommandSpec, Group, Positional};
use kiro_core::finding::Severity;
use kiro_core::resolve::ResolvedVia;
use kiro_core::spec::{Doc, phase_label};
use kiro_core::table;

pub use kiro_core::spec::phase_label as exported_phase_label;

pub struct Ws;

pub static COMMANDS: [CommandSpec; 2] = [
    CommandSpec {
        group: Group::Ws,
        name: "specs",
        title: "Kiro: List Specs",
        spec_addressable: false,
        insert_capable: true,
        positional: Positional::PassThrough,
    },
    CommandSpec {
        group: Group::Ws,
        name: "current",
        title: "Kiro: Current Spec",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::Spec,
    },
];

impl FeatureModule for Ws {
    fn group(&self) -> Group {
        Group::Ws
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
            "specs" => specs(ctx, req),
            "current" => current(ctx, req),
            _ => Err(format!("unknown command `{}`", cmd.name)),
        }
    }
}

/// Executes `ws.specs`: lists known specs, sources, readability, phase, and per-spec command status.
pub fn specs(ctx: &mut CommandContext<'_>, _req: &Request) -> Result<Output, String> {
    if ctx.known.is_empty() {
        return Ok(Output {
            title: "Kiro: 0 specs known".to_owned(),
            level: Level::Warning,
            text: "No spec is known. Looked for .kiro/steering/roadmap.md and an active file under .kiro/specs/<spec>/.".to_owned(),
        });
    }

    let roadmap_count = ctx
        .known
        .iter()
        .filter(|(_, vias)| vias.contains(&KnownVia::Roadmap))
        .count();
    let active_count = ctx
        .known
        .iter()
        .filter(|(_, vias)| vias.contains(&KnownVia::ActiveFile))
        .count();
    let arg_count = ctx
        .known
        .iter()
        .filter(|(_, vias)| vias.contains(&KnownVia::Argument))
        .count();

    let line1 = format!(
        "Known specs: {} (roadmap: {roadmap_count}, active file: {active_count}, argument: {arg_count})",
        ctx.known.len()
    );

    let line2 = match ctx.roadmap {
        RoadmapState::Present { entries } => {
            format!("Roadmap: .kiro/steering/roadmap.md ({entries} entries)")
        }
        RoadmapState::Absent => "Roadmap: .kiro/steering/roadmap.md not found".to_owned(),
        RoadmapState::NoSpecsSection => {
            "Roadmap: .kiro/steering/roadmap.md has no \"## Specs (dependency order)\" section"
                .to_owned()
        }
    };

    let mut rows: Vec<Vec<String>> = Vec::new();
    let known_snapshot = ctx.known.clone();
    for (name, vias) in &known_snapshot {
        let sources_str = vias
            .iter()
            .map(|v| match v {
                KnownVia::Roadmap => "roadmap",
                KnownVia::ActiveFile => "active file",
                KnownVia::Argument => "argument",
            })
            .collect::<Vec<_>>()
            .join(", ");

        let feat = ctx.load(name);
        let spec_json_str = if feat.meta.is_some() {
            "readable"
        } else {
            "missing"
        };
        let phase = phase_label(feat.meta.as_ref());

        rows.push(vec![
            name.clone(),
            sources_str,
            spec_json_str.to_owned(),
            phase,
        ]);
    }

    let table_text = table(&["Spec", "Source", "spec.json", "Phase"], &rows);

    let mut per_spec = format!(
        "Per-spec commands: {} registered, {} unchanged",
        ctx.sync.registered, ctx.sync.unchanged
    );
    if !ctx.sync.skipped_budget.is_empty() {
        per_spec.push_str(&format!(
            "; skipped (command limit): {}",
            ctx.sync.skipped_budget.join(", ")
        ));
    }
    if !ctx.sync.skipped_invalid.is_empty() {
        per_spec.push_str(&format!(
            "; skipped (invalid name): {}",
            ctx.sync.skipped_invalid.join(", ")
        ));
    }

    let text = format!("{line1}\n{line2}\n\n{table_text}\n\n{per_spec}");
    let title = format!("Kiro: {} specs known", ctx.known.len());

    Ok(Output::info(title, text))
}

/// Executes `ws.current`: reports the current spec, resolution source, documents, and findings.
pub fn current(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let via_str = match resolved.via {
        ResolvedVia::Explicit => "named explicitly",
        ResolvedVia::ActiveFile => "from the active file",
        ResolvedVia::OnlyKnown => "the only known spec",
    };
    let spec_line = format!("Spec: {} ({via_str})", resolved.name);

    let feat = ctx.load(&resolved.name);
    let phase = phase_label(feat.meta.as_ref());
    let phase_line = format!("Phase: {phase}");

    let doc_status = |readable: bool| {
        if readable { "present" } else { "missing" }
    };
    let docs_line = format!(
        "Documents: spec.json {}, requirements.md {}, design.md {}, tasks.md {}",
        doc_status(CommandContext::is_readable(feat, Doc::SpecJson)),
        doc_status(CommandContext::is_readable(feat, Doc::Requirements)),
        doc_status(CommandContext::is_readable(feat, Doc::Design)),
        doc_status(CommandContext::is_readable(feat, Doc::Tasks)),
    );

    let errors = feat
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = feat
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let infos = feat
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let findings_line = format!("Findings: {errors} errors, {warnings} warnings, {infos} info");

    let intro = format!("{spec_line}\n{phase_line}\n{docs_line}\n{findings_line}");
    let title = format!("Kiro: {} ({phase})", resolved.name);

    Ok(Output::with_findings(title, &intro, &feat.findings))
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
    use crate::commands::{activate, run_command};
    use crate::host::FakeHost;

    #[test]
    fn run_ws_specs_and_current_over_fake_host_with_two_specs() {
        let rm = "# Roadmap\n## Specs (dependency order)\n- [ ] spec-a\n- [ ] spec-b\n";
        let host = FakeHost::new()
            .with_root("C:\\proj")
            .with_file(".kiro/steering/roadmap.md", rm)
            .with_file(
                ".kiro/specs/spec-a/spec.json",
                r#"{"name": "spec-a", "phase": "tasks"}"#,
            )
            .with_file(
                ".kiro/specs/spec-a/requirements.md",
                "# Requirements\nContent",
            )
            .with_file(
                ".kiro/specs/spec-b/spec.json",
                r#"{"name": "spec-b", "phase": "design"}"#,
            );

        // Activate plugin
        activate(
            &host,
            &[
                "project-read".to_owned(),
                "editor-read".to_owned(),
                "editor-write".to_owned(),
            ],
        )
        .unwrap();

        // 1. Run ws.specs
        let specs_out = run_command(&host, "ws.specs", None).unwrap().unwrap();
        assert!(specs_out.starts_with("Known specs: 2 (roadmap: 2, active file: 0, argument: 0)"));
        assert!(specs_out.contains("Roadmap: .kiro/steering/roadmap.md (2 entries)"));
        assert!(specs_out.contains("spec-a"));
        assert!(specs_out.contains("spec-b"));
        assert!(specs_out.contains("Per-spec commands: 20 registered, 0 unchanged"));

        // Per-spec variants for ws.current, nav.*, and vocab.* should be registered
        let registered = host.registered();
        assert!(registered.contains_key("ws.current.spec-a"));
        assert!(registered.contains_key("ws.current.spec-b"));
        assert!(registered.contains_key("nav.context.spec-a"));
        assert!(registered.contains_key("nav.files.spec-a"));
        assert!(registered.contains_key("vocab.term.spec-a"));
        assert!(registered.contains_key("vocab.list.spec-a"));
        assert!(registered.contains_key("vocab.topics.spec-a"));

        // 2. Run ws.current with explicit spec
        let current_out = run_command(&host, "ws.current", Some("spec-a"))
            .unwrap()
            .unwrap();
        assert!(current_out.contains("Spec: spec-a (named explicitly)"));
        assert!(current_out.contains("Phase: tasks"));
        assert!(current_out.contains(
            "Documents: spec.json present, requirements.md present, design.md missing, tasks.md missing"
        ));
        assert!(current_out.contains("Findings:"));

        // Notifications
        let notifs = host.notifications();
        assert!(
            notifs
                .iter()
                .any(|(_, title, _)| title == "Kiro: 2 specs known")
        );
        assert!(
            notifs
                .iter()
                .any(|(_, title, _)| title == "Kiro: spec-a (tasks)")
        );
    }

    #[test]
    fn run_ws_specs_when_no_specs_known_warns() {
        let host = FakeHost::new().with_root("C:\\proj");
        activate(
            &host,
            &[
                "project-read".to_owned(),
                "editor-read".to_owned(),
                "editor-write".to_owned(),
            ],
        )
        .unwrap();

        let specs_out = run_command(&host, "ws.specs", None).unwrap().unwrap();
        assert!(specs_out.contains("No spec is known"));

        let notifs = host.notifications();
        let last_notif = notifs.last().unwrap();
        assert_eq!(last_notif.0, Level::Warning);
        assert_eq!(last_notif.1, "Kiro: 0 specs known");
    }
}
