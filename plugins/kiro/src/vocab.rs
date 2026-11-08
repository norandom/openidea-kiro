//! Vocabulary commands: `vocab.term`, `vocab.list`, and `vocab.topics` (task 4.1, 4.2).

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::{Level, LogLevel};
use crate::module::FeatureModule;
use crate::present::{Output, level_of};
use crate::registry::{CommandSpec, Group, Positional};
use kiro_core::finding::Finding;
use kiro_core::spec::Doc;
use kiro_core::table;
use kiro_core::vocab::{
    Hit, Kind, Lookup, MatchVia, Status, Term, Vocabulary, load, scope_findings, spec_topics,
};

pub struct Vocab;

pub static COMMANDS: [CommandSpec; 3] = [
    CommandSpec {
        group: Group::Vocab,
        name: "term",
        title: "Kiro: Vocabulary Term",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::PassThrough,
    },
    CommandSpec {
        group: Group::Vocab,
        name: "list",
        title: "Kiro: List Vocabulary Terms",
        spec_addressable: true,
        insert_capable: true,
        positional: Positional::PassThrough,
    },
    CommandSpec {
        group: Group::Vocab,
        name: "topics",
        title: "Kiro: Vocabulary Topics",
        spec_addressable: true,
        insert_capable: false,
        positional: Positional::PassThrough,
    },
];

impl FeatureModule for Vocab {
    fn group(&self) -> Group {
        Group::Vocab
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
            "term" => term(ctx, req),
            "list" => list(ctx, req),
            "topics" => topics(ctx, req),
            _ => Err(format!("unknown command `{}`", cmd.name)),
        }
    }
}

/// Per-call scope: resolved spec, its `spec.json` path, declared topics, loaded vocabulary,
/// and scope findings.
struct Scoped {
    spec: String,
    topics: Vec<String>,
    loaded: kiro_core::vocab::Loaded,
    scope_findings: Vec<Finding>,
}

fn scoped(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Scoped, String> {
    let resolved = ctx.resolve(req.spec.as_deref())?;
    let spec = resolved.name;
    let (spec_json, topics) = {
        let feature = ctx.load(&spec);
        (
            feature.path(Doc::SpecJson),
            spec_topics(feature.meta.as_ref()),
        )
    };

    let loaded = load(&mut ctx.workspace);
    let scope_findings = scope_findings(&spec, &spec_json, &topics, &loaded);

    ctx.host.log(
        LogLevel::Debug,
        &format!(
            "vocab: spec `{}`, {} topics, status {:?}",
            spec,
            topics.len(),
            loaded.status
        ),
    );

    Ok(Scoped {
        spec,
        topics,
        loaded,
        scope_findings,
    })
}

const DEFINITION_COLUMN_CHARS: usize = 72;

fn kind_text(kind: &Kind) -> String {
    match kind {
        Kind::Entity => "entity".to_string(),
        Kind::Bool => "bool".to_string(),
        Kind::Enum { values } => format!("enum: {}", values.join(", ")),
        Kind::Int { min, max } => match (min, max) {
            (Some(mn), Some(mx)) => format!("int: {mn}..{mx}"),
            (Some(mn), None) => format!("int: {mn}.."),
            (None, Some(mx)) => format!("int: ..{mx}"),
            (None, None) => "int: ..".to_string(),
        },
    }
}

fn one_line(definition: &str, max_chars: usize) -> String {
    let first = definition.lines().next().unwrap_or("").trim();
    let collapsed = first.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = collapsed.chars().count();
    if count <= max_chars {
        collapsed
    } else {
        let limit = max_chars.saturating_sub(1);
        let mut s: String = collapsed.chars().take(limit).collect();
        s.push('…');
        s
    }
}

fn hits_text(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| format!("{}/{}", h.topic, h.id))
        .collect::<Vec<_>>()
        .join(", ")
}

