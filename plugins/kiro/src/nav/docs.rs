//! Document outline view for requirements, design and tasks documents (task 2.6).

use crate::args::Request;
use crate::context::CommandContext;
use crate::host::Level;
use crate::nav::{checkbox, file_name, reject_extra};
use crate::present::Output;
use kiro_core::source::Feature;
use kiro_core::spec::Doc;

/// A single entry in a document outline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineEntry {
    pub line: usize,
    pub indent: usize,
    pub text: String,
}

/// Executes an outline command (`nav.to-requirements`, `nav.to-design`, or `nav.to-tasks`).
pub fn run(ctx: &mut CommandContext<'_>, req: &Request, doc: Doc) -> Result<Output, String> {
    let cmd = match doc {
        Doc::Requirements => &crate::nav::COMMANDS[3],
        Doc::Design => &crate::nav::COMMANDS[4],
        Doc::Tasks => &crate::nav::COMMANDS[5],
        Doc::SpecJson => return Err("spec.json has no outline command".to_string()),
    };
    reject_extra(cmd, req, 0)?;

    let resolved = ctx.resolve(req.spec.as_deref())?;
    let feature = ctx.load(&resolved.name);
    let path = feature.path(doc);
    let entries = outline(feature, doc);
    let (text, level) = outline_text(&resolved.name, &path, entries.as_deref());
    let doc_file_name = file_name(&path);
    let title = format!("Kiro: {} {doc_file_name}", resolved.name);

    Ok(Output { title, level, text })
}

/// Extracts the outline entries for `doc` from `feature`.
///
/// Returns `None` if the document is absent or for `Doc::SpecJson`.
#[must_use]
pub fn outline(feature: &Feature, doc: Doc) -> Option<Vec<OutlineEntry>> {
    match doc {
        Doc::SpecJson => None,
        Doc::Requirements => {
            let r = feature.requirements.as_ref()?;
            let entries = r
                .outline
                .iter()
                .map(|h| OutlineEntry {
                    line: h.line,
                    indent: 0,
                    text: format!("{} {}", "#".repeat(usize::from(h.level)), h.text),
                })
                .collect();
            Some(entries)
        }
        Doc::Design => {
            let d = feature.design.as_ref()?;
            let entries = d
                .outline
                .iter()
                .map(|h| OutlineEntry {
                    line: h.line,
                    indent: 0,
                    text: format!("{} {}", "#".repeat(usize::from(h.level)), h.text),
                })
                .collect();
            Some(entries)
        }
        Doc::Tasks => {
            let t = feature.tasks.as_ref()?;
            let mut entries: Vec<OutlineEntry> = Vec::new();

            for h in &t.outline {
                entries.push(OutlineEntry {
                    line: h.line,
                    indent: 0,
                    text: format!("{} {}", "#".repeat(usize::from(h.level)), h.text),
                });
            }

            for task in t.all() {
                let indent = if task.sub.is_none() { 0 } else { 2 };
                entries.push(OutlineEntry {
                    line: task.line,
                    indent,
                    text: format!(
                        "- {} {} {}",
                        checkbox(task.checked),
                        task.id,
                        task.description
                    ),
                });
            }

            entries.sort_by_key(|e| e.line);
            Some(entries)
        }
    }
}

