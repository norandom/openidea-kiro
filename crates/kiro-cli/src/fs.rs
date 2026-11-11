//! Filesystem adapter and confinement boundary for `kiro-cli`.
//!
//! This is the only module in `kiro-cli` permitted to name `std::fs`.

use std::fmt;
use std::path::{Path, PathBuf};

use kiro_core::source::Source;

use crate::exit::Failure;

/// Maximum size in bytes accepted by the read port (16 MiB).
pub const MAX_READ_BYTES: u64 = 16 * 1024 * 1024;

/// Fixed project-relative directory containing specs.
pub const SPECS_DIR: &str = ".kiro/specs";

/// The canonicalised project root and its user-facing display string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    path: PathBuf,
    pub display: String,
}

impl ProjectRoot {
    /// Constructs a project root from canonical path and display string.
    #[must_use]
    pub fn new(canonical: PathBuf, display: impl Into<String>) -> Self {
        Self {
            path: canonical,
            display: display.into(),
        }
    }

    /// Appends a project-relative path to the canonical root.
    #[must_use]
    pub fn join(&self, project_relative: &str) -> PathBuf {
        self.path.join(project_relative)
    }

    /// Returns a reference to the canonical root path.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Canonicalises `arg` and verifies it exists and is a directory.
///
/// Returns `Failure` naming `arg` when unresolvable or not a directory.
pub fn resolve_root(arg: &Path) -> Result<ProjectRoot, Failure> {
    let canonical = match arg.canonicalize() {
        Ok(c) => c,
        Err(_) => {
            return Err(Failure::new(format!(
                "project root `{}` is not a directory",
                display_path(arg)
            )));
        }
    };
    if !canonical.is_dir() {
        return Err(Failure::new(format!(
            "project root `{}` is not a directory",
            display_path(arg)
        )));
    }
    Ok(ProjectRoot {
        path: canonical,
        display: display_path(arg),
    })
}

/// Reason why an engine read was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Absolute,
    ParentSegment,
    NotFile,
    TooLarge(u64),
    NotUtf8,
    Io(String),
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute => write!(f, "absolute path"),
            Self::ParentSegment => write!(f, "parent directory segment (..)"),
            Self::NotFile => write!(f, "not a regular file"),
            Self::TooLarge(size) => {
                write!(
                    f,
                    "file size {size} exceeds limit of {MAX_READ_BYTES} bytes"
                )
            }
            Self::NotUtf8 => write!(f, "not valid UTF-8"),
            Self::Io(err) => write!(f, "{err}"),
        }
    }
}

/// Root-confined [`Source`] backed by the local filesystem.
#[derive(Debug, Clone)]
pub struct FsSource {
    root: ProjectRoot,
    verbose: bool,
}

impl FsSource {
    /// Creates a new filesystem source rooted at `root`.
    #[must_use]
    pub fn new(root: ProjectRoot, verbose: bool) -> Self {
        Self { root, verbose }
    }

    /// Access the underlying project root.
    #[must_use]
    pub fn root(&self) -> &ProjectRoot {
        &self.root
    }

    /// Pure lexical guard on `project_relative` without filesystem access.
    ///
    /// Rejects:
    /// - `Path::is_absolute()`
    /// - Leading `/` or `\`
    /// - Drive prefixes like `C:`
    /// - Any parent segment `..`
    ///
    /// Accepts all other relative paths and returns `root.join(path)`.
    pub fn guard(&self, project_relative: &str) -> Result<PathBuf, Refusal> {
        if project_relative.starts_with('/') || project_relative.starts_with('\\') {
            return Err(Refusal::Absolute);
        }
        let bytes = project_relative.as_bytes();
        if matches!((bytes.first(), bytes.get(1)), (Some(first), Some(b':')) if first.is_ascii_alphabetic())
        {
            return Err(Refusal::Absolute);
        }
        if Path::new(project_relative).is_absolute() {
            return Err(Refusal::Absolute);
        }
        if project_relative.split(['/', '\\']).any(|seg| seg == "..") {
            return Err(Refusal::ParentSegment);
        }
        Ok(self.root.join(project_relative))
    }

