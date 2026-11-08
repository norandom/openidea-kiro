//! Command context: root, active file, workspace, known specs, resolution (task 3.1).

use crate::host::HostApi;
use crate::host_source::HostSource;
use crate::registry::SyncReport;
use kiro_core::finding::codes::ROADMAP_NO_SPECS_SECTION;
use kiro_core::resolve::{
    ProjectPath, Resolution, ResolveInput, ResolvedVia, feature_of_path, to_project_relative,
};
use kiro_core::source::{Feature, Workspace};
use kiro_core::spec::Doc;

/// Where a known spec was discovered from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnownVia {
    Roadmap,
    ActiveFile,
    Argument,
}

/// State of the roadmap file (`.kiro/steering/roadmap.md`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoadmapState {
    Absent,
    NoSpecsSection,
    Present { entries: usize },
}

/// A successfully resolved spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    pub name: String,
    pub via: ResolvedVia,
}

pub const ROADMAP_PATH: &str = ".kiro/steering/roadmap.md";

/// Everything a command execution knows about the active project.
pub struct CommandContext<'h> {
    pub host: &'h dyn HostApi,
    /// Root directory as reported by the host.
    pub root: String,
    /// Absolute active file as reported by the host, or `None` if absent or outside the root.
    pub active_file: Option<String>,
    /// Normalised active project path, or `None` if absent or outside the root.
    pub active_path: Option<ProjectPath>,
    /// State of the roadmap file.
    pub roadmap: RoadmapState,
    /// Known specs and their discovery sources in order: roadmap order first, then active-file spec, then argument spec.
    pub known: Vec<(String, Vec<KnownVia>)>,
    /// Fresh engine workspace backed by the host source adapter.
    pub workspace: Workspace<HostSource<'h>>,
    /// Per-spec command variant synchronisation report.
    pub sync: SyncReport,
}

impl<'h> CommandContext<'h> {
    /// Builds a new command context.
    ///
    /// Fails with `"no project is open"` when the host reports no root.
    /// Normalises the reported active file, treating absent or outside files as no active file.
    /// Reads the roadmap once through the workspace, records `RoadmapState`,
    /// adds the active-file spec and the explicit spec, and keeps each known spec
    /// with its discovery sources in roadmap order.
    pub fn new(host: &'h dyn HostApi, explicit: Option<&str>) -> Result<Self, String> {
        let root = host
            .workspace_root()
            .filter(|r| !r.trim().is_empty())
            .ok_or_else(|| "no project is open".to_owned())?;

        let (active_file, active_path) = match host.active_file() {
            Some(file) if !file.trim().is_empty() => {
                let path = to_project_relative(&file, Some(&root));
                match path {
                    ProjectPath::Relative(_) => (Some(file), Some(path)),
                    _ => (None, None),
                }
            }
            _ => (None, None),
        };

        let source = HostSource::new(host);
        let mut workspace = Workspace::new(source, Vec::new());

        let roadmap_doc = workspace.load_roadmap();
        let (roadmap_state, roadmap_names) = match roadmap_doc {
            None => (RoadmapState::Absent, Vec::new()),
            Some(doc) => {
                let state = if doc
                    .findings
                    .iter()
                    .any(|f| f.code == ROADMAP_NO_SPECS_SECTION)
                {
                    RoadmapState::NoSpecsSection
                } else {
                    RoadmapState::Present {
                        entries: doc.entries.len(),
                    }
                };
                (state, doc.names())
            }
        };

        let _ = workspace.adopt_roadmap_features();

        let active_spec = active_path.as_ref().and_then(|p| match p {
            ProjectPath::Relative(rel) => feature_of_path(rel),
            _ => None,
        });

        let mut known: Vec<(String, Vec<KnownVia>)> = Vec::new();
        let mut add_known_entry = |name: &str, via: KnownVia| {
            if let Some((_, vias)) = known.iter_mut().find(|(n, _)| n == name) {
                if !vias.contains(&via) {
                    vias.push(via);
                }
            } else {
                known.push((name.to_owned(), vec![via]));
            }
        };

        for name in &roadmap_names {
            add_known_entry(name, KnownVia::Roadmap);
        }
        if let Some(ref name) = active_spec {
            add_known_entry(name, KnownVia::ActiveFile);
            workspace.add_known(name);
        }
        if let Some(name) = explicit {
            add_known_entry(name, KnownVia::Argument);
            workspace.add_known(name);
        }

        Ok(Self {
            host,
            root,
            active_file,
            active_path,
            roadmap: roadmap_state,
            known,
            workspace,
            sync: SyncReport::default(),
        })
    }

