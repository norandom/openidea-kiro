//! One pass over the whole project for the rules that count across files:
//! duplicate blocks, locality and similar-routine families. Every file is
//! parsed once here and the tree is shared by the extractions.

use ast_grep_core::AstGrep;
use ast_grep_core::tree_sitter::StrDoc;
use kiro_core::code::clones::FileLines;
use kiro_core::code::data::DataFacts;
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
    /// Accesses to tabular data by position.
    pub data: DataFacts,
}

/// Scans one file. A file that no profile covers or that does not parse
/// contributes nothing: no rule is judged on a tree with holes.
fn scan_file(
    file: &SourceText,
    min_statements: u32,
    out: &mut ProjectScan,
    shapes: &mut ShapeCollector,
) {
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
    shapes.add_file(&root, profile, comments, &file.path, min_statements);
    if crate::data_access::covers(profile.name) {
        crate::data_access::extract(&root, profile.name, &file.path, &mut out.data.accesses);
        out.data.files = out.data.files.saturating_add(1);
    }
}

/// What the similar-routine comparison takes from the configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScanSettings {
    /// Fewest statements of a routine that is compared.
    pub min_statements: u32,
    /// Lowest similarity that connects two routines.
    pub threshold: f64,
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            min_statements: DEFAULT_MIN_STATEMENTS,
            threshold: DEFAULT_THRESHOLD,
        }
    }
}

/// Scans every file of the bundle that a profile covers, with the
/// built-in settings.
#[must_use]
pub fn scan(bundle: &SourceBundle) -> ProjectScan {
    scan_with(bundle, &ScanSettings::default())
}

/// Scans every file of the bundle that a profile covers.
#[must_use]
pub fn scan_with(bundle: &SourceBundle, settings: &ScanSettings) -> ProjectScan {
    let mut out = ProjectScan::default();
    let mut shapes = ShapeCollector::default();
    for file in &bundle.files {
        scan_file(file, settings.min_statements, &mut out, &mut shapes);
    }
    out.locality.bindings.sort();
    out.locality.comparisons.sort();
    out.families = shapes.finish(settings.threshold);
    out
}
