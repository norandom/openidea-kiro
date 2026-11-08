//! Host API port and test fake (task 1.2).
//!
//! Provides the `HostApi` trait defining the interface to the OpenIDEA IDE host
//! functions (excluding language/theme registrations) and `FakeHost` for in-memory
//! native testing without launching the IDE.

use std::cell::RefCell;
use std::collections::BTreeMap;

/// Host logging severity level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Notification severity level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Info,
    Warning,
    Error,
}

/// Required permissions declared in plugin manifest and verified at activation.
pub const REQUIRED_PERMISSIONS: [&str; 3] = ["project-read", "editor-read", "editor-write"];

/// The port every plugin module uses to reach the IDE host functions.
pub trait HostApi {
    fn log(&self, level: LogLevel, message: &str);
    fn register_command(&self, id: &str, title: &str, category: Option<&str>);
    fn show_notification(&self, level: Level, title: &str, body: &str);
    fn workspace_root(&self) -> Option<String>;
    fn active_file(&self) -> Option<String>;
    fn read_file(&self, path: &str) -> Result<String, String>;
    fn insert_text(&self, text: &str);
    fn replace_selection(&self, text: &str);
}

/// Events recorded by `FakeHost`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostEvent {
    Log(LogLevel, String),
    Register {
        id: String,
        title: String,
        category: Option<String>,
    },
    Notify {
        level: Level,
        title: String,
        body: String,
    },
    Insert(String),
    Replace(String),
    Read(String),
}

/// In-memory fake host for native tests.
#[derive(Default)]
pub struct FakeHost {
    pub root: Option<String>,
    pub active: Option<String>,
    pub files: BTreeMap<String, String>,
    events: RefCell<Vec<HostEvent>>,
}

impl FakeHost {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_root(mut self, root: &str) -> Self {
        self.root = Some(root.to_owned());
        self
    }

    pub fn with_active(mut self, path: &str) -> Self {
        self.active = Some(path.to_owned());
        self
    }

    pub fn with_file(mut self, relative: &str, text: &str) -> Self {
        let normalized = relative.replace('\\', "/");
        let clean = normalized.strip_prefix("./").unwrap_or(&normalized);
        self.files.insert(clean.to_owned(), text.to_owned());
        self
    }

    pub fn events(&self) -> Vec<HostEvent> {
        self.events.borrow().clone()
    }

    pub fn registered(&self) -> BTreeMap<String, (String, Option<String>)> {
        let mut map = BTreeMap::new();
        for ev in self.events.borrow().iter() {
            if let HostEvent::Register {
                id,
                title,
                category,
            } = ev
            {
                map.insert(id.clone(), (title.clone(), category.clone()));
            }
        }
        map
    }