/// Formats the outline entries into human-readable text.
#[must_use]
pub fn outline_text(spec: &str, path: &str, entries: Option<&[OutlineEntry]>) -> (String, Level) {
    let mut lines = Vec::new();
    lines.push(format!("Spec: {spec}"));

    let level = match entries {
        Some(list) => {
            lines.push(format!("Document: {path}"));
            if list.is_empty() {
                lines.push("Outline: empty".to_string());
            } else {
                lines.push(format!("Outline: {} entries", list.len()));
                lines.push(String::new());
                for e in list {
                    let spaces = " ".repeat(e.indent);
                    lines.push(format!("{}: {spaces}{}", e.line, e.text));
                }
            }
            Level::Info
        }
        None => {
            lines.push(format!("Document: {path} (missing)"));
            lines.push("Outline: empty".to_string());
            Level::Warning
        }
    };

    (lines.join("\n"), level)
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
    use crate::context::CommandContext;
    use crate::host::FakeHost;

    fn fixture_host() -> FakeHost {
        let host = FakeHost::new().with_root("C:\\proj");
        host.with_file(
            ".kiro/steering/roadmap.md",
            include_str!("../../tests/fixtures/nav/.kiro/steering/roadmap.md"),
        )
        .with_file(
            ".kiro/specs/alpha/spec.json",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/spec.json"),
        )
        .with_file(
            ".kiro/specs/alpha/requirements.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/requirements.md"),
        )
        .with_file(
            ".kiro/specs/alpha/design.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/design.md"),
        )
        .with_file(
            ".kiro/specs/alpha/tasks.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/alpha/tasks.md"),
        )
        .with_file(
            ".kiro/specs/gamma/spec.json",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/spec.json"),
        )
        .with_file(
            ".kiro/specs/gamma/requirements.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/requirements.md"),
        )
        .with_file(
            ".kiro/specs/gamma/tasks.md",
            include_str!("../../tests/fixtures/nav/.kiro/specs/gamma/tasks.md"),
        )
    }

    #[test]
    fn headings_only_outline() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let alpha = ctx.load("alpha");

        let entries = outline(alpha, Doc::Requirements).unwrap();
        assert!(!entries.is_empty());
        assert_eq!(entries[0].indent, 0);
        assert_eq!(entries[0].text, "# Requirements Document: alpha");
        assert_eq!(entries[1].text, "## Requirements");

        let (text, level) =
            outline_text("alpha", ".kiro/specs/alpha/requirements.md", Some(&entries));
        assert_eq!(level, Level::Info);
        assert!(text.contains("Spec: alpha"));
        assert!(text.contains("Document: .kiro/specs/alpha/requirements.md"));
        assert!(text.contains(&format!("Outline: {} entries", entries.len())));
        assert!(text.contains("1: # Requirements Document: alpha"));
    }

    #[test]
    fn tasks_merged_outline() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let alpha = ctx.load("alpha");

        let entries = outline(alpha, Doc::Tasks).unwrap();
        // Alpha tasks.md has:
        // line 1: # Implementation Plan: alpha
        // line 3: - [ ] 1. Foundation
        // line 4: - [x] 1.1 Unscoped task
        // line 5: - [ ] 2. Core
        // line 6: - [x] 2.1 Build alpha
        // line 9: - [ ] 2.2 Empty work
        // line 11: - [ ] 3. Integration
        // line 12: - [ ] 3.1 Glue work
        assert_eq!(entries[0].line, 1);
        assert_eq!(entries[0].indent, 0);
        assert_eq!(entries[0].text, "# Implementation Plan: alpha");

        assert_eq!(entries[1].line, 3);
        assert_eq!(entries[1].indent, 0);
        assert_eq!(entries[1].text, "- [ ] 1 Foundation");

        assert_eq!(entries[2].line, 4);
        assert_eq!(entries[2].indent, 2);
        assert_eq!(entries[2].text, "- [x] 1.1 Unscoped task");

        let (text, level) = outline_text("alpha", ".kiro/specs/alpha/tasks.md", Some(&entries));
        assert_eq!(level, Level::Info);
        assert!(text.contains("3: - [ ] 1 Foundation"));
        assert!(text.contains("4:   - [x] 1.1 Unscoped task"));
    }

    #[test]
    fn missing_and_empty_document_outline() {
        let host = fixture_host();
        let mut ctx = CommandContext::new(&host, None).unwrap();
        let gamma = ctx.load("gamma");

        // Missing design
        let entries = outline(gamma, Doc::Design);
        assert!(entries.is_none());

        let (text, level) =
            outline_text("gamma", ".kiro/specs/gamma/design.md", entries.as_deref());
        assert_eq!(level, Level::Warning);
        assert!(text.contains("Document: .kiro/specs/gamma/design.md (missing)"));
        assert!(text.contains("Outline: empty"));

        // Empty entries
        let empty_entries: Vec<OutlineEntry> = Vec::new();
        let (text_empty, level_empty) =
            outline_text("gamma", ".kiro/specs/gamma/empty.md", Some(&empty_entries));
        assert_eq!(level_empty, Level::Info);
        assert!(text_empty.contains("Document: .kiro/specs/gamma/empty.md"));
        assert!(text_empty.contains("Outline: empty"));
    }
}
