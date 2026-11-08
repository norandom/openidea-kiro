//! WASM WIT bindings for the OpenIDEA host plugin API (task 3.5).

#![allow(
    unsafe_code,
    clippy::all,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::pedantic
)]

wit_bindgen::generate!({
    world: "plugin",
    path: "../../../openidea/wit",
});

use openidea::plugin::host;
use openidea::plugin::types::{LogLevel as WitLogLevel, NotificationLevel as WitLevel};

pub struct WasmHost;

impl crate::host::HostApi for WasmHost {
    fn log(&self, level: crate::host::LogLevel, message: &str) {
        let wit_level = match level {
            crate::host::LogLevel::Trace => WitLogLevel::Trace,
            crate::host::LogLevel::Debug => WitLogLevel::Debug,
            crate::host::LogLevel::Info => WitLogLevel::Info,
            crate::host::LogLevel::Warn => WitLogLevel::Warn,
            crate::host::LogLevel::Error => WitLogLevel::Error,
        };
        host::log(wit_level, message);
    }

    fn register_command(&self, id: &str, title: &str, category: Option<&str>) {
        host::register_command(id, title, category);
    }

    fn show_notification(&self, level: crate::host::Level, title: &str, body: &str) {
        let wit_level = match level {
            crate::host::Level::Info => WitLevel::Info,
            crate::host::Level::Warning => WitLevel::Warning,
            crate::host::Level::Error => WitLevel::Error,
        };
        host::show_notification(wit_level, title, body);
    }

    fn workspace_root(&self) -> Option<String> {
        host::workspace_root()
    }

    fn active_file(&self) -> Option<String> {
        host::active_file()
    }

    fn read_file(&self, path: &str) -> Result<String, String> {
        host::read_file(path)
    }

    fn insert_text(&self, text: &str) {
        host::insert_text(text);
    }

    fn replace_selection(&self, text: &str) {
        host::replace_selection(text);
    }
}

struct Kiro;

impl Guest for Kiro {
    fn activate(ctx: Context) -> Result<(), String> {
        crate::commands::activate(&WasmHost, &ctx.permissions)
    }

    fn deactivate() {
        crate::commands::deactivate(&WasmHost);
    }

    fn run_command(id: String, arg: Option<String>) -> Result<Option<String>, String> {
        crate::commands::run_command(&WasmHost, &id, arg.as_deref())
    }
}

export!(Kiro with_types_in self);
