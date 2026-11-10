//! One pass over the whole project for the rules that count across files:
//! duplicate blocks, locality and similar-routine families. Every file is
//! parsed once here and the tree is shared by the extractions.

use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use kiro_core::code::clones::FileLines;
use kiro_core::code::families::{DEFAULT_MIN_STATEMENTS, DEFAULT_THRESHOLD, FamilyFacts};
use kiro_core::code::locality::LocalityFacts;

use crate::measure::{comment_kinds, language_of, profile_for};
use crate::shapes::ShapeCollector;
use crate::walk::{SourceBundle, SourceText};

/// What the project-wide rules need.
#[derive(Debug, Clone, Default)]
pub struct ProjectScan {
    /// Code lines of every file that parses.
    pub lines: Vec<FileLines>,
    /// Module-level bindings and literal comparisons.
    pub locality: LocalityFacts,
    /// Routines of at least `DEFAULT_MIN_STATEMENTS` statements and the
    /// pairs that are at least `DEFAULT_THRESHOLD` similar.
    pub families: FamilyFacts,
}

/// Scans one file. A file that no profile covers or that does not parse
/// contributes nothing: no rule is judged on a tree with holes.
fn scan_file(file: &SourceText, out: &mut ProjectScan, shapes: &mut ShapeCollector) {
    let (Some(lang), Some(comments), Some(profile)) = (
        language_of(&file.path),
        comment_kinds(&file.path),
        profile_for(&file.path),
    ) else {
        return;
    };
    let normalized = file.text.replace("\r\n", "\n");
    let Ok(tree) = StrDoc::try_new(&normalized, lang).map(AstGrep::doc) else {
        return;
    };
    let root = tree.root();
    let Some(lines) = crate::clones::lines_of(&root, comments, &file.path) else {
        return;
    };
    out.lines.push(lines);
    crate::locality::extract(
        &root,
        profile.name,
        &file.path,
        &mut out.locality.bindings,
        &mut out.locality.comparisons,
    );
    out.locality.files = out.locality.files.saturating_add(1);
    shapes.add_file(&root, profile, comments, &file.path, DEFAULT_MIN_STATEMENTS);
}

/// Scans every file of the bundle that a profile covers.
#[must_use]
pub fn scan(bundle: &SourceBundle) -> ProjectScan {
    let mut out = ProjectScan::default();
    let mut shapes = ShapeCollector::default();
    for file in &bundle.files {
        scan_file(file, &mut out, &mut shapes);
    }
    out.locality.bindings.sort();
    out.locality.comparisons.sort();
    out.families = shapes.finish(DEFAULT_THRESHOLD);
    out
}
