//! Topic-scoped vocabulary loader and queries (design component `VocabLoader`).
//!
//! Loads and validates `.kiro/vocabulary.toml`, providing cached access
//! via `Workspace`, resolution of terms strictly inside a spec's declared
//! topics, and semantic findings.

pub mod index;
pub mod model;
pub mod parse;
pub mod scope;

pub use model::{
    Hit, Kind, SCHEMA_VERSION, Term, Topic, VOCABULARY_PATH, Vocabulary, is_id, normalize_label,
};
