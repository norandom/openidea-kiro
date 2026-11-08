//! Topic-scoped vocabulary loader and queries (design component `VocabLoader`).
//!
//! Loads and validates `.kiro/vocabulary.toml`, providing cached access
//! via `Workspace`, resolution of terms strictly inside a spec's declared
//! topics, and semantic findings.

pub mod index;
pub mod model;
pub mod parse;
pub mod scope;

use crate::finding::{Finding, codes};
use crate::source::{Source, Workspace};
use crate::vocab::index::build;
use crate::vocab::parse::parse_raw;

pub use model::{
    Hit, Kind, SCHEMA_VERSION, Term, Topic, VOCABULARY_PATH, Vocabulary, is_id, normalize_label,
};
pub use scope::{Lookup, MatchVia, scope_findings, spec_topics};

/// Status of the loaded vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Loaded,
    Missing,
    Invalid,
}

/// The result of loading a vocabulary: status, vocabulary value and findings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loaded {
    pub status: Status,
    pub vocabulary: Vocabulary,
    pub findings: Vec<Finding>,
}

/// Parses the text of a vocabulary file.
/// If invalid, returns `Status::Invalid` with an empty vocabulary and `[VOCAB_INVALID]`.
/// Otherwise returns `Status::Loaded` with the indexed vocabulary and sorted build findings.
#[must_use]
pub fn parse(text: &str, file: &str) -> Loaded {
    match parse_raw(text, file) {
        Ok(raw) => {
            let (vocabulary, findings) = build(raw, text, file);
            Loaded {
                status: Status::Loaded,
                vocabulary,
                findings,
            }
        }
        Err(finding) => Loaded {
            status: Status::Invalid,
            vocabulary: Vocabulary::default(),
            findings: vec![finding],
        },
    }
}

/// Loads `.kiro/vocabulary.toml` through the workspace cache.
/// Returns `Status::Missing` with `[VOCAB_MISSING]` if absent, or `parse` output if present.
pub fn load<S: Source>(ws: &mut Workspace<S>) -> Loaded {
    match ws.read(VOCABULARY_PATH) {
        Some(text) => parse(text, VOCABULARY_PATH),
        None => Loaded {
            status: Status::Missing,
            vocabulary: Vocabulary::default(),
            findings: vec![Finding::new(
                codes::VOCAB_MISSING,
                VOCABULARY_PATH,
                ".kiro/vocabulary.toml could not be read",
            )],
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;
    use crate::source::MemorySource;
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    struct CountingSource {
        files: MemorySource,
        counts: RefCell<BTreeMap<String, usize>>,
    }

    impl CountingSource {
        fn new(files: MemorySource) -> Self {
            Self {
                files,
                counts: RefCell::new(BTreeMap::new()),
            }
        }

        fn count(&self, path: &str) -> usize {
            self.counts.borrow().get(path).copied().unwrap_or(0)
        }
    }

    impl Source for CountingSource {
        fn read(&self, path: &str) -> Option<String> {
            *self.counts.borrow_mut().entry(path.to_owned()).or_insert(0) += 1;
            self.files.read(path)
        }
    }

    #[test]
    fn parse_valid_returns_loaded_status_and_findings() {
        let content = "schema_version = \"2\"\n\n[terms.task]\ntopic = \"sdd\"\nlabel = \"Task\"\ndefinition = \"A task.\"\n";
        let loaded = parse(content, VOCABULARY_PATH);
        assert_eq!(loaded.status, Status::Loaded);
        assert_eq!(loaded.vocabulary.len(), 1);
        assert!(loaded.findings.is_empty());
    }

    #[test]
    fn parse_invalid_returns_invalid_status_empty_vocab_and_single_finding() {
        let content = "schema_version = \"2\"\n\n[terms.task\n";
        let loaded = parse(content, VOCABULARY_PATH);
        assert_eq!(loaded.status, Status::Invalid);
        assert!(loaded.vocabulary.is_empty());
        assert_eq!(loaded.findings.len(), 1);
        assert_eq!(loaded.findings[0].code, codes::VOCAB_INVALID);
    }

    #[test]
    fn workspace_load_missing_returns_missing_status_empty_vocab_and_single_finding() {
        let source = CountingSource::new(MemorySource::new());
        let mut ws = Workspace::new(source, Vec::<String>::new());
        let loaded = load(&mut ws);
        assert_eq!(loaded.status, Status::Missing);
        assert!(loaded.vocabulary.is_empty());
        assert_eq!(loaded.findings.len(), 1);
        assert_eq!(loaded.findings[0].code, codes::VOCAB_MISSING);

        // Exactly one read on absent file
        assert_eq!(ws.source().count(VOCABULARY_PATH), 1);

        // Repeated load should perform no additional read
        let _ = load(&mut ws);
        assert_eq!(ws.source().count(VOCABULARY_PATH), 1);
    }

    #[test]
    fn workspace_load_present_reads_exactly_once() {
        let content = "schema_version = \"2\"\n\n[terms.task]\ntopic = \"sdd\"\nlabel = \"Task\"\ndefinition = \"A task.\"\n";
        let source = CountingSource::new(MemorySource::new().with(VOCABULARY_PATH, content));
        let mut ws = Workspace::new(source, Vec::<String>::new());

        let loaded1 = load(&mut ws);
        assert_eq!(loaded1.status, Status::Loaded);
        assert_eq!(ws.source().count(VOCABULARY_PATH), 1);

        let loaded2 = load(&mut ws);
        assert_eq!(loaded2.status, Status::Loaded);
        assert_eq!(ws.source().count(VOCABULARY_PATH), 1); // exactly one read
    }
}
