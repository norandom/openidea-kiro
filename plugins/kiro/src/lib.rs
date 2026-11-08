//! OpenIDEA Kiro Plugin: spec constraint checking, spec navigation, and
//! visualization inside the OpenIDEA editor.

#![deny(unsafe_code)]

pub mod args;
pub mod commands;
pub mod context;
pub mod host;
pub mod host_source;
pub mod module;
pub mod present;
pub mod registry;
pub mod ws;

#[cfg(target_arch = "wasm32")]
mod bindings;
