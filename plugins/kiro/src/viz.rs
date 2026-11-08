//! Visualization commands: `viz.status`, `viz.graph`, and `viz.tasks` (task 3.1).

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::Level;
use crate::module::FeatureModule;
use crate::present::{Output, level_of};
use crate::registry::{CommandSpec, Group, Positional};
use kiro_core::spec::Phase;
use kiro_core::viz::graph::{graph_summary, graph_view};
use kiro_core::viz::status::{status_text, status_view};
use kiro_core::viz::tasks::{tasks_summary, tasks_view};
use kiro_core::viz::{CAPS, Format, Labeler, RawLabeler, TEXT_OPTION, compose, render_body};

pub struct Viz;

pub static COMMANDS: [CommandSpec; 3] = [
    CommandSpec {
        group: Group::Viz,
        name: "status",
        title: "Kiro: Spec Status",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Viz,
        name: "graph",
        title: "Kiro: Spec Graph",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::Spec,
    },
    CommandSpec {
        group: Group::Viz,
        name: "tasks",
        title: "Kiro: Task Graph",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::Spec,
    },
];

impl FeatureModule for Viz {
    fn group(&self) -> Group {
        Group::Viz
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
        let opts = parse_options(cmd, &req.rest)?;
        match cmd.name {
            "status" => status(ctx, req),
            "graph" => graph(ctx, req, opts),
            "tasks" => tasks(ctx, req, opts),
            _ => Err(format!("unknown command `{}`", cmd.name)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub format: Format,
}

/// Parses options for visualization commands.
///
/// `viz.status` accepts no options. `viz.graph` and `viz.tasks` accept `text` (repeated tokens are accepted as the same option).
/// Unknown options return an error without reading files.
pub fn parse_options(cmd: &CommandSpec, rest: &[String]) -> Result<Options, String> {
    if cmd.name == "status" {
        if let Some(tok) = rest.first() {
            return Err(format!(
                "unknown option `{tok}` for viz.status (accepted: none)"
            ));
        }
        return Ok(Options {
            format: Format::Mermaid,
        });
    }

    let mut format = Format::Mermaid;
    for tok in rest {
        if tok == TEXT_OPTION {
            format = Format::Table;
        } else {
            return Err(format!(
                "unknown option `{tok}` for viz.{} (accepted: text)",
                cmd.name
            ));
        }
    }
    Ok(Options { format })
}

/// Preferred labeler for visualization commands.
///
/// Currently returns `RawLabeler`. This is the single substitution point for a vocabulary-backed labeler.
#[must_use]
pub fn labeler(_ctx: &CommandContext<'_>) -> impl Labeler + use<> {
    RawLabeler
}

/// Generates the title for `viz.status`.
#[must_use]
pub fn title_status(spec: &str, phase: &str) -> String {
    format!("Kiro: {spec} status ({phase})")
}

/// Generates the title for `viz.graph`.
#[must_use]
pub fn title_graph(spec: &str, drawn_nodes: usize) -> String {
    format!("Kiro: {spec} graph ({drawn_nodes} nodes)")
}

/// Generates the title for `viz.tasks`.
#[must_use]
pub fn title_tasks(spec: &str, checked: usize, total: usize) -> String {
    format!("Kiro: {spec} tasks ({checked}/{total} done)")
}

/// Determines notification level for graph and tasks views.
///
/// Warning when missing documents, unresolved references / unknown dependencies, or truncation; else Info.
#[must_use]
pub fn level_for(missing: bool, unresolved: usize, truncated: bool) -> Level {
    if missing || unresolved > 0 || truncated {
        Level::Warning
    } else {
        Level::Info
    }
}

fn status(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let name = resolved.name;
    let superseded = ctx.load(&name).phase() == Phase::Superseded;
    let chain = if superseded {
        Some(ctx.workspace.successor_chain(&name))
    } else {
        None
    };
    let feature = ctx.load(&name);
    let view = status_view(feature, chain.as_ref());
    Ok(Output {
        title: title_status(&name, &view.phase),
        level: level_of(&feature.findings),
        text: status_text(&view),
    })
}

fn graph(ctx: &mut CommandContext<'_>, req: &Request, opts: Options) -> Result<Output, String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let name = resolved.name;
    let lab = labeler(ctx);
    let feature = ctx.load(&name);
    let view = graph_view(feature, &lab).ok_or_else(|| {
        format!("spec `{name}` has no readable requirements.md, design.md or tasks.md")
    })?;
    let body = render_body(&view.graph, opts.format, &CAPS);
    let summary = graph_summary(&view, &body);
    let text = compose(&summary, &body.text);
    let drawn_nodes = view.graph.nodes.len().saturating_sub(body.omitted_nodes);
    let level = level_for(
        !view.stats.missing.is_empty(),
        view.stats.unresolved_refs,
        body.truncated,
    );
    Ok(Output {
        title: title_graph(&name, drawn_nodes),
        level,
        text,
    })
}

fn tasks(ctx: &mut CommandContext<'_>, req: &Request, opts: Options) -> Result<Output, String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let name = resolved.name;
    let feature = ctx.load(&name);
    let tasks_doc = feature
        .tasks
        .as_ref()
        .ok_or_else(|| format!("spec `{name}` has no readable .kiro/specs/{name}/tasks.md"))?;
    let view = tasks_view(&name, tasks_doc);
    let body = render_body(&view.graph, opts.format, &CAPS);
    let summary = tasks_summary(&view, &body);
    let text = compose(&summary, &body.text);
    let level = level_for(false, view.stats.unknown_depends, body.truncated);
    Ok(Output {
        title: title_tasks(&name, view.stats.checked, view.stats.total),
        level,
        text,
    })
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

    #[test]
    fn parse_options_matrix() {
        let status_cmd = &COMMANDS[0];
        let graph_cmd = &COMMANDS[1];
        let tasks_cmd = &COMMANDS[2];

        // status accepts no options
        assert_eq!(
            parse_options(status_cmd, &[]),
            Ok(Options {
                format: Format::Mermaid
            })
        );
        assert_eq!(
            parse_options(status_cmd, &[String::from("text")]),
            Err(String::from(
                "unknown option `text` for viz.status (accepted: none)"
            ))
        );
        assert_eq!(
            parse_options(status_cmd, &[String::from("any")]),
            Err(String::from(
                "unknown option `any` for viz.status (accepted: none)"
            ))
        );

        // graph options
        assert_eq!(
            parse_options(graph_cmd, &[]),
            Ok(Options {
                format: Format::Mermaid
            })
        );
        assert_eq!(
            parse_options(graph_cmd, &[String::from("text")]),
            Ok(Options {
                format: Format::Table
            })
        );
        assert_eq!(
            parse_options(graph_cmd, &[String::from("text"), String::from("text")]),
            Ok(Options {
                format: Format::Table
            })
        );
        assert_eq!(
            parse_options(graph_cmd, &[String::from("txt")]),
            Err(String::from(
                "unknown option `txt` for viz.graph (accepted: text)"
            ))
        );

        // tasks options
        assert_eq!(
            parse_options(tasks_cmd, &[]),
            Ok(Options {
                format: Format::Mermaid
            })
        );
        assert_eq!(
            parse_options(tasks_cmd, &[String::from("text")]),
            Ok(Options {
                format: Format::Table
            })
        );
        assert_eq!(
            parse_options(tasks_cmd, &[String::from("unknown")]),
            Err(String::from(
                "unknown option `unknown` for viz.tasks (accepted: text)"
            ))
        );
    }

    #[test]
    fn titles_match_spec_format() {
        assert_eq!(title_status("demo", "draft"), "Kiro: demo status (draft)");
        assert_eq!(title_graph("demo", 12), "Kiro: demo graph (12 nodes)");
        assert_eq!(title_tasks("demo", 5, 10), "Kiro: demo tasks (5/10 done)");
    }

    #[test]
    fn level_for_rules() {
        assert_eq!(level_for(false, 0, false), Level::Info);
        assert_eq!(level_for(true, 0, false), Level::Warning);
        assert_eq!(level_for(false, 1, false), Level::Warning);
        assert_eq!(level_for(false, 0, true), Level::Warning);
        assert_eq!(level_for(true, 3, true), Level::Warning);
    }
}