    pub fn notifications(&self) -> Vec<(Level, String, String)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|ev| match ev {
                HostEvent::Notify { level, title, body } => {
                    Some((*level, title.clone(), body.clone()))
                }
                _ => None,
            })
            .collect()
    }

    pub fn inserts(&self) -> Vec<String> {
        self.events
            .borrow()
            .iter()
            .filter_map(|ev| match ev {
                HostEvent::Insert(text) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    pub fn reads(&self) -> Vec<String> {
        self.events
            .borrow()
            .iter()
            .filter_map(|ev| match ev {
                HostEvent::Read(path) => Some(path.clone()),
                _ => None,
            })
            .collect()
    }
}

impl HostApi for FakeHost {
    fn log(&self, level: LogLevel, message: &str) {
        self.events
            .borrow_mut()
            .push(HostEvent::Log(level, message.to_owned()));
    }

    fn register_command(&self, id: &str, title: &str, category: Option<&str>) {
        self.events.borrow_mut().push(HostEvent::Register {
            id: id.to_owned(),
            title: title.to_owned(),
            category: category.map(str::to_owned),
        });
    }

    fn show_notification(&self, level: Level, title: &str, body: &str) {
        self.events.borrow_mut().push(HostEvent::Notify {
            level,
            title: title.to_owned(),
            body: body.to_owned(),
        });
    }

    fn workspace_root(&self) -> Option<String> {
        self.root.clone()
    }

    fn active_file(&self) -> Option<String> {
        self.active.clone()
    }

    fn read_file(&self, path: &str) -> Result<String, String> {
        self.events
            .borrow_mut()
            .push(HostEvent::Read(path.to_owned()));

        if path.is_empty() {
            return Err("empty path".to_owned());
        }

        let Some(root) = &self.root else {
            return Err("no project is open".to_owned());
        };

        let norm_path = path.replace('\\', "/");
        let norm_root = root.replace('\\', "/");
        let norm_root_trimmed = norm_root.trim_end_matches('/');

        let is_abs = norm_path.starts_with('/')
            || (norm_path.len() >= 2
                && norm_path
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && norm_path.as_bytes().get(1) == Some(&b':'));

        let rel_path = if is_abs {
            let lower_path = norm_path.to_ascii_lowercase();
            let lower_root = norm_root_trimmed.to_ascii_lowercase();
            if lower_path == lower_root {
                ""
            } else if lower_path.starts_with(&(lower_root.clone() + "/")) {
                let suffix = norm_path.get(norm_root_trimmed.len()..).unwrap_or_default();
                suffix.trim_start_matches('/')
            } else {
                return Err(format!(
                    "{path}: access outside the project root is not allowed"
                ));
            }
        } else {
            norm_path.as_str()
        };

        let rel_clean = rel_path.strip_prefix("./").unwrap_or(rel_path);

        // Disallow any .. segments
        if rel_clean.split('/').any(|seg| seg == "..") {
            return Err(format!(
                "{path}: access outside the project root is not allowed"
            ));
        }

        match self.files.get(rel_clean) {
            Some(content) => Ok(content.clone()),
            None => Err(format!("{path}: no such file")),
        }
    }

    fn insert_text(&self, text: &str) {
        self.events
            .borrow_mut()
            .push(HostEvent::Insert(text.to_owned()));
    }

    fn replace_selection(&self, text: &str) {
        self.events
            .borrow_mut()
            .push(HostEvent::Replace(text.to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fake_host_records_all_call_types() {
        let host = FakeHost::new()
            .with_root(r"C:\proj")
            .with_active(r"C:\proj\src\lib.rs")
            .with_file("src/lib.rs", "fn main() {}");

        host.log(LogLevel::Info, "startup");
        host.register_command("ws.specs", "Kiro: List Specs", Some("Kiro"));
        host.show_notification(Level::Info, "Title", "Body");
        host.insert_text("inserted text");
        host.replace_selection("replaced text");

        assert_eq!(host.workspace_root(), Some(r"C:\proj".to_owned()));
        assert_eq!(host.active_file(), Some(r"C:\proj\src\lib.rs".to_owned()));

        let read_res = host.read_file("src/lib.rs");
        assert_eq!(read_res.as_deref(), Ok("fn main() {}"));

        assert_eq!(host.inserts(), vec!["inserted text"]);
        assert_eq!(host.reads(), vec!["src/lib.rs"]);
        assert_eq!(
            host.notifications(),
            vec![(Level::Info, "Title".to_owned(), "Body".to_owned())]
        );
        let reg = host.registered();
        assert_eq!(
            reg.get("ws.specs"),
            Some(&("Kiro: List Specs".to_owned(), Some("Kiro".to_owned())))
        );
    }

    #[test]
    fn fake_host_read_file_outcomes() {
        let host = FakeHost::new()
            .with_root(r"C:\proj")
            .with_file("src/main.rs", "content");

        // Empty path
        assert_eq!(host.read_file(""), Err("empty path".to_owned()));

        // Backslash normalization
        assert_eq!(host.read_file(r"src\main.rs").as_deref(), Ok("content"));

        // Absolute path under root (Windows style, case-insensitive)
        assert_eq!(
            host.read_file(r"c:\PROJ\src\main.rs").as_deref(),
            Ok("content")
        );

        // Absolute path outside root
        assert_eq!(
            host.read_file(r"D:\other\x.rs"),
            Err(r"D:\other\x.rs: access outside the project root is not allowed".to_owned())
        );

        // Path with .. segment
        assert_eq!(
            host.read_file("src/../outside.rs"),
            Err("src/../outside.rs: access outside the project root is not allowed".to_owned())
        );

        // Missing file
        assert_eq!(
            host.read_file("src/missing.rs"),
            Err("src/missing.rs: no such file".to_owned())
        );
    }

    #[test]
    fn fake_host_read_without_root_fails() {
        let host = FakeHost::new();
        assert_eq!(
            host.read_file("src/main.rs"),
            Err("no project is open".to_owned())
        );
    }
}
