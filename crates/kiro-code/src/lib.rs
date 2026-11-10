//! Native code measurement for the kiro code gate (spec `code-constraints`).
//!
//! Reads a confined, immutable bundle of source texts and measures it into
//! the engine's `CodeFacts`. Parsing is adopted from `ast-grep-core`; this
//! crate writes no lexer and no parser.

pub mod clones;
pub mod git;
pub mod locality;
pub mod measure;
pub mod scan;
pub mod shapes;
pub mod walk;

use kiro_core::code::CodeFacts;

pub use git::{ChangeSet, GitError, read_before};
pub use measure::{DEFINITION_VERSION, PRODUCER, measure_into, profile_for};
pub use walk::{
    BundleError, DEFAULT_EXCLUDED_DIRS, MAX_SOURCE_BYTES, SourceBundle, SourceText, read_bundle,
};

/// Measures every text of a bundle. The result is in canonical order.
#[must_use]
pub fn measure(bundle: &SourceBundle) -> CodeFacts {
    let mut facts = CodeFacts::default();
    for file in &bundle.files {
        measure_into(&file.path, &file.text, &mut facts);
    }
    for unreadable in &bundle.unreadable {
        facts.coverage.push(kiro_core::code::CoverageCell {
            path: unreadable.path.clone(),
            metric: None,
            state: kiro_core::code::Completeness::Unavailable,
            reason: Some(unreadable.reason.clone()),
            line: None,
        });
    }
    facts.sort();
    facts
}