    /// Checked read performing lexical guard, metadata check, size limit,
    /// read, and UTF-8 verification.
    pub fn read_checked(&self, project_relative: &str) -> Result<String, Refusal> {
        let path = self.guard(project_relative)?;
        let metadata = std::fs::metadata(&path).map_err(|e| Refusal::Io(e.to_string()))?;
        if !metadata.is_file() {
            return Err(Refusal::NotFile);
        }
        let len = metadata.len();
        if len > MAX_READ_BYTES {
            return Err(Refusal::TooLarge(len));
        }
        let bytes = std::fs::read(&path).map_err(|e| Refusal::Io(e.to_string()))?;
        String::from_utf8(bytes).map_err(|_| Refusal::NotUtf8)
    }
}

impl Source for FsSource {
    fn read(&self, project_relative_path: &str) -> Option<String> {
        match self.read_checked(project_relative_path) {
            Ok(content) => Some(content),
            Err(refusal) => {
                if self.verbose {
                    let verb = match refusal {
                        Refusal::Absolute | Refusal::ParentSegment => "refused",
                        Refusal::NotFile
                        | Refusal::TooLarge(_)
                        | Refusal::NotUtf8
                        | Refusal::Io(_) => "failed",
                    };
                    eprintln!("read {verb}: {project_relative_path}: {refusal}");
                }
                None
            }
        }
    }
}

/// Discovers spec directories directly under `.kiro/specs`.
///
/// A spec directory must be a non-hidden directory containing `spec.json` or
/// `requirements.md`. Returned spec names are sorted in ascending order.
///
/// Fails when `.kiro/specs` is missing or not a directory.
pub fn discover_features(root: &ProjectRoot) -> Result<Vec<String>, Failure> {
    let specs_dir = root.join(SPECS_DIR);
    if !specs_dir.is_dir() {
        return Err(Failure::new(format!(
            "no .kiro/specs directory under {}",
            root.display
        )));
    }
    let entries = std::fs::read_dir(&specs_dir).map_err(|e| {
        Failure::new(format!(
            "failed to read directory `{}`: {e}",
            display_path(&specs_dir)
        ))
    })?;

    let mut features = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| {
            Failure::new(format!(
                "failed to read directory entry in `{}`: {e}",
                display_path(&specs_dir)
            ))
        })?;
        let file_type = entry.file_type().map_err(|e| {
            Failure::new(format!(
                "failed to read file type of `{}`: {e}",
                display_path(&entry.path())
            ))
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let dir_path = entry.path();
        if dir_path.join("spec.json").is_file() || dir_path.join("requirements.md").is_file() {
            features.push(name);
        }
    }
    features.sort();
    Ok(features)
}

/// Reads an explicit argument file without project-root confinement.
///
/// Strips a leading UTF-8 Byte Order Mark (BOM) if present.
pub fn read_argument_file(path: &Path) -> Result<String, Failure> {
    let bytes = std::fs::read(path)
        .map_err(|e| Failure::new(format!("failed to read file `{}`: {e}", display_path(path))))?;
    let mut text = String::from_utf8(bytes).map_err(|e| {
        Failure::new(format!(
            "file `{}` is not valid UTF-8: {e}",
            display_path(path)
        ))
    })?;
    if let Some(stripped) = text.strip_prefix('\u{FEFF}') {
        text = stripped.to_string();
    }
    Ok(text)
}

/// Checks whether an output file already exists.
#[must_use]
pub fn output_exists(path: &Path) -> bool {
    path.exists()
}

/// Writes `text` to `path`, wrapping any I/O error into a [`Failure`].
pub fn write_output(path: &Path, text: &str) -> Result<(), Failure> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, text).map_err(|e| {
        Failure::new(format!(
            "failed to write output `{}`: {e}",
            display_path(path)
        ))
    })
}

/// Recursively creates a directory and all of its parent components.
pub fn create_dir_all(path: &Path) -> Result<(), Failure> {
    std::fs::create_dir_all(path).map_err(|e| {
        Failure::new(format!(
            "failed to create directory `{}`: {e}",
            display_path(path)
        ))
    })
}

