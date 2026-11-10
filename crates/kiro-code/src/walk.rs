//! Confined source reads (code-constraints 1.6, 9.6).
//!
//! The walk stays inside the project root, skips build and dependency
//! directories, nested repositories and worktrees, and everything the
//! project's `.gitignore` excludes,
//! follows no link that leaves the root, and reads each file
//! once into an immutable bundle that every analyzer shares.

use std::path::{Path, PathBuf};

/// Directories never descended into.
pub const DEFAULT_EXCLUDED_DIRS: &[&str] = &[".git", "target", "node_modules", "dist"];
/// Largest source file read; a larger one is listed as not measured.
pub const MAX_SOURCE_BYTES: u64 = 16 * 1024 * 1024;

/// One source file: project-relative path with `/` separators and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceText {
    pub path: String,
    pub text: String,
}

/// A file that was selected but could not be read as source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    pub path: String,
    pub reason: String,
}

/// The source texts of one run, sorted by path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceBundle {
    pub files: Vec<SourceText>,
    pub unreadable: Vec<Unreadable>,
}

/// Why the bundle could not be read at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleError(pub String);

impl std::fmt::Display for BundleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BundleError {}

fn relative(root: &Path, path: &Path) -> Option<String> {
    let rest = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rest
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// Reads every file under `root` for which `wanted` is true.
///
/// `root` must be canonical. `wanted` receives the project-relative path
/// and decides selection; files in excluded directories are never offered.
/// A link whose target lies outside the root is skipped and listed.
pub fn read_bundle(
    root: &Path,
    wanted: &dyn Fn(&str) -> bool,
) -> Result<SourceBundle, BundleError> {
    let mut bundle = SourceBundle::default();
    // The `ignore` crate walks and applies the project's `.gitignore` and
    // `.ignore` files. Only files inside the root decide: the user's global
    // Git settings and ignore files above the root are not read, so the
    // same tree gives the same bundle on every machine, with or without a
    // `.git` directory.
    let walk = ignore::WalkBuilder::new(root)
        .hidden(false)
        .parents(false)
        .git_global(false)
        .git_exclude(false)
        .require_git(false)
        .follow_links(false)
        .filter_entry(|entry| {
            let excluded = entry
                .file_name()
                .to_str()
                .is_some_and(|name| DEFAULT_EXCLUDED_DIRS.contains(&name));
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            // A directory below the root with its own `.git` is another
            // repository or a worktree: Git does not count its files as
            // part of this project, and neither does the walk.
            let nested = is_dir && entry.depth() > 0 && entry.path().join(".git").exists();
            !(nested || excluded && is_dir)
        })
        .build();
    for entry in walk {
        let entry =
            entry.map_err(|error| BundleError(format!("cannot walk the project: {error}")))?;
        let path: PathBuf = entry.into_path();
        let Some(project_path) = relative(root, &path) else {
            continue;
        };
        // Resolve links before trusting the entry.
        let Ok(canonical) = path.canonicalize() else {
            continue;
        };
        if !canonical.starts_with(root) {
            bundle.unreadable.push(Unreadable {
                path: project_path,
                reason: "link leaves the project root".to_string(),
            });
            continue;
        }
        // A directory is walked where it stands; a link to one is not
        // followed, so no file is read twice.
        if canonical.is_dir() || !wanted(&project_path) {
            continue;
        }
        let too_large = std::fs::metadata(&canonical)
            .map(|metadata| metadata.len() > MAX_SOURCE_BYTES)
            .unwrap_or(false);
        if too_large {
            bundle.unreadable.push(Unreadable {
                path: project_path,
                reason: "file is larger than the 16 MiB source limit".to_string(),
            });
            continue;
        }
        match std::fs::read_to_string(&canonical) {
            Ok(text) => bundle.files.push(SourceText {
                path: project_path,
                text,
            }),
            Err(error) => bundle.unreadable.push(Unreadable {
                path: project_path,
                reason: format!("cannot read as UTF-8 text: {error}"),
            }),
        }
    }
    bundle.files.sort_by(|a, b| a.path.cmp(&b.path));
    bundle.unreadable.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(bundle)
}