    /// Known spec names in `known` order (roadmap order first).
    #[must_use]
    pub fn known_names(&self) -> Vec<String> {
        self.known.iter().map(|(name, _)| name.clone()).collect()
    }

    /// Resolves the current spec through the engine.
    ///
    /// Checks explicit name first, then active file, then single known spec.
    /// Returns informative error messages when unresolved.
    pub fn resolve(&self, explicit: Option<&str>) -> Result<Resolved, String> {
        let input = ResolveInput {
            explicit,
            active_file: self.active_file.as_deref(),
            workspace_root: Some(&self.root),
        };
        match self.workspace.resolve_current(&input) {
            Resolution::Feature { name, via } => Ok(Resolved { name, via }),
            Resolution::Unresolved { candidates, .. } => {
                if candidates.is_empty() {
                    Err("no spec is known: add a `## Specs (dependency order)` list to .kiro/steering/roadmap.md or open a spec file".to_owned())
                } else {
                    Err(format!(
                        "no current spec: open a file under .kiro/specs/<spec>/ or run the per-spec command; candidates: {}",
                        candidates.join(", ")
                    ))
                }
            }
        }
    }

    /// Loads a spec into the workspace cache and returns a reference to it.
    pub fn load(&mut self, name: &str) -> &Feature {
        self.workspace.load(name)
    }