/// Formats a path with forward slashes on all platforms.
#[must_use]
pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Reads an optional project file: `None` when it does not exist, a
/// failure when it exists and cannot be read as UTF-8 text.
pub fn read_optional(path: &Path) -> Result<Option<String>, Failure> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(Failure::new(format!(
            "failed to read `{}`: {error}",
            display_path(path)
        ))),
    }
}

/// Resolves a telemetry directory and refuses one inside the project.
///
/// A relative path is taken from the current directory. The directory need
/// not exist yet. The path is resolved component by component: as long as
/// the part walked so far exists it is made canonical, so a symbolic link
/// is followed; from the first missing component on, names are appended and
/// `..` removes the last one. The comparison with the project root is
/// therefore not fooled by `..`, by a link, or by a `..` that passes through
/// a directory that does not exist (which Windows resolves lexically and
/// other systems do not resolve at all).
pub fn telemetry_dir(root: &ProjectRoot, dir: &Path) -> Result<PathBuf, Failure> {
    use std::path::Component;
    let absolute = if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| Failure::new(format!("failed to read the current directory: {e}")))?
            .join(dir)
    };
    let mut resolved = PathBuf::new();
    // True while `resolved` names something that exists and is canonical.
    let mut real = true;
    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => resolved.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            Component::Normal(name) => {
                resolved.push(name);
                // Once a component is missing nothing after it is looked up.
                real = real && make_canonical(&mut resolved);
            }
        }
    }
    if resolved.starts_with(root.path()) {
        return Err(Failure::new(format!(
            "telemetry directory `{}` is inside the project; choose a directory outside the repository",
            display_path(dir)
        )));
    }
    Ok(resolved)
}

/// Replaces `path` by its canonical form when it exists. Returns whether
/// it did; a path that does not exist is left as it is.
fn make_canonical(path: &mut PathBuf) -> bool {
    match path.canonicalize() {
        Ok(canonical) => {
            *path = canonical;
            true
        }
        Err(_) => false,
    }
}

