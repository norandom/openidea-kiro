//! Check runner and configuration (task 4.1).
//!
//! Submodules:
//! - `text`: Criterion text masking and whole-word scanning.
//! - `ears`: EARS shape analysis and classification.
//! - `annotations`: Annotation reading over requirements documents.
//! - `guard`: Guard and effect expression parser.
//! - `terms`: Term scope resolution, checking and suggestions.
//! - `finitize`: Finitization, typed guards, domains and conflicts.
//! - `bdd`: ROBDD encoding over `biodivine-lib-bdd`.
//! - `contradiction`: Incompatible pair selection and contradiction search.

pub mod annotations;
pub mod bdd;
pub mod contradiction;
pub mod ears;
pub mod finitize;
pub mod guard;
pub mod terms;
pub mod text;
