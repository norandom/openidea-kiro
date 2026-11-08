//! OpenIDEA Kiro Plugin (`openidea-plugin-kiro`).
//!
//! A sandboxed WebAssembly component (`wasm32-wasip2`) that adapts the pure
//! `kiro-core` engine to OpenIDEA's plugin host API. It provides spec-driven
//! development workflows inside the IDE: discovering known specs without directory
//! listing, resolving current specs, executing commands, synchronizing per-spec
//! variants, and delivering results.
//!
//! # Command Grammar
//!
//! Commands exposed to the host follow the grammar:
//!
//! ```text
//! <id>          ::= <group> "." <name> [ "." <spec> | ".insert" ]
//! <group>       ::= "ws" | "nav" | "viz" | "vocab" | "check"
//! <name>        ::= [a-z0-9][a-z0-9_-]*
//! <spec>        ::= [a-z0-9][a-z0-9._-]* (excluding "insert")
//! ```
//!
//! When invoked with an argument string, arguments are tokenized by whitespace:
//! - The `insert` keyword in any token position sets the insertion flag and is removed.
//! - For commands with `positional = Positional::Spec`, the first remaining token names the target spec.
//! - All other tokens pass through to the owning feature module.
//!
//! # Output Channels
//!
//! The plugin provides a three-tier output contract:
//!
//! 1. **Notification Balloon**: Immediate UI toast summary, clipped below 16 KiB (at 15 KiB)
//!    on line boundaries. When clipped, insert-capable commands append a trailer:
//!    `… N more lines — run <id>.insert for the full text`.
//! 2. **Editor Insertion**: For insert-capable commands when the `insert` flag is set,
//!    the full text payload is inserted into the active editor buffer, clipped below 1 MiB
//!    (at 1,047,552 bytes) on line boundaries with an omitted-lines trailer.
//! 3. **CLI Durable Record**: Companion CLI tools produce durable structured JSON records
//!    for automated CI/CD and pre-commit hooks.
//!
//! # Host Resource Limits
//!
//! - **Notification Budget**: 16 KiB hard ceiling; clipped at 15 KiB.
//! - **Title Budget**: 256 bytes ceiling; cut on UTF-8 character boundaries.
//! - **Editor Insertion Budget**: 1 MiB hard ceiling; clipped at 1,047,552 bytes.
//! - **Command Limit**: 256 total registered commands per plugin instance.
//! - **Timeout**: Host cancels executions taking longer than 5 seconds.

#![deny(unsafe_code)]

pub mod args;
pub mod commands;
pub mod context;
pub mod host;
pub mod host_source;
pub mod module;
pub mod nav;
pub mod present;
pub mod registry;
pub mod vocab;
pub mod ws;

#[cfg(target_arch = "wasm32")]
mod bindings;
