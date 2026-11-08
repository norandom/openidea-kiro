//! Semantic validation and indexing for vocabulary files (design component `VocabIndex`).
//!
//! Validates term definitions, kinds, duplicate labels, broader relations,
//! replacement chains, cycle detection, and builds the nearest-first broader closure.
