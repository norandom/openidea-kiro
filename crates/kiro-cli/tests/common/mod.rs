//! Shared test harness for kiro-cli integration tests.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    dead_code
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output as ProcessOutput};

use tempfile::TempDir;

/// Output of running the `kiro-cli` binary.
#[derive(Debug, Clone)]
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    fn from_process(out: ProcessOutput) -> Self {
        Self {
            code: out.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n"),
            stderr: String::from_utf8_lossy(&out.stderr).replace("\r\n", "\n"),
        }
    }
}

/// Returns the path to the root workspace directory.
#[must_use]
pub fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Returns the path to a fixture inside `crates/kiro-core/fixtures/`.
#[must_use]
pub fn core_fixture_path(rel: &str) -> PathBuf {
    workspace_root()
        .join("crates")
        .join("kiro-core")
        .join("fixtures")
        .join(rel)
}

/// Returns the path to a fixture inside `crates/kiro-cli/fixtures/`.
#[must_use]
pub fn cli_fixture_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(rel)
}

/// Recursively copies a directory tree from `src` to `dst`.
pub fn copy_dir_all(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path);
        } else {
            std::fs::copy(&src_path, &dst_path).unwrap();
        }
    }
}

/// Sets up a fixture tree in a fresh temporary directory.
#[must_use]
pub fn setup_tree(fixture: &Path) -> (TempDir, PathBuf) {
    let temp = TempDir::new().unwrap();
    copy_dir_all(fixture, temp.path());
    let path = temp.path().to_path_buf();
    (temp, path)
}

/// Runs the compiled `kiro-cli` binary with the given arguments.
pub fn run_cli(args: &[&str]) -> Output {
    let bin = env!("CARGO_BIN_EXE_kiro-cli");
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap();
    Output::from_process(out)
}

/// Runs the compiled `kiro-cli` binary in `current_dir` with the given arguments.
pub fn run_cli_in(current_dir: &Path, args: &[&str]) -> Output {
    let bin = env!("CARGO_BIN_EXE_kiro-cli");
    let out = Command::new(bin)
        .current_dir(current_dir)
        .args(args)
        .output()
        .unwrap();
    Output::from_process(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_fs_confinement_source_grep() {
        let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut non_fs_hits = Vec::new();

        fn scan_dir(dir: &Path, non_fs_hits: &mut Vec<String>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    scan_dir(&path, non_fs_hits);
                } else if path.extension().is_some_and(|ext| ext == "rs") {
                    let file_name = path.file_name().unwrap().to_string_lossy();
                    if file_name != "fs.rs" {
                        let content = std::fs::read_to_string(&path).unwrap();
                        for (line_idx, line) in content.lines().enumerate() {
                            if line.contains("std::fs") {
                                non_fs_hits.push(format!("{}:{}: {}", path.display(), line_idx + 1, line));
                            }
                        }
                    }
                }
            }
        }

        scan_dir(&src_dir, &mut non_fs_hits);
        assert!(
            non_fs_hits.is_empty(),
            "Found std::fs outside fs.rs: {:?}",
            non_fs_hits
        );
    }

    #[test]
    fn smoke_good_tree_helper() {
        let good_tree = core_fixture_path("checks/good");
        let (_temp, path) = setup_tree(&good_tree);
        let path_str = path.to_str().unwrap();

        let out = run_cli(&["--project", path_str, "check", "--all"]);
        assert_eq!(out.code, 0, "smoke run failed: stderr:\n{}", out.stderr);
        assert!(out.stdout.contains("fail-on error: pass"));
    }
}
