//! Path normalisation and current-spec resolution: relative path
//! normalisation, absolute-to-project-relative conversion, the feature of a
//! path under `.kiro/specs/`, resolution from an explicit name, the active
//! editor file and the known features, and the superseded-to-successor
//! chain.
//!
//! Design component `Resolver` (requirements 8.1-8.9).

use crate::finding::{Finding, Severity, codes};
use crate::source::Workspace;
use crate::spec::Phase;

/// Re-exported for consumers; defined in the metadata parser so that
/// `Workspace::load` can emit `SPEC_SUPERSEDED_DANGLING` without importing
/// upward.
pub use crate::spec::successors;

/// A path relative to the project, or one that lies outside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectPath {
    /// Project-relative path with `/` separators.
    Relative(String),
    /// A path that could not be relativized against the workspace root.
    Outside(String),
}

/// `\`→`/`, strip leading `./` (repeatedly), collapse `//`, trim trailing `/`.
pub fn normalize_relative(path: &str) -> String {
    let mut s = path.replace('\\', "/");
    while let Some(rest) = s.strip_prefix("./") {
        s = rest.to_owned();
    }
    let parts: Vec<&str> = s.split('/').filter(|p| !p.is_empty()).collect();
    parts.join("/")
}

/// True when the raw path starts with a drive letter or a UNC prefix.
fn is_windows_style(path: &str) -> bool {
    let mut chars = path.chars();
    match (chars.next(), chars.next()) {
        (Some(letter), Some(':')) => letter.is_ascii_alphabetic(),
        _ => path.starts_with("//") || path.starts_with('\\'),
    }
}

/// True when the raw path is absolute (POSIX root, UNC or drive letter).
fn is_absolute(path: &str) -> bool {
    path.starts_with('/') || path.starts_with('\\') || is_windows_style(path)
}

/// Separator unification that preserves the leading separator style
/// (POSIX `/`, UNC `//` or drive letter) while collapsing inner repeats
/// and dropping trailing slashes.
fn normalize_any(path: &str) -> String {
    let unified = path.replace('\\', "/");
    let leading = if unified.starts_with("//") {
        "//"
    } else if unified.starts_with('/') {
        "/"
    } else {
        ""
    };
    let body: Vec<&str> = unified
        .trim_start_matches('/')
        .split('/')
        .filter(|p| !p.is_empty())
        .collect();
    if leading.is_empty() {
        body.join("/")
    } else {
        format!("{leading}{}", body.join("/"))
    }
}

/// Absolute-to-relative conversion against the workspace root: the root is
/// compared after normalisation, case-insensitively when the path is
/// Windows-style (drive letter or UNC), else case-sensitively, and only at
/// a path-segment boundary. An absolute path without a root, or one not
/// under the root, is reported as outside the project.
pub fn to_project_relative(path: &str, workspace_root: Option<&str>) -> ProjectPath {
    if !is_absolute(path) {
        return ProjectPath::Relative(normalize_relative(path));
    }
    let Some(root) = workspace_root else {
        return ProjectPath::Outside(normalize_any(path));
    };
    let normalized = normalize_any(path);
    let root = normalize_any(root);
    let ignore_case = is_windows_style(path);
    let stripped = if root.is_empty() {
        normalized.strip_prefix('/').map(str::to_owned)
    } else {
        strip_root_prefix(&normalized, &root, ignore_case)
    };
    match stripped {
        Some(rest) => ProjectPath::Relative(rest),
        None => ProjectPath::Outside(normalized),
    }
}

/// Strips `root` from `path` when the prefix matches (case rule per
/// `ignore_case`) at a segment boundary.
fn strip_root_prefix(path: &str, root: &str, ignore_case: bool) -> Option<String> {
    let head = path.get(..root.len())?;
    let matches = if ignore_case {
        head.eq_ignore_ascii_case(root)
    } else {
        head == root
    };
    if !matches {
        return None;
    }
    let rest = &path[root.len()..];
    if rest.is_empty() {
        return Some(String::new());
    }
    rest.strip_prefix('/').map(str::to_owned)
}

/// `Some(feature)` when the relative path is `.kiro/specs/<feature>/<more>`.
pub fn feature_of_path(relative: &str) -> Option<String> {
    let rest = relative.strip_prefix(".kiro/specs/")?;
    let (feature, more) = rest.split_once('/')?;
    if feature.is_empty() || more.is_empty() {
        return None;
    }
    Some(feature.to_owned())
}

/// Inputs for current-spec resolution; `explicit` is trimmed by the caller
/// and an empty string counts as absent.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResolveInput<'a> {
    /// Explicit feature name from the command line, if any.
    pub explicit: Option<&'a str>,
    /// The active editor file, project-relative or absolute.
    pub active_file: Option<&'a str>,
    /// The workspace root for absolutizing, when known.
    pub workspace_root: Option<&'a str>,
}

