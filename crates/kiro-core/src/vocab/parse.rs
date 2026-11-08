//! Strict TOML parser for vocabulary files (design component `VocabParser`).
//!
//! Deserialises `.kiro/vocabulary.toml` into raw types with spans,
//! or produces a structural `VOCAB_INVALID` finding at the offending line.
