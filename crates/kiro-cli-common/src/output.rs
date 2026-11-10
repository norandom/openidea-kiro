//! Report destination and output routing (cli-contracts 1.5).
//!
//! A report goes to exactly one place: standard output, or a named file.
//! With a named file nothing of the report is copied to standard output.
//! Diagnostics never go through this module; they belong on standard error.

use std::io::Write;
use std::path::PathBuf;

use crate::Failure;

/// Where a report is written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    Stdout,
    File(PathBuf),
}

/// Writes a complete report document to its destination.
///
/// A file is replaced as a whole; its parent directory must exist. Any
/// failure is a [`Failure`] (exit code 2) naming the destination.
pub fn write_report(destination: &Destination, document: &[u8]) -> Result<(), Failure> {
    match destination {
        Destination::Stdout => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(document)
                .and_then(|()| stdout.flush())
                .map_err(|error| Failure::new(format!("failed to write report to stdout: {error}")))
        }
        Destination::File(path) => std::fs::write(path, document).map_err(|error| {
            Failure::new(format!(
                "failed to write report to {}: {error}",
                path.display()
            ))
        }),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn file_destination_receives_the_whole_document() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("report.json");
        write_report(&Destination::File(path.clone()), b"{\"a\":1}\n").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"a\":1}\n");
        write_report(&Destination::File(path.clone()), b"x").unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"x",
            "replaced, not appended"
        );
    }

    #[test]
    fn missing_parent_directory_is_a_failure_naming_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("missing").join("report.txt");
        let failure = write_report(&Destination::File(path), b"x").unwrap_err();
        assert!(failure.0.starts_with("failed to write report to "));
        assert!(failure.0.contains("report.txt"));
        assert!(!failure.0.contains('\n'));
    }
}