/// How the current feature was determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedVia {
    /// An explicit name was given.
    Explicit,
    /// The active file lies under the feature's directory.
    ActiveFile,
    /// Exactly one feature is known.
    OnlyKnown,
}

/// The resolved current feature, or the unresolved candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The current feature and how it was found.
    Feature {
        /// Feature name (may be unknown to the workspace via `ActiveFile`).
        name: String,
        /// Detection path.
        via: ResolvedVia,
    },
    /// No feature could be determined.
    Unresolved {
        /// The sorted known feature names.
        candidates: Vec<String>,
        /// The normalised active path, when one was given.
        active_file: Option<ProjectPath>,
    },
}

/// Resolves the current feature: explicit name first, then the active
/// file's feature, then a single known feature, else unresolved with the
/// sorted candidates (requirements 8.1-8.4).
pub fn resolve_current(input: &ResolveInput<'_>, known: &[String]) -> Resolution {
    let explicit = input.explicit.map(str::trim).filter(|s| !s.is_empty());
    if let Some(name) = explicit {
        return Resolution::Feature {
            name: name.to_owned(),
            via: ResolvedVia::Explicit,
        };
    }
    let active = input
        .active_file
        .map(|file| to_project_relative(file, input.workspace_root));
    if let Some(ProjectPath::Relative(relative)) = &active
        && let Some(name) = feature_of_path(relative)
    {
        return Resolution::Feature {
            name,
            via: ResolvedVia::ActiveFile,
        };
    }
    if known.len() == 1 {
        return Resolution::Feature {
            name: known.first().cloned().unwrap_or_default(),
            via: ResolvedVia::OnlyKnown,
        };
    }
    let mut candidates = known.to_vec();
    candidates.sort();
    Resolution::Unresolved {
        candidates,
        active_file: active,
    }
}

/// The result of following the superseded → successor chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuccessorChain {
    /// Visited feature names in order, including a repeated name that
    /// closed a cycle.
    pub path: Vec<String>,
    /// The first non-superseded feature reached, if any.
    pub terminal: Option<String>,
    /// True when a feature was revisited.
    pub cyclic: bool,
    /// Cycle and dangling findings produced along the chain.
    pub findings: Vec<Finding>,
}

impl<S: crate::source::Source> Workspace<S> {
    /// Resolves the current feature against this workspace's known names.
    pub fn resolve_current(&self, input: &ResolveInput<'_>) -> Resolution {
        resolve_current(input, &self.known_features())
    }

    /// Follows the first successor of each superseded feature, loading as
    /// needed; stops at a non-superseded feature (terminal), at a superseded
    /// feature with no known successor (surfacing its dangling finding), or
    /// at a repeat (cyclic, `SPEC_SUPERSEDED_CYCLE`).
    pub fn successor_chain(&mut self, feature: &str) -> SuccessorChain {
        let mut chain = SuccessorChain {
            path: Vec::new(),
            terminal: None,
            cyclic: false,
            findings: Vec::new(),
        };
        let start = feature.to_owned();
        let mut current = start.clone();
        loop {
            if chain.path.contains(&current) {
                chain.cyclic = true;
                chain.findings.push(
                    Finding::new(
                        codes::SPEC_SUPERSEDED_CYCLE,
                        spec_json_path(&current),
                        format!("successor chain from '{start}' revisits '{current}'"),
                    )
                    .severity(Severity::Warning)
                    .feature(&current),
                );
                chain.path.push(current);
                break;
            }
            chain.path.push(current.clone());
            let loaded = self.load(&current).clone();
            if loaded.phase() != Phase::Superseded {
                chain.terminal = Some(current);
                break;
            }
            let known = self.known_features();
            let next = loaded.meta.as_ref().map(|meta| successors(meta, &known));
            match next.as_deref().and_then(|names| names.first()) {
                Some(next_name) => current = next_name.clone(),
                None => {
                    chain.findings.extend(
                        loaded
                            .findings
                            .iter()
                            .filter(|f| f.code == codes::SPEC_SUPERSEDED_DANGLING)
                            .cloned(),
                    );
                    break;
                }
            }
        }
        chain
    }
}

/// The fixed path of a feature's `spec.json`.
fn spec_json_path(feature: &str) -> String {
    format!(".kiro/specs/{feature}/spec.json")
}

#[cfg(test)]
mod tests {
    use super::{
        ProjectPath, Resolution, ResolveInput, ResolvedVia, feature_of_path, normalize_relative,
        resolve_current, successors, to_project_relative,
    };
    use crate::finding::codes;
    use crate::source::{MemorySource, Workspace};

    const SPEC_TASKS: &str = r#"{"phase": "tasks-generated"}"#;