/// Appends `line` and a newline to `path` in one write, creating the file
/// and its directory when missing. One write per line keeps concurrent
/// runs from interleaving within a line.
pub fn append_line(path: &Path, line: &str) -> Result<(), Failure> {
    use std::io::Write as _;
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|e| {
            Failure::new(format!(
                "failed to create telemetry directory `{}`: {e}",
                display_path(parent)
            ))
        })?;
    }
    let mut record = String::with_capacity(line.len().saturating_add(1));
    record.push_str(line);
    record.push('\n');
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(record.as_bytes()))
        .map_err(|e| {
            Failure::new(format!(
                "failed to write telemetry file `{}`: {e}",
                display_path(path)
            ))
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn guard_lexical_checks() {
        let root_dir = tempdir().unwrap();
        let canonical = root_dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");
        let source = FsSource::new(root, false);

        // Absolute / drive letters
        assert_eq!(source.guard("/abs/path"), Err(Refusal::Absolute));
        assert_eq!(source.guard("\\abs\\path"), Err(Refusal::Absolute));
        assert_eq!(source.guard("C:/path"), Err(Refusal::Absolute));
        assert_eq!(source.guard("c:\\path"), Err(Refusal::Absolute));

        // Parent segments
        assert_eq!(source.guard(".."), Err(Refusal::ParentSegment));
        assert_eq!(source.guard("../foo"), Err(Refusal::ParentSegment));
        assert_eq!(source.guard("foo/.."), Err(Refusal::ParentSegment));
        assert_eq!(source.guard("foo/../bar"), Err(Refusal::ParentSegment));
        assert_eq!(source.guard("foo\\..\\bar"), Err(Refusal::ParentSegment));

        // Valid relative paths
        assert!(source.guard("foo/bar").is_ok());
        assert!(source.guard("foo/./bar").is_ok());
        assert!(source.guard("foo..bar").is_ok());
        assert!(source.guard(".kiro/specs/test/spec.json").is_ok());
    }

    #[test]
    fn resolve_root_valid_and_invalid() {
        let dir = tempdir().unwrap();
        let valid = resolve_root(dir.path()).unwrap();
        assert_eq!(valid.display, display_path(dir.path()));

        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "hello").unwrap();
        let err_file = resolve_root(&file_path).unwrap_err();
        assert!(err_file.0.contains("is not a directory"));

        let non_existent = dir.path().join("does-not-exist");
        let err_missing = resolve_root(&non_existent).unwrap_err();
        assert!(err_missing.0.contains("is not a directory"));
    }

    #[test]
    fn read_checked_file_types_and_encoding() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, ".");
        let source = FsSource::new(root, false);

        // Subdirectory is not a file
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        assert_eq!(source.read_checked("subdir"), Err(Refusal::NotFile));

        // Valid UTF-8 file
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "sample text").unwrap();
        assert_eq!(source.read_checked("test.txt").unwrap(), "sample text");
        assert_eq!(source.read("test.txt"), Some("sample text".to_string()));

        // Non-UTF-8 bytes
        let bad_utf8 = dir.path().join("bad.bin");
        fs::write(&bad_utf8, [0xFF, 0xFE, 0xFD]).unwrap();
        assert_eq!(source.read_checked("bad.bin"), Err(Refusal::NotUtf8));
        assert_eq!(source.read("bad.bin"), None);

        // Oversized file (> 16 MiB)
        let large_file = dir.path().join("large.bin");
        let file = fs::File::create(&large_file).unwrap();
        file.set_len(MAX_READ_BYTES + 1).unwrap();
        assert_eq!(
            source.read_checked("large.bin"),
            Err(Refusal::TooLarge(MAX_READ_BYTES + 1))
        );
        assert_eq!(source.read("large.bin"), None);
    }

    #[test]
    fn discover_features_filtering_and_order() {
        let dir = tempdir().unwrap();
        let canonical = dir.path().canonicalize().unwrap();
        let root = ProjectRoot::new(canonical, "my-root");

        // Missing .kiro/specs fails
        let err = discover_features(&root).unwrap_err();
        assert_eq!(err.0, "no .kiro/specs directory under my-root");

        let specs = dir.path().join(".kiro").join("specs");
        fs::create_dir_all(&specs).unwrap();

        // Empty specs directory
        let discovered = discover_features(&root).unwrap();
        assert!(discovered.is_empty());

        // Add various entries
        // 1. dot directory (ignored)
        fs::create_dir(specs.join(".hidden")).unwrap();
        fs::write(specs.join(".hidden").join("spec.json"), "{}").unwrap();

        // 2. regular file directly in .kiro/specs (ignored)
        fs::write(specs.join("stray.txt"), "stray").unwrap();

        // 3. directory without spec.json or requirements.md (ignored)
        fs::create_dir(specs.join("empty-feature")).unwrap();

        // 4. feature 'beta' with requirements.md
        fs::create_dir(specs.join("beta")).unwrap();
        fs::write(specs.join("beta").join("requirements.md"), "# Requirements").unwrap();

        // 5. feature 'alpha' with spec.json
        fs::create_dir(specs.join("alpha")).unwrap();
        fs::write(specs.join("alpha").join("spec.json"), "{}").unwrap();

        let discovered = discover_features(&root).unwrap();
        assert_eq!(discovered, vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[test]
    fn read_argument_file_and_bom_stripping() {
        let dir = tempdir().unwrap();
        let bom_file = dir.path().join("bom.txt");
        let content = "\u{FEFF}hello with bom";
        fs::write(&bom_file, content).unwrap();

        let read = read_argument_file(&bom_file).unwrap();
        assert_eq!(read, "hello with bom");
        assert!(!read.starts_with('\u{FEFF}'));

        let plain_file = dir.path().join("plain.txt");
        fs::write(&plain_file, "plain text").unwrap();
        let read_plain = read_argument_file(&plain_file).unwrap();
        assert_eq!(read_plain, "plain text");
    }

    #[test]
    fn output_helpers() {
        let dir = tempdir().unwrap();
        let out_path = dir.path().join("output.txt");

        assert!(!output_exists(&out_path));
        write_output(&out_path, "generated text").unwrap();
        assert!(output_exists(&out_path));
        let read = read_argument_file(&out_path).unwrap();
        assert_eq!(read, "generated text");
    }
}
