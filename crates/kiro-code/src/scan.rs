//! One pass over the whole project for the rules that count across files:
//! duplicate blocks and locality. Every file is parsed once here and the
//! tree is shared by both extractions.

use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use kiro_core::code::clones::FileLines;
use kiro_core::code::locality::LocalityFacts;

use crate::measure::{comment_kinds, language_of, profile_name};
use crate::walk::SourceBundle;

/// What the project-wide rules need.
#[derive(Debug, Clone, Default)]
pub struct ProjectScan {
    /// Code lines of every file that parses.
    pub lines: Vec<FileLines>,
    /// Module-level bindings and literal comparisons.
    pub locality: LocalityFacts,
}

/// Scans every file of the bundle that a profile covers. A file that does
/// not parse contributes nothing: no rule is judged on a tree with holes.
#[must_use]
pub fn scan(bundle: &SourceBundle) -> ProjectScan {
    let mut out = ProjectScan::default();
    for file in &bundle.files {
        let (Some(lang), Some(comments), Some(profile)) = (
            language_of(&file.path),
            comment_kinds(&file.path),
            profile_name(&file.path),
        ) else {
            continue;
        };
        let normalized = file.text.replace("\r\n", "\n");
        let Ok(tree) = StrDoc::try_new(&normalized, lang).map(AstGrep::doc) else {
            continue;
        };
        let root = tree.root();
        let Some(lines) = crate::clones::lines_of(&root, comments, &file.path) else {
            continue;
        };
        out.lines.push(lines);
        crate::locality::extract(
            &root,
            profile,
            &file.path,
            &mut out.locality.bindings,
            &mut out.locality.comparisons,
        );
        out.locality.files = out.locality.files.saturating_add(1);
    }
    out.locality.bindings.sort();
    out.locality.comparisons.sort();
    out
}