    fn superseded(notes: &str) -> String {
        format!(r#"{{"phase": "superseded", "notes": "{notes}"}}"#)
    }

    #[test]
    fn normalize_relative_rules() {
        assert_eq!(normalize_relative(r"a\b\c"), "a/b/c");
        assert_eq!(normalize_relative("././x"), "x");
        assert_eq!(normalize_relative("a//b"), "a/b");
        assert_eq!(normalize_relative("a/b/"), "a/b");
        // Interior `./` is not stripped: only the leading form is.
        assert_eq!(normalize_relative("a/./b"), "a/./b");
        assert_eq!(normalize_relative(""), "");
    }

    #[test]
    fn to_project_relative_windows_and_posix_roots() {
        // Windows-style root: case-insensitive prefix, separators equalised.
        assert_eq!(
            to_project_relative(r"C:\root\x\file.md", Some(r"C:\root")),
            ProjectPath::Relative("x/file.md".to_owned())
        );
        assert_eq!(
            to_project_relative("c:/ROOT/x", Some("C:/root")),
            ProjectPath::Relative("x".to_owned())
        );
        // Mixed separators.
        assert_eq!(
            to_project_relative(r"C:/root\sub/file.md", Some("C:/root")),
            ProjectPath::Relative("sub/file.md".to_owned())
        );
        // POSIX-style root: case-sensitive.
        assert_eq!(
            to_project_relative("/root/x", Some("/root")),
            ProjectPath::Relative("x".to_owned())
        );
        assert!(matches!(
            to_project_relative("/ROOT/x", Some("/root")),
            ProjectPath::Outside(_)
        ));
        // Outside the root.
        assert!(matches!(
            to_project_relative(r"C:\other\x", Some("C:/root")),
            ProjectPath::Outside(_)
        ));
        // Segment boundary: `/roo` is not a prefix of `/rooted/x`.
        assert!(matches!(
            to_project_relative("/rooted/x", Some("/root")),
            ProjectPath::Outside(_)
        ));
        // Relative paths pass through normalised; roots are irrelevant.
        assert_eq!(
            to_project_relative("x\\y.md", None),
            ProjectPath::Relative("x/y.md".to_owned())
        );
        // Absolute without a root cannot be relativized.
        assert!(matches!(
            to_project_relative("/a/b", None),
            ProjectPath::Outside(_)
        ));
    }

    #[test]
    fn feature_of_path_rules() {
        assert_eq!(
            feature_of_path(".kiro/specs/alpha/tasks.md").as_deref(),
            Some("alpha")
        );
        // A file directly under specs/ is not a feature directory.
        assert!(feature_of_path(".kiro/specs/alpha").is_none());
        assert!(feature_of_path("src/main.rs").is_none());
        assert!(feature_of_path(".kiro/steering/roadmap.md").is_none());
    }

    fn input<'a>(
        explicit: Option<&'a str>,
        active_file: Option<&'a str>,
        workspace_root: Option<&'a str>,
    ) -> ResolveInput<'a> {
        ResolveInput {
            explicit,
            active_file,
            workspace_root,
        }
    }

    #[test]
    fn resolution_explicit_wins_regardless_of_active_file() {
        let known = vec!["a".to_owned(), "b".to_owned()];
        let res = resolve_current(
            &input(Some("feat"), Some(".kiro/specs/a/design.md"), None),
            &known,
        );
        assert_eq!(
            res,
            Resolution::Feature {
                name: "feat".to_owned(),
                via: ResolvedVia::Explicit
            }
        );
        // Empty (or whitespace) explicit counts as absent.
        let res = resolve_current(&input(Some("   "), None, None), &known);
        assert!(matches!(res, Resolution::Unresolved { .. }));
    }

    #[test]
    fn resolution_active_file_identifies_the_feature() {
        let known = vec!["alpha".to_owned(), "beta".to_owned()];
        let res = resolve_current(
            &input(None, Some(".kiro/specs/beta/design.md"), None),
            &known,
        );
        assert_eq!(
            res,
            Resolution::Feature {
                name: "beta".to_owned(),
                via: ResolvedVia::ActiveFile
            }
        );
        // Absolute Windows-style active file under the workspace root.
        let res = resolve_current(
            &input(
                None,
                Some(r"C:\root\.kiro\specs\beta\design.md"),
                Some(r"C:\root"),
            ),
            &known,
        );
        assert_eq!(
            res,
            Resolution::Feature {
                name: "beta".to_owned(),
                via: ResolvedVia::ActiveFile
            }
        );
        // A feature named by the active file resolves even when not known.
        let known_one = vec!["alpha".to_owned()];
        let res = resolve_current(
            &input(None, Some(".kiro/specs/gamma/x.md"), None),
            &known_one,
        );
        assert_eq!(
            res,
            Resolution::Feature {
                name: "gamma".to_owned(),
                via: ResolvedVia::ActiveFile
            }
        );
    }

    #[test]
    fn resolution_only_known_and_unresolved_branches() {
        let solo = vec!["solo".to_owned()];
        assert_eq!(
            resolve_current(&input(None, None, None), &solo),
            Resolution::Feature {
                name: "solo".to_owned(),
                via: ResolvedVia::OnlyKnown
            }
        );

        let many = vec!["b".to_owned(), "a".to_owned()];
        assert_eq!(
            resolve_current(&input(None, None, None), &many),
            Resolution::Unresolved {
                candidates: vec!["a".to_owned(), "b".to_owned()],
                active_file: None,
            }
        );

        // Zero known plus an outside active file reports the outside path.
        let res = resolve_current(
            &input(None, Some(r"C:\elsewhere\x.md"), Some("C:/root")),
            &[],
        );
        assert!(
            matches!(
                &res,
                Resolution::Unresolved {
                    candidates,
                    active_file: Some(ProjectPath::Outside(_)),
                } if candidates.is_empty()
            ),
            "expected unresolved with an outside active path, got {res:?}"
        );
    }

    #[test]
    fn workspace_resolve_current_uses_its_known_features() {
        let ws = Workspace::new(MemorySource::new(), vec!["only".to_owned()]);
        assert_eq!(
            ws.resolve_current(&input(None, None, None)),
            Resolution::Feature {
                name: "only".to_owned(),
                via: ResolvedVia::OnlyKnown
            }
        );
    }

    fn chain_source() -> MemorySource {
        // a -> b (terminal), c dangling, d <-> e cycle, b also plain.
        MemorySource::new()
            .with(".kiro/specs/a/spec.json", superseded("Superseded by b."))
            .with(".kiro/specs/b/spec.json", SPEC_TASKS)
            .with(
                ".kiro/specs/c/spec.json",
                superseded("Superseded by nobody."),
            )
            .with(".kiro/specs/d/spec.json", superseded("Superseded by e."))
            .with(".kiro/specs/e/spec.json", superseded("Superseded by d."))
    }

    fn ws_with(names: &[&str]) -> Workspace<MemorySource> {
        let known = names.iter().map(|n| n.to_string()).collect::<Vec<String>>();
        Workspace::new(chain_source(), known)
    }

    #[test]
    fn successor_chain_follows_first_successor_to_a_terminal() {
        let mut ws = ws_with(&["a", "b", "c", "d", "e"]);
        let chain = ws.successor_chain("a");
        assert_eq!(chain.path, vec!["a".to_owned(), "b".to_owned()]);
        assert_eq!(chain.terminal.as_deref(), Some("b"));
        assert!(!chain.cyclic);
        assert!(chain.findings.is_empty());
        // The chain loaded b.
        assert!(ws.feature("b").is_some());
        // Starting at a non-superseded feature terminates immediately.
        let chain = ws.successor_chain("b");
        assert_eq!(chain.path, vec!["b".to_owned()]);
        assert_eq!(chain.terminal.as_deref(), Some("b"));
        assert!(!chain.cyclic);
    }

    #[test]
    fn successor_chain_surfaces_dangling_at_a_dead_end() {
        let mut ws = ws_with(&["a", "b", "c", "d", "e"]);
        let chain = ws.successor_chain("c");
        assert_eq!(chain.path, vec!["c".to_owned()]);
        assert_eq!(chain.terminal, None);
        assert!(!chain.cyclic);
        assert_eq!(
            chain
                .findings
                .iter()
                .filter(|f| f.code == codes::SPEC_SUPERSEDED_DANGLING)
                .count(),
            1
        );
    }

    #[test]
    fn successor_chain_reports_a_two_feature_cycle_once() {
        let mut ws = ws_with(&["a", "b", "c", "d", "e"]);
        let chain = ws.successor_chain("d");
        assert!(chain.cyclic);
        assert_eq!(chain.terminal, None);
        assert_eq!(
            chain
                .findings
                .iter()
                .filter(|f| f.code == codes::SPEC_SUPERSEDED_CYCLE)
                .count(),
            1,
            "exactly one cycle finding"
        );
        assert!(chain.findings.iter().all(|f| f.feature.is_some()));
    }

    #[test]
    fn successors_reexport_prefers_longer_names() {
        let meta = crate::spec::parse(
            r#"{"phase": "superseded", "notes": "Use demo-two, not demo."}"#,
            ".kiro/specs/x/spec.json",
        )
        .0;
        let known = vec!["demo".to_owned(), "demo-two".to_owned()];
        assert_eq!(
            successors(&meta, &known),
            vec!["demo-two".to_owned(), "demo".to_owned()]
        );
    }
}
