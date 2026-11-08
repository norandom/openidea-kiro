//! Source adapter over host read_file (task 2.4).
//!
//! Implements `kiro_core::source::Source` over `HostApi::read_file`, enforcing lexical
//! path guards before contacting the host and logging read failures at debug level.

use crate::host::{HostApi, LogLevel};

/// Implements `kiro_core::source::Source` over the host API port.
pub struct HostSource<'h> {
    host: &'h dyn HostApi,
}

impl<'h> HostSource<'h> {
    pub fn new(host: &'h dyn HostApi) -> Self {
        Self { host }
    }
}

impl kiro_core::source::Source for HostSource<'_> {
    fn read(&self, project_relative_path: &str) -> Option<String> {
        if project_relative_path.is_empty()
            || project_relative_path.starts_with('/')
            || project_relative_path.starts_with('\\')
            || (project_relative_path.len() >= 2
                && project_relative_path
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && project_relative_path.as_bytes().get(1) == Some(&b':'))
            || project_relative_path.contains('\\')
            || project_relative_path.split('/').any(|seg| seg == "..")
        {
            return None;
        }

        match self.host.read_file(project_relative_path) {
            Ok(content) => Some(content),
            Err(reason) => {
                self.host.log(
                    LogLevel::Debug,
                    &format!("read {project_relative_path}: {reason}"),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{FakeHost, HostEvent};
    use kiro_core::source::Source;

    #[test]
    fn guarded_paths_cause_no_host_call() {
        let host = FakeHost::new().with_root(r"C:\proj");
        let source = HostSource::new(&host);

        assert_eq!(source.read(""), None);
        assert_eq!(source.read("/abs/path"), None);
        assert_eq!(source.read(r"\abs\path"), None);
        assert_eq!(source.read(r"C:\abs\path"), None);
        assert_eq!(source.read("path\\with\\backslash"), None);
        assert_eq!(source.read("src/../outside.rs"), None);

        // Prove no read events were recorded on the host
        assert!(host.reads().is_empty());
    }

    #[test]
    fn host_read_failure_logs_debug_and_returns_none() {
        let host = FakeHost::new().with_root(r"C:\proj");
        let source = HostSource::new(&host);

        let res = source.read("missing.txt");
        assert_eq!(res, None);

        assert_eq!(host.reads(), vec!["missing.txt"]);
        let events = host.events();
        assert_eq!(events.len(), 2);
        assert_eq!(
            events.get(1),
            Some(&HostEvent::Log(
                LogLevel::Debug,
                "read missing.txt: missing.txt: no such file".into()
            ))
        );
    }

    #[test]
    fn successful_read_returns_content() {
        let host = FakeHost::new()
            .with_root(r"C:\proj")
            .with_file("src/lib.rs", "pub fn hello() {}");
        let source = HostSource::new(&host);

        let res = source.read("src/lib.rs");
        assert_eq!(res.as_deref(), Some("pub fn hello() {}"));
        assert_eq!(host.reads(), vec!["src/lib.rs"]);
    }
}
