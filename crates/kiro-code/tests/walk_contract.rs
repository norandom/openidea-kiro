//! The source walk honors the project's ignore files (code-constraints 1.6).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use std::path::Path;

use kiro_code::read_bundle;

fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn paths(root: &Path) -> Vec<String> {
    let root = root.canonicalize().unwrap();
    read_bundle(&root, &|path| path.ends_with(".rs"))
        .unwrap()
        .files
        .into_iter()
        .map(|file| file.path)
        .collect()
}

#[test]
fn ignored_paths_are_not_read_and_no_git_directory_is_needed() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();
    write(root, "src/a.rs", "");
    write(root, ".hidden/b.rs", "");
    write(root, ".copy/plugins/c.rs", "");
    write(root, "gen/d.rs", "");
    write(root, "gen/keep.rs", "");
    write(root, "sub/e.rs", "");
    write(root, "sub/local.rs", "");
    write(root, "target/f.rs", "");
    write(root, "node_modules/g.rs", "");

    // Without ignore files: everything except the fixed exclusions, and a
    // directory whose name starts with a dot is walked.
    assert_eq!(
        paths(root),
        [
            ".copy/plugins/c.rs",
            ".hidden/b.rs",
            "gen/d.rs",
            "gen/keep.rs",
            "src/a.rs",
            "sub/e.rs",
            "sub/local.rs",
        ]
    );

    // A root-anchored directory, a pattern with an exception, and an
    // ignore file in a subdirectory.
    write(root, ".gitignore", "/.copy/\ngen/*\n!gen/keep.rs\n");
    write(root, "sub/.gitignore", "local.rs\n");
    assert_eq!(
        paths(root),
        [".hidden/b.rs", "gen/keep.rs", "src/a.rs", "sub/e.rs"]
    );
}

#[test]
fn a_nested_repository_or_worktree_is_not_part_of_the_project() {
    let temp = tempfile::TempDir::new().unwrap();
    let root = temp.path();
    // The root's own `.git` does not exclude the root.
    write(root, ".git/HEAD", "");
    write(root, "src/a.rs", "");
    // A nested repository has a `.git` directory, a worktree a `.git` file.
    write(root, "vendor/lib/.git/HEAD", "");
    write(root, "vendor/lib/src/b.rs", "");
    write(root, ".worktrees/w1/.git", "gitdir: elsewhere\n");
    write(root, ".worktrees/w1/src/c.rs", "");
    write(root, "vendor/plain/d.rs", "");
    assert_eq!(paths(root), ["src/a.rs", "vendor/plain/d.rs"]);
}

#[test]
fn an_ignore_file_above_the_root_does_not_decide() {
    let temp = tempfile::TempDir::new().unwrap();
    write(temp.path(), ".gitignore", "*.rs\n");
    write(temp.path(), "project/src/a.rs", "");
    assert_eq!(paths(&temp.path().join("project")), ["src/a.rs"]);
}
