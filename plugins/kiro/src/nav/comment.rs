//! Comment syntax determination and line wrapping for code files (task 2.1).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommentStyle {
    DoubleSlash,
    Hash,
    DoubleDash,
    Html,
    Bare,
}

pub const DOUBLE_SLASH_EXTENSIONS: &[&str] = &[
    "c", "h", "cpp", "cc", "cxx", "hpp", "hh", "cs", "java", "kt", "kts", "swift", "go", "rs",
    "js", "mjs", "cjs", "jsx", "ts", "tsx", "dart", "scala", "groovy", "php", "zig", "wit",
    "proto",
];

pub const HASH_EXTENSIONS: &[&str] = &[
    "py", "rb", "sh", "bash", "zsh", "ps1", "psm1", "psd1", "toml", "yaml", "yml", "pl", "pm", "r",
    "cmake", "ex", "exs", "nim", "tf", "hcl",
];

pub const DOUBLE_DASH_EXTENSIONS: &[&str] = &[
    "sql", "lua", "hs", "elm", "ada", "adb", "ads", "vhd", "vhdl",
];

pub const HTML_EXTENSIONS: &[&str] = &[
    "md", "markdown", "html", "htm", "xml", "svg", "vue", "svelte", "xaml", "csproj", "fsproj",
    "vbproj", "props", "targets",
];

pub const HASH_BASENAMES: &[&str] = &["makefile", "dockerfile", "justfile"];

/// Selects the comment style appropriate for `path`.
///
/// `name` is the segment after the last forward slash.
/// `extension` is the segment after the last `.` of `name`, provided that `.`
/// is not the first character of `name` and the extension is not empty.
/// Both `extension` and `name` are compared case-insensitively (ASCII lowercase).
/// Extension tables are checked first. If there is no extension, `HASH_BASENAMES`
/// is checked. Otherwise `CommentStyle::Bare` is returned.
#[must_use]
pub fn style_for(path: &str) -> CommentStyle {
    let name = path.rsplit('/').next().unwrap_or(path);

    if let Some(dot_idx) = name.rfind('.') {
        let ext = if dot_idx > 0 {
            name.get(dot_idx + 1..).unwrap_or("")
        } else {
            ""
        };
        if !ext.is_empty() {
            let lower = ext.to_ascii_lowercase();
            if DOUBLE_SLASH_EXTENSIONS.iter().any(|&e| e == lower) {
                return CommentStyle::DoubleSlash;
            }
            if HASH_EXTENSIONS.iter().any(|&e| e == lower) {
                return CommentStyle::Hash;
            }
            if DOUBLE_DASH_EXTENSIONS.iter().any(|&e| e == lower) {
                return CommentStyle::DoubleDash;
            }
            if HTML_EXTENSIONS.iter().any(|&e| e == lower) {
                return CommentStyle::Html;
            }
            return CommentStyle::Bare;
        }
    }

    let lower_name = name.to_ascii_lowercase();
    if HASH_BASENAMES.iter().any(|&b| b == lower_name) {
        return CommentStyle::Hash;
    }

    CommentStyle::Bare
}

/// Wraps `text` in comment markers according to `style`.
///
/// For `Html`, replaces `--` with `- -`.
/// Does not add a trailing newline.
#[must_use]
pub fn wrap(style: CommentStyle, text: &str) -> String {
    match style {
        CommentStyle::DoubleSlash => format!("// {text}"),
        CommentStyle::Hash => format!("# {text}"),
        CommentStyle::DoubleDash => format!("-- {text}"),
        CommentStyle::Html => {
            let safe_text = text.replace("--", "- -");
            format!("<!-- {safe_text} -->")
        }
        CommentStyle::Bare => text.to_string(),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]
mod tests {
    use super::*;

    #[test]
    fn table_class_representatives() {
        assert_eq!(style_for("src/lib.rs"), CommentStyle::DoubleSlash);
        assert_eq!(style_for("scripts/run.py"), CommentStyle::Hash);
        assert_eq!(style_for("db/migration.sql"), CommentStyle::DoubleDash);
        assert_eq!(style_for("docs/spec.md"), CommentStyle::Html);
    }

    #[test]
    fn uppercase_extension() {
        assert_eq!(style_for("SRC/FOO.RS"), CommentStyle::DoubleSlash);
        assert_eq!(style_for("CONFIG.TOML"), CommentStyle::Hash);
        assert_eq!(style_for("INDEX.HTML"), CommentStyle::Html);
    }

    #[test]
    fn basenames_without_extension() {
        assert_eq!(style_for("Makefile"), CommentStyle::Hash);
        assert_eq!(style_for("subdir/DOCKERFILE"), CommentStyle::Hash);
        assert_eq!(style_for("Justfile"), CommentStyle::Hash);
    }

    #[test]
    fn leading_dot_and_special_names() {
        assert_eq!(style_for(".gitignore"), CommentStyle::Bare);
        assert_eq!(style_for("path/.env"), CommentStyle::Bare);
        assert_eq!(style_for("data.json"), CommentStyle::Bare);
        assert_eq!(style_for("archive.tar.gz"), CommentStyle::Bare);
        assert_eq!(style_for("unrecognized_name"), CommentStyle::Bare);
    }

    #[test]
    fn wrap_styles() {
        assert_eq!(
            wrap(CommentStyle::DoubleSlash, "kiro-spec: foo"),
            "// kiro-spec: foo"
        );
        assert_eq!(
            wrap(CommentStyle::Hash, "kiro-spec: foo"),
            "# kiro-spec: foo"
        );
        assert_eq!(
            wrap(CommentStyle::DoubleDash, "kiro-spec: foo"),
            "-- kiro-spec: foo"
        );
        assert_eq!(
            wrap(CommentStyle::Html, "kiro-spec: foo -- bar"),
            "<!-- kiro-spec: foo - - bar -->"
        );
        assert_eq!(wrap(CommentStyle::Bare, "kiro-spec: foo"), "kiro-spec: foo");
    }
}