    /// Checks whether a given document is readable on a loaded feature.
    #[must_use]
    pub fn is_readable(feature: &Feature, doc: Doc) -> bool {
        match doc {
            Doc::SpecJson => feature.meta.is_some(),
            Doc::Requirements => feature.requirements.is_some(),
            Doc::Design => feature.design.is_some(),
            Doc::Tasks => feature.tasks.is_some(),
        }
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
    use crate::host::{FakeHost, HostEvent};

    #[test]
    fn fails_without_project_root() {
        let host = FakeHost::new();
        let res = CommandContext::new(&host, None);
        assert_eq!(res.err().as_deref(), Some("no project is open"));
    }

    #[test]
    fn outside_and_absent_active_file_treated_as_none() {
        let mut host = FakeHost::new().with_root("C:\\proj");

        // Case 1: absent active file
        let ctx = CommandContext::new(&host, None).unwrap();
        assert_eq!(ctx.active_file, None);
        assert_eq!(ctx.active_path, None);

        // Case 2: outside active file
        host.active = Some("D:\\other\\file.txt".to_owned());
        let ctx2 = CommandContext::new(&host, None).unwrap();
        assert_eq!(ctx2.active_file, None);
        assert_eq!(ctx2.active_path, None);

        // Case 3: inside active file
        host.active = Some("C:\\proj\\.kiro\\specs\\my-spec\\design.md".to_owned());
        let ctx3 = CommandContext::new(&host, None).unwrap();
        assert_eq!(
            ctx3.active_file.as_deref(),
            Some("C:\\proj\\.kiro\\specs\\my-spec\\design.md")
        );
        assert_eq!(
            ctx3.active_path,
            Some(ProjectPath::Relative(
                ".kiro/specs/my-spec/design.md".to_owned()
            ))
        );
    }

    #[test]
    fn roadmap_states_derived_accurately() {
        let mut host = FakeHost::new().with_root("C:\\proj");

        // Case 1: Absent
        let ctx1 = CommandContext::new(&host, None).unwrap();
        assert_eq!(ctx1.roadmap, RoadmapState::Absent);

        // Case 2: NoSpecsSection
        host.files.insert(
            ".kiro/steering/roadmap.md".to_owned(),
            "# Roadmap\nJust text here\n".to_owned(),
        );
        let ctx2 = CommandContext::new(&host, None).unwrap();
        assert_eq!(ctx2.roadmap, RoadmapState::NoSpecsSection);

        // Case 3: Present
        let rm_content = "# Roadmap\n## Specs (dependency order)\n- [ ] a\n- [ ] b\n";
        host.files.insert(
            ".kiro/steering/roadmap.md".to_owned(),
            rm_content.to_owned(),
        );
        let ctx3 = CommandContext::new(&host, None).unwrap();
        assert_eq!(ctx3.roadmap, RoadmapState::Present { entries: 2 });
        assert_eq!(ctx3.known_names(), vec!["a", "b"]);
    }

    #[test]
    fn known_spec_union_with_sources_and_ordering() {
        let rm = "# Roadmap\n## Specs (dependency order)\n- [ ] alpha\n- [ ] beta\n";
        let host = FakeHost::new()
            .with_root("C:\\proj")
            .with_file(".kiro/steering/roadmap.md", rm)
            .with_active("C:\\proj\\.kiro\\specs\\beta\\tasks.md");

        let ctx = CommandContext::new(&host, Some("gamma")).unwrap();
        assert_eq!(ctx.known_names(), vec!["alpha", "beta", "gamma"]);

        assert_eq!(
            ctx.known,
            vec![
                ("alpha".to_owned(), vec![KnownVia::Roadmap]),
                (
                    "beta".to_owned(),
                    vec![KnownVia::Roadmap, KnownVia::ActiveFile]
                ),
                ("gamma".to_owned(), vec![KnownVia::Argument]),
            ]
        );
    }

    #[test]
    fn resolution_success_and_unresolved_error_messages() {
        let mut host = FakeHost::new().with_root("C:\\proj");

        // 1. No specs known
        let ctx1 = CommandContext::new(&host, None).unwrap();
        let err1 = ctx1.resolve(None).unwrap_err();
        assert_eq!(
            err1,
            "no spec is known: add a `## Specs (dependency order)` list to .kiro/steering/roadmap.md or open a spec file"
        );

        // 2. Multiple candidates known, active file absent
        let rm = "# Roadmap\n## Specs (dependency order)\n- [ ] beta\n- [ ] alpha\n";
        host.files
            .insert(".kiro/steering/roadmap.md".to_owned(), rm.to_owned());
        let ctx2 = CommandContext::new(&host, None).unwrap();
        let err2 = ctx2.resolve(None).unwrap_err();
        // candidates must be sorted alphabetically
        assert_eq!(
            err2,
            "no current spec: open a file under .kiro/specs/<spec>/ or run the per-spec command; candidates: alpha, beta"
        );

        // 3. Explicit resolution
        let res3 = ctx2.resolve(Some("beta")).unwrap();
        assert_eq!(
            res3,
            Resolved {
                name: "beta".to_owned(),
                via: ResolvedVia::Explicit
            }
        );

        // 4. Single known spec resolved via OnlyKnown
        let host3 = FakeHost::new().with_root("C:\\proj").with_file(
            ".kiro/steering/roadmap.md",
            "# Roadmap\n## Specs (dependency order)\n- [ ] solo\n",
        );
        let ctx3 = CommandContext::new(&host3, None).unwrap();
        let res4 = ctx3.resolve(None).unwrap();
        assert_eq!(
            res4,
            Resolved {
                name: "solo".to_owned(),
                via: ResolvedVia::OnlyKnown
            }
        );

        // 5. Active file resolution
        let host4 = FakeHost::new()
            .with_root("C:\\proj")
            .with_file(
                ".kiro/steering/roadmap.md",
                "# Roadmap\n## Specs (dependency order)\n- [ ] one\n- [ ] two\n",
            )
            .with_active("C:\\proj\\.kiro\\specs\\two\\design.md");
        let ctx4 = CommandContext::new(&host4, None).unwrap();
        let res5 = ctx4.resolve(None).unwrap();
        assert_eq!(
            res5,
            Resolved {
                name: "two".to_owned(),
                via: ResolvedVia::ActiveFile
            }
        );
    }

    #[test]
    fn repeated_loads_cause_no_repeated_read_events() {
        let host = FakeHost::new().with_root("C:\\proj").with_file(
            ".kiro/specs/test-spec/spec.json",
            r#"{"name": "test-spec", "phase": "tasks"}"#,
        );

        let mut ctx = CommandContext::new(&host, Some("test-spec")).unwrap();

        // Count reads before first load
        let reads_before = host
            .events()
            .into_iter()
            .filter(|e| matches!(e, HostEvent::Read(_)))
            .count();

        let feat1 = ctx.load("test-spec");
        assert_eq!(feat1.name, "test-spec");
        assert!(CommandContext::is_readable(feat1, Doc::SpecJson));
        assert!(!CommandContext::is_readable(feat1, Doc::Design));

        let reads_after_first = host
            .events()
            .into_iter()
            .filter(|e| matches!(e, HostEvent::Read(_)))
            .count();
        assert!(reads_after_first > reads_before);

        // Second load of the same spec
        let feat2 = ctx.load("test-spec");
        assert_eq!(feat2.name, "test-spec");

        let reads_after_second = host
            .events()
            .into_iter()
            .filter(|e| matches!(e, HostEvent::Read(_)))
            .count();
        assert_eq!(reads_after_first, reads_after_second);
    }
}