fn available_topics_summary(vocab: &Vocabulary) -> String {
    let topics = vocab.topics();
    if topics.is_empty() {
        "none".to_string()
    } else {
        topics
            .iter()
            .map(|t| format!("{} ({} terms)", t.id, t.term_count))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn append_findings_footer(text: &mut String, findings_count: usize) {
    if findings_count > 0 {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&format!(
            "Vocabulary findings: {findings_count} (run Kiro: Vocabulary Topics)"
        ));
    }
}

fn degraded(scoped: &Scoped, cmd: &str) -> Option<Output> {
    match scoped.loaded.status {
        Status::Missing => Some(Output::info(
            "Kiro: No vocabulary",
            "No vocabulary file: .kiro/vocabulary.toml not found.\nSee docs/vocabulary.md to create one or import it with kiro-cli.",
        )),
        Status::Invalid => Some(Output::with_findings(
            "Kiro: Vocabulary invalid",
            "The vocabulary could not be loaded.",
            &scoped.loaded.findings,
        )),
        Status::Loaded => {
            if cmd != "topics" && scoped.topics.is_empty() {
                let avail_str = available_topics_summary(&scoped.loaded.vocabulary);
                Some(Output::info(
                    format!("Kiro: {} declares no topics", scoped.spec),
                    format!(
                        "Spec {} declares no topics: add \"topics\": [\"sdd\"] to .kiro/specs/{}/spec.json.\nAvailable topics: {}",
                        scoped.spec, scoped.spec, avail_str
                    ),
                ))
            } else {
                None
            }
        }
    }
}

fn format_list_text(
    scoped: &Scoped,
    topic_filter: Option<&str>,
    visible_terms: &[&Term],
) -> String {
    let mut text = format!(
        "Spec: {} (topics: {})",
        scoped.spec,
        scoped.topics.join(", ")
    );
    if let Some(t) = topic_filter {
        text.push_str(&format!("\nTopic: {t}"));
    }
    text.push_str("\n\n");

    let rows: Vec<Vec<String>> = visible_terms
        .iter()
        .map(|term| {
            vec![
                term.id.clone(),
                term.label.clone(),
                term.topic.clone(),
                kind_text(&term.kind),
                one_line(&term.definition, DEFINITION_COLUMN_CHARS),
            ]
        })
        .collect();

    text.push_str(&table(
        &["Id", "Label", "Topic", "Kind", "Definition"],
        &rows,
    ));
    text
}

/// Executes `vocab.term`: looks up a term within the spec's declared topics.
fn term(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let scoped = scoped(ctx, req)?;
    if let Some(out) = degraded(&scoped, "term") {
        ctx.host
            .log(LogLevel::Debug, "vocab.term: outcome degraded");
        return Ok(out);
    }

    let query = req.rest.join(" ");
    let query_trimmed = query.trim();

    if query_trimmed.is_empty() {
        ctx.host
            .log(LogLevel::Debug, "vocab.term: outcome empty query");
        let visible = scoped.loaded.vocabulary.visible(&scoped.topics);
        let list_body = format_list_text(&scoped, None, &visible);
        let mut text = format!(
            "vocab.term expects a term label or id as its argument (e.g. run plugin.kiro.vocab.term with \"acceptance criterion\").\n\n{list_body}"
        );
        append_findings_footer(&mut text, scoped.loaded.findings.len());
        return Ok(Output::info("Kiro: Vocabulary Term", text));
    }

    let lookup = scoped
        .loaded
        .vocabulary
        .resolve(query_trimmed, &scoped.topics);
    let out = match lookup {
        Lookup::Found {
            term,
            via,
            broader,
            replacement,
        } => {
            ctx.host.log(LogLevel::Debug, "vocab.term: outcome Found");
            let mut text = format!(
                "Term: {}\nLabel: {}\nTopic: {}\nKind: {}\nDefinition: {}",
                term.id,
                term.label,
                term.topic,
                kind_text(&term.kind),
                term.definition
            );
            if !broader.is_empty() {
                let broader_str = broader
                    .iter()
                    .map(|b| b.id.as_str())
                    .collect::<Vec<_>>()
                    .join(" > ");
                text.push_str(&format!("\nBroader: {broader_str}"));
            }
            if !term.alt_labels.is_empty() {
                text.push_str(&format!("\nAlt labels: {}", term.alt_labels.join(", ")));
            }
            if term.deprecated {
                if let Some(rep) = replacement {
                    text.push_str(&format!(
                        "\nDeprecated: yes, replaced by {} ({})",
                        rep.id, rep.label
                    ));
                } else if let Some(rep_id) = &term.replaced_by {
                    text.push_str(&format!(
                        "\nDeprecated: yes, replacement {rep_id} is not in the declared topics"
                    ));
                } else {
                    text.push_str("\nDeprecated: yes, no replacement");
                }
            }
            let via_str = match via {
                MatchVia::Id => "id".to_string(),
                MatchVia::Label => "label".to_string(),
                MatchVia::AltLabel(alt) => format!("alt label \"{alt}\""),
            };
            text.push_str(&format!("\nMatched via: {via_str}"));
            text.push_str(&format!(
                "\nScope: {} (topics: {})",
                scoped.spec,
                scoped.topics.join(", ")
            ));
            append_findings_footer(&mut text, scoped.loaded.findings.len());
            Output::info(format!("Kiro: {} ({})", term.label, term.topic), text)
        }
        Lookup::NotInTopic { hits } => {
            ctx.host
                .log(LogLevel::Debug, "vocab.term: outcome NotInTopic");
            let mut text = format!(
                "`{query_trimmed}` is a term of another topic: {}.\nSpec {} declares: {}.\nCross-topic matches are never resolved; declare the topic in spec.json if the spec is about it.",
                hits_text(&hits),
                scoped.spec,
                scoped.topics.join(", ")
            );
            append_findings_footer(&mut text, scoped.loaded.findings.len());
            Output {
                title: format!("Kiro: {query_trimmed} not in scope"),
                level: Level::Warning,
                text,
            }
        }
        Lookup::Ambiguous { hits } => {
            ctx.host
                .log(LogLevel::Debug, "vocab.term: outcome Ambiguous");
            let mut text = format!(
                "`{query_trimmed}` matches several terms in the declared topics: {}. Use the term id.",
                hits_text(&hits)
            );
            append_findings_footer(&mut text, scoped.loaded.findings.len());
            Output {
                title: format!("Kiro: {query_trimmed} is ambiguous"),
                level: Level::Warning,
                text,
            }
        }
        Lookup::Unknown => {
            ctx.host.log(LogLevel::Debug, "vocab.term: outcome Unknown");
            let mut text = format!(
                "`{query_trimmed}` is not a term in topics {} of spec {}.",
                scoped.topics.join(", "),
                scoped.spec
            );
            append_findings_footer(&mut text, scoped.loaded.findings.len());
            Output {
                title: format!("Kiro: {query_trimmed} unknown"),
                level: Level::Warning,
                text,
            }
        }
        Lookup::Unscoped => {
            ctx.host
                .log(LogLevel::Debug, "vocab.term: outcome Unscoped");
            let avail_str = available_topics_summary(&scoped.loaded.vocabulary);
            Output::info(
                format!("Kiro: {} declares no topics", scoped.spec),
                format!(
                    "Spec {} declares no topics: add \"topics\": [\"sdd\"] to .kiro/specs/{}/spec.json.\nAvailable topics: {}",
                    scoped.spec, scoped.spec, avail_str
                ),
            )
        }
    };

    Ok(out)
}

/// Executes `vocab.list`: lists visible vocabulary terms for the spec's declared topics.
fn list(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let scoped = scoped(ctx, req)?;
    if let Some(out) = degraded(&scoped, "list") {
        ctx.host
            .log(LogLevel::Debug, "vocab.list: outcome degraded");
        return Ok(out);
    }

    let topic_arg = req.rest.first().map(String::as_str);
    let (selected_topics, topic_filter) = match topic_arg {
        Some(t) => {
            if scoped.topics.iter().any(|st| st == t) {
                (vec![t.to_string()], Some(t))
            } else {
                ctx.host
                    .log(LogLevel::Debug, "vocab.list: outcome topic not declared");
                let mut text = format!(
                    "Topic {t} is not declared by spec {}; declared: {}.",
                    scoped.spec,
                    scoped.topics.join(", ")
                );
                append_findings_footer(&mut text, scoped.loaded.findings.len());
                return Ok(Output {
                    title: format!("Kiro: topic {t} not declared"),
                    level: Level::Warning,
                    text,
                });
            }
        }
        None => (scoped.topics.clone(), None),
    };

    ctx.host.log(LogLevel::Debug, "vocab.list: outcome Ok");
    let visible_terms = scoped.loaded.vocabulary.visible(&selected_topics);
    let mut text = format_list_text(&scoped, topic_filter, &visible_terms);
    append_findings_footer(&mut text, scoped.loaded.findings.len());
    let title = format!(
        "Kiro: {} terms visible to {}",
        visible_terms.len(),
        scoped.spec
    );
    Ok(Output::info(title, text))
}

/// Executes `vocab.topics`: lists declared, available, and missing topics along with findings.
fn topics(ctx: &mut CommandContext<'_>, req: &Request) -> Result<Output, String> {
    let scoped = scoped(ctx, req)?;
    if let Some(out) = degraded(&scoped, "topics") {
        ctx.host
            .log(LogLevel::Debug, "vocab.topics: outcome degraded");
        return Ok(out);
    }

    ctx.host.log(LogLevel::Debug, "vocab.topics: outcome Ok");
    let mut lines = Vec::new();
    lines.push(format!("Spec: {}", scoped.spec));

    if scoped.topics.is_empty() {
        lines.push("Declared: none".to_string());
    } else {
        lines.push(format!("Declared: {}", scoped.topics.join(", ")));
    }

    let avail_topics = scoped.loaded.vocabulary.topics();
    if avail_topics.is_empty() {
        lines.push("Available: none".to_string());
    } else {
        let avail_strs: Vec<String> = avail_topics
            .iter()
            .map(|t| {
                if let Some(lbl) = &t.label {
                    format!("{} ({} terms, \"{}\")", t.id, t.term_count, lbl)
                } else {
                    format!("{} ({} terms)", t.id, t.term_count)
                }
            })
            .collect();
        lines.push(format!("Available: {}", avail_strs.join(", ")));
    }

    // Not declared: available topics not in declared topics
    let not_declared: Vec<&str> = avail_topics
        .iter()
        .map(|t| t.id.as_str())
        .filter(|tid| !scoped.topics.iter().any(|st| st == *tid))
        .collect();
    if !not_declared.is_empty() {
        lines.push(format!("Not declared: {}", not_declared.join(", ")));
    }

    // Unknown to vocabulary: declared topics not in available topics
    let unknown: Vec<&str> = scoped
        .topics
        .iter()
        .map(String::as_str)
        .filter(|st| !avail_topics.iter().any(|at| at.id == *st))
        .collect();
    if !unknown.is_empty() {
        lines.push(format!("Unknown to the vocabulary: {}", unknown.join(", ")));
    }

    let status_str = match scoped.loaded.status {
        Status::Loaded => "loaded",
        Status::Missing => "missing",
        Status::Invalid => "invalid",
    };
    lines.push(format!("Vocabulary: {status_str}"));

    let mut all_findings = scoped.scope_findings;
    all_findings.extend(scoped.loaded.findings);

    let level = level_of(&all_findings);
    let mut text = lines.join("\n");
    if !all_findings.is_empty() {
        text.push_str("\n\n");
        text.push_str(&kiro_core::findings_text(&all_findings));
    }

    let title = format!("Kiro: Topics of {}", scoped.spec);
    Ok(Output { title, level, text })
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
    use crate::host::FakeHost;

    #[test]
    fn helper_kind_text_variants() {
        assert_eq!(kind_text(&Kind::Entity), "entity");
        assert_eq!(kind_text(&Kind::Bool), "bool");
        assert_eq!(
            kind_text(&Kind::Enum {
                values: vec!["a".into(), "b".into()]
            }),
            "enum: a, b"
        );
        assert_eq!(
            kind_text(&Kind::Int {
                min: Some(0),
                max: Some(10)
            }),
            "int: 0..10"
        );
        assert_eq!(
            kind_text(&Kind::Int {
                min: Some(5),
                max: None
            }),
            "int: 5.."
        );
        assert_eq!(
            kind_text(&Kind::Int {
                min: None,
                max: Some(10)
            }),
            "int: ..10"
        );
        assert_eq!(
            kind_text(&Kind::Int {
                min: None,
                max: None
            }),
            "int: .."
        );
    }

    #[test]
    fn helper_one_line_truncation() {
        assert_eq!(one_line("Simple line", 72), "Simple line");
        assert_eq!(one_line("  Multiline\nSecond line", 72), "Multiline");
        assert_eq!(
            one_line("Internal   whitespace   collapsed", 72),
            "Internal whitespace collapsed"
        );
        assert_eq!(one_line("Hello world", 5), "Hell…");
    }

    #[test]
    fn helper_hits_text() {
        let hits = vec![
            Hit {
                topic: "sdd".into(),
                id: "task".into(),
            },
            Hit {
                topic: "pm".into(),
                id: "pm-task".into(),
            },
        ];
        assert_eq!(hits_text(&hits), "sdd/task, pm/pm-task");
    }

    #[test]
    fn degraded_missing_and_invalid_and_empty_topics() {
        let missing_scoped = Scoped {
            spec: "demo".into(),
            topics: vec!["sdd".into()],
            loaded: kiro_core::vocab::Loaded {
                status: Status::Missing,
                vocabulary: Default::default(),
                findings: vec![],
            },
            scope_findings: vec![],
        };
        let out_m = degraded(&missing_scoped, "term").unwrap();
        assert_eq!(out_m.title, "Kiro: No vocabulary");
        assert_eq!(out_m.level, Level::Info);

        let invalid_scoped = Scoped {
            spec: "demo".into(),
            topics: vec!["sdd".into()],
            loaded: kiro_core::vocab::Loaded {
                status: Status::Invalid,
                vocabulary: Default::default(),
                findings: vec![Finding::new(
                    kiro_core::finding::codes::VOCAB_INVALID,
                    ".kiro/vocabulary.toml",
                    "syntax error",
                )],
            },
            scope_findings: vec![],
        };
        let out_i = degraded(&invalid_scoped, "term").unwrap();
        assert_eq!(out_i.title, "Kiro: Vocabulary invalid");
        assert_eq!(out_i.level, Level::Error);

        let no_topics_scoped = Scoped {
            spec: "none".into(),
            topics: vec![],
            loaded: kiro_core::vocab::Loaded {
                status: Status::Loaded,
                vocabulary: Default::default(),
                findings: vec![],
            },
            scope_findings: vec![],
        };
        // term/list are short-circuited
        let out_n = degraded(&no_topics_scoped, "term").unwrap();
        assert_eq!(out_n.title, "Kiro: none declares no topics");
        assert_eq!(out_n.level, Level::Info);
        // topics is NOT short-circuited
        assert!(degraded(&no_topics_scoped, "topics").is_none());
    }

    fn make_test_host() -> FakeHost {
        let vocab_content = include_str!("../../../crates/kiro-core/fixtures/vocab/valid.toml");
        FakeHost::new()
            .with_root("C:\\proj")
            .with_file(
                ".kiro/specs/demo/spec.json",
                "{\"feature\": \"demo\", \"phase\": \"initialized\", \"topics\": [\"sdd\"]}",
            )
            .with_file(
                ".kiro/specs/both/spec.json",
                "{\"feature\": \"both\", \"phase\": \"initialized\", \"topics\": [\"sdd\", \"pm\"]}",
            )
            .with_file(
                ".kiro/specs/none/spec.json",
                "{\"feature\": \"none\", \"phase\": \"initialized\", \"topics\": []}",
            )
            .with_file(
                ".kiro/specs/stale/spec.json",
                "{\"feature\": \"stale\", \"phase\": \"initialized\", \"topics\": [\"legacy\"]}",
            )
            .with_file(".kiro/vocabulary.toml", vocab_content)
    }

    #[test]
    fn term_outcomes_via_command_context() {
        let host = make_test_host();
        let mut ctx = CommandContext::new(&host, Some("demo")).unwrap();

        // 1. Found via Id ("task")
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["task".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Task (sdd)");
        assert_eq!(out.level, Level::Info);
        assert!(out.text.contains("Term: task"));
        assert!(out.text.contains("Matched via: id"));
        assert!(out.text.contains("Broader: spec"));
        assert!(
            out.text
                .contains("Vocabulary findings: 1 (run Kiro: Vocabulary Topics)")
        );

        // 2. Found via Label ("Specification")
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["Specification".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Specification (sdd)");
        assert!(out.text.contains("Matched via: label"));

        // 3. Found via AltLabel ("criterion")
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["criterion".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Acceptance criterion (sdd)");
        assert!(out.text.contains("Matched via: alt label \"criterion\""));

        // 4. Deprecated with replacement ("old-task")
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["old-task".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert!(
            out.text
                .contains("Deprecated: yes, replaced by task (Task)")
        );

        // 5. Not in topic ("sprint" is in pm)
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["sprint".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: sprint not in scope");
        assert_eq!(out.level, Level::Warning);
        assert!(
            out.text
                .contains("`sprint` is a term of another topic: pm/sprint")
        );

        // 6. Unknown
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["nonexistent".into()],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: nonexistent unknown");
        assert_eq!(out.level, Level::Warning);

        // 7. Ambiguous (on "both" spec)
        let mut both_ctx = CommandContext::new(&host, Some("both")).unwrap();
        let req = Request {
            spec: Some("both".into()),
            insert: false,
            rest: vec!["Task".into()],
            raw: None,
        };
        let out = term(&mut both_ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Task is ambiguous");
        assert_eq!(out.level, Level::Warning);
        assert!(
            out.text
                .contains("matches several terms in the declared topics: pm/pm-task, sdd/task")
        );

        // 8. Empty query
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        let out = term(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Vocabulary Term");
        assert_eq!(out.level, Level::Info);
        assert!(
            out.text
                .contains("vocab.term expects a term label or id as its argument")
        );
        assert!(out.text.contains("| Id"));
    }

    #[test]
    fn list_and_topics_outputs() {
        let host = make_test_host();
        let mut ctx = CommandContext::new(&host, Some("demo")).unwrap();

        // 1. List all visible
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        let out = list(&mut ctx, &req).unwrap();
        assert!(out.title.contains("terms visible to demo"));
        assert_eq!(out.level, Level::Info);
        assert!(out.text.contains("Spec: demo (topics: sdd)"));
        assert!(
            out.text
                .contains("Vocabulary findings: 1 (run Kiro: Vocabulary Topics)")
        );

        // 2. List with valid topic filter
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["sdd".into()],
            raw: None,
        };
        let out = list(&mut ctx, &req).unwrap();
        assert!(out.text.contains("Topic: sdd"));

        // 3. List with undeclared topic filter
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec!["pm".into()],
            raw: None,
        };
        let out = list(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: topic pm not declared");
        assert_eq!(out.level, Level::Warning);
        assert!(
            out.text
                .contains("Topic pm is not declared by spec demo; declared: sdd.")
        );

        // 4. Topics on demo
        let req = Request {
            spec: Some("demo".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        let out = topics(&mut ctx, &req).unwrap();
        assert_eq!(out.title, "Kiro: Topics of demo");
        assert_eq!(out.level, Level::Info); // VOCAB_LABEL_CROSS_TOPIC is info
        assert!(out.text.contains("Spec: demo"));
        assert!(out.text.contains("Declared: sdd"));
        assert!(out.text.contains("Available:"));
        assert!(out.text.contains("Not declared: pm"));
        assert!(out.text.contains("Vocabulary: loaded"));
        assert!(out.text.contains("VOCAB_LABEL_CROSS_TOPIC"));

        // 5. Topics on none
        let mut none_ctx = CommandContext::new(&host, Some("none")).unwrap();
        let none_req = Request {
            spec: Some("none".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        let out = topics(&mut none_ctx, &none_req).unwrap();
        assert_eq!(out.title, "Kiro: Topics of none");
        assert!(out.text.contains("Declared: none"));
        assert!(out.text.contains("TOPICS_UNDECLARED"));

        // 6. Topics on stale
        let mut stale_ctx = CommandContext::new(&host, Some("stale")).unwrap();
        let stale_req = Request {
            spec: Some("stale".into()),
            insert: false,
            rest: vec![],
            raw: None,
        };
        let out = topics(&mut stale_ctx, &stale_req).unwrap();
        assert_eq!(out.title, "Kiro: Topics of stale");
        assert_eq!(out.level, Level::Warning); // TOPIC_UNKNOWN is warning
        assert!(out.text.contains("Declared: legacy"));
        assert!(out.text.contains("Unknown to the vocabulary: legacy"));
        assert!(out.text.contains("TOPIC_UNKNOWN"));
    }
}
