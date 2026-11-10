//! The before side from Git (code-constraints 5.4, 11.6).
//!
//! Reads what changed between a revision and the working tree, and the
//! text the changed files had at that revision. Git is only read: no
//! checkout, no fetch, no write. Every call passes its arguments
//! separately; nothing is interpolated into a shell.

use std::path::Path;
use std::process::Command;

use crate::walk::{SourceBundle, SourceText, Unreadable};

/// What changed between a revision and the working tree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChangeSet {
    /// The revision, resolved to a full commit id.
    pub revision: String,
    /// Paths that exist now and differ from the revision (modified, added,
    /// renamed to, type-changed), sorted.
    pub changed: Vec<String>,
    /// Paths that existed at the revision and are gone, sorted.
    pub deleted: Vec<String>,
    /// `(old path, new path)` of the renames Git detected.
    pub renames: Vec<(String, String)>,
    /// The text of the changed and deleted files as they were at the
    /// revision, under their old paths.
    pub before: SourceBundle,
}

/// Why the before side could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitError(pub String);

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for GitError {}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|error| GitError(format!("git could not be started: {error}")))?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        let message = String::from_utf8_lossy(&output.stderr);
        Err(GitError(message.trim().replace('\n', " ")))
    }
}

/// Parses `git diff --name-status -z`: records are NUL-separated; a rename
/// or copy record is followed by two paths, every other by one.
fn parse_name_status(raw: &[u8], changes: &mut ChangeSet) {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0').filter(|field| !field.is_empty());
    while let Some(status) = fields.next() {
        let kind = status.chars().next().unwrap_or('M');
        match kind {
            'R' | 'C' => {
                let (Some(old), Some(new)) = (fields.next(), fields.next()) else {
                    break;
                };
                changes.changed.push(new.to_string());
                if kind == 'R' {
                    changes.renames.push((old.to_string(), new.to_string()));
                }
            }
            'D' => {
                if let Some(path) = fields.next() {
                    changes.deleted.push(path.to_string());
                }
            }
            _ => {
                if let Some(path) = fields.next() {
                    changes.changed.push(path.to_string());
                }
            }
        }
    }
    changes.changed.sort();
    changes.changed.dedup();
    changes.deleted.sort();
    changes.renames.sort();
}

/// Reads the changes of the tracked files under `root` against `revision`
/// and the before text of every changed or deleted file `wanted` accepts.
///
/// Staged and unstaged changes of tracked files are covered. A file Git
/// does not track is not part of the diff; select it with `--files` or
/// `--all`. A missing revision (for example in a shallow clone) is an
/// error that says so; nothing is fetched.
pub fn read_before(
    root: &Path,
    revision: &str,
    wanted: &dyn Fn(&str) -> bool,
) -> Result<ChangeSet, GitError> {
    if revision.is_empty() || revision.starts_with('-') {
        return Err(GitError(format!("`{revision}` is not a revision")));
    }
    let commit = format!("{revision}^{{commit}}");
    let resolved = git(root, &["rev-parse", "--verify", "--quiet", &commit]).map_err(|_| {
        GitError(format!(
            "revision `{revision}` was not found; in a shallow clone, fetch more history first"
        ))
    })?;
    let mut changes = ChangeSet {
        revision: String::from_utf8_lossy(&resolved).trim().to_string(),
        ..ChangeSet::default()
    };
    // `--relative` limits the diff to `root` and reports paths relative to it.
    let raw = git(
        root,
        &[
            "diff",
            "--name-status",
            "-z",
            "--find-renames",
            "--relative",
            &changes.revision,
            "--",
        ],
    )?;
    parse_name_status(&raw, &mut changes);

    let renamed_to: Vec<&str> = changes
        .renames
        .iter()
        .map(|(_, new)| new.as_str())
        .collect();
    let mut old_paths: Vec<String> = changes
        .changed
        .iter()
        .filter(|path| !renamed_to.contains(&path.as_str()))
        .cloned()
        .chain(changes.renames.iter().map(|(old, _)| old.clone()))
        .chain(changes.deleted.iter().cloned())
        .collect();
    old_paths.sort();
    old_paths.dedup();
    for path in old_paths {
        if !wanted(&path) {
            continue;
        }
        let object = format!("{}:./{path}", changes.revision);
        // An added file has no before text; that is not an error.
        let Ok(bytes) = git(root, &["show", &object]) else {
            continue;
        };
        match String::from_utf8(bytes) {
            Ok(text) => changes.before.files.push(SourceText { path, text }),
            Err(_) => changes.before.unreadable.push(Unreadable {
                path,
                reason: "not UTF-8 text at the revision".to_string(),
            }),
        }
    }
    Ok(changes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn name_status_records_are_parsed() {
        let raw = b"M\0src/a.rs\0A\0src/new.rs\0D\0src/gone.rs\0R087\0src/old.rs\0src/moved.rs\0T\0link\0";
        let mut changes = ChangeSet::default();
        parse_name_status(raw, &mut changes);
        assert_eq!(
            changes.changed,
            ["link", "src/a.rs", "src/moved.rs", "src/new.rs"]
        );
        assert_eq!(changes.deleted, ["src/gone.rs"]);
        assert_eq!(
            changes.renames,
            [("src/old.rs".to_string(), "src/moved.rs".to_string())]
        );
    }

    #[test]
    fn a_truncated_rename_record_is_ignored() {
        let mut changes = ChangeSet::default();
        parse_name_status(b"R100\0only-one\0", &mut changes);
        assert!(changes.changed.is_empty());
        assert!(changes.renames.is_empty());
    }

    #[test]
    fn an_option_like_revision_is_refused_before_git_runs() {
        let error = read_before(Path::new("."), "--output=x", &|_| true).unwrap_err();
        assert!(error.0.contains("is not a revision"));
        assert!(read_before(Path::new("."), "", &|_| true).is_err());
    }
}
