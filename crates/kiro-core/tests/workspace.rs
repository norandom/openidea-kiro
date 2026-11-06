//! Workspace integration tests over the minimal fixture tree and this
//! spec's own documents (task 5.2): clean load, missing-document
//! severities, single read per path, the superseded chain, owners of a
//! demo source file, resolution from an absolute Windows-style path, and
//! the self-fixture invariants.

mod common;

use std::cell::RefCell;
use std::collections::BTreeMap;

use common::minimal_source;
use kiro_core::{
    MemorySource, Phase, Resolution, ResolveInput, ResolvedVia, Severity, Source, Workspace, codes,
    owners_of,
};

/// Severity of the first finding for a path, if any.
fn severity_of(findings: &[kiro_core::Finding], path: &str) -> Option<Severity> {
    findings.iter().find(|f| f.file == path).map(|f| f.severity)
}

/// Wraps a source, counting every read per path (requirement 1.3).
struct CountingSource {
    files: MemorySource,
    counts: RefCell<BTreeMap<String, usize>>,
}

impl CountingSource {
    fn new(files: MemorySource) -> Self {
        Self {
            files,
            counts: RefCell::new(BTreeMap::new()),
        }
    }

    fn count(&self, path: &str) -> usize {
        self.counts.borrow().get(path).copied().unwrap_or(0)
    }
}

impl Source for CountingSource {
    fn read(&self, path: &str) -> Option<String> {
        *self.counts.borrow_mut().entry(path.to_owned()).or_insert(0) += 1;
        self.files.read(path)
    }
}

#[test]
fn demo_loads_cleanly() {
    let mut ws = common::minimal_workspace();
    let demo = ws.load("demo");
    assert_eq!(demo.name, "demo");
    assert_eq!(demo.phase(), Phase::TasksGenerated);
    assert!(
        demo.findings.is_empty(),
        "the clean fixture produces no findings: {:?}",
        demo.findings
    );
}

#[test]
fn initialized_feature_missing_document_severities() {
    // tasks-generated (demo, from the fixture): all three documents are
    // required but present — no missing findings. The synthesised
    // `initialized` feature has only spec.json: requirements is required
    // (warning), design and tasks are not (info).
    let mut src = minimal_source();
    src.insert(".kiro/specs/init/spec.json", r#"{"phase": "initialized"}"#);
    let mut ws = Workspace::new(src, vec!["demo".to_owned()]);
    assert!(ws.load("demo").findings.is_empty());

    let init = ws.load("init").clone();
    assert!(matches!(
        severity_of(&init.findings, ".kiro/specs/init/requirements.md"),
        Some(Severity::Warning)
    ));
    assert!(matches!(
        severity_of(&init.findings, ".kiro/specs/init/design.md"),
        Some(Severity::Info)
    ));
    assert!(matches!(
        severity_of(&init.findings, ".kiro/specs/init/tasks.md"),
        Some(Severity::Info)
    ));
}

#[test]
fn each_path_is_read_exactly_once() {
    let src = CountingSource::new(minimal_source());
    let mut ws = Workspace::new(src, vec!["demo".to_owned(), "old".to_owned()]);
    {
        let _ = ws.load("demo");
    }
    {
        let _ = ws.load("demo");
    }
    let _ = ws.feature("demo");
    let _ = ws.load_roadmap();
    let _ = ws.load_roadmap();
    let _ = ws.adopt_roadmap_features();
    for path in [
        ".kiro/specs/demo/spec.json",
        ".kiro/specs/demo/requirements.md",
        ".kiro/specs/demo/design.md",
        ".kiro/specs/demo/tasks.md",
        ".kiro/steering/roadmap.md",
    ] {
        assert_eq!(ws.source().count(path), 1, "path {path} read exactly once");
    }
    // `old` was never loaded.
    assert_eq!(ws.source().count(".kiro/specs/old/spec.json"), 0);
}

#[test]
fn superseded_old_resolves_to_demo_with_a_terminating_chain() {
    let mut ws = common::minimal_workspace();
    let chain = ws.successor_chain("old");
    assert_eq!(chain.path, vec!["old".to_owned(), "demo".to_owned()]);
    assert_eq!(chain.terminal.as_deref(), Some("demo"));
    assert!(!chain.cyclic);
    assert!(
        chain.findings.is_empty(),
        "known successor: no dangling, no cycle: {:?}",
        chain.findings
    );
}

#[test]
fn owners_of_a_demo_source_file() {
    let mut ws = common::minimal_workspace();
    let demo = ws.load("demo").clone();
    let owners = owners_of(demo.design.as_ref(), demo.tasks.as_ref(), "src/demo.rs");
    assert!(owners.covered);
    assert_eq!(owners.components, vec!["Core".to_owned()]);
    let task_ids: Vec<String> = owners.tasks.iter().map(|t| t.id.clone()).collect();
    assert_eq!(task_ids, vec!["1.1".to_owned(), "2".to_owned()]);
    assert_eq!(owners.criteria, vec!["1.1".to_owned(), "2.1".to_owned()]);
}

#[test]
fn resolution_from_an_absolute_windows_style_active_path() {
    let ws = common::minimal_workspace();
    let resolution = ws.resolve_current(&ResolveInput {
        explicit: None,
        active_file: Some(r"C:\proj\.kiro\specs\demo\tasks.md"),
        workspace_root: Some(r"C:\proj"),
    });
    assert_eq!(
        resolution,
        Resolution::Feature {
            name: "demo".to_owned(),
            via: ResolvedVia::ActiveFile
        }
    );
}

#[test]
fn merged_findings_are_deterministic_across_two_runs() {
    let mut first = common::minimal_workspace();
    first.load("demo");
    first.load("old");
    first.load_roadmap();
    first.adopt_roadmap_features();
    let first_merged = first.findings();

    let mut second = common::minimal_workspace();
    second.load("demo");
    second.load("old");
    second.load_roadmap();
    second.adopt_roadmap_features();
    assert_eq!(first_merged, second.findings());
}

#[test]
fn self_fixture_components_have_files_and_clean_boundaries() {
    fn self_workspace() -> Workspace<MemorySource> {
        let src = MemorySource::new()
            .with(
                ".kiro/specs/kiro-core/spec.json",
                include_str!("../fixtures/self/spec.json"),
            )
            .with(
                ".kiro/specs/kiro-core/requirements.md",
                include_str!("../fixtures/self/requirements.md"),
            )
            .with(
                ".kiro/specs/kiro-core/design.md",
                include_str!("../fixtures/self/design.md"),
            )
            .with(
                ".kiro/specs/kiro-core/tasks.md",
                include_str!("../fixtures/self/tasks.md"),
            )
            .with(
                ".kiro/steering/roadmap.md",
                include_str!("../fixtures/self/roadmap.md"),
            );
        Workspace::new(src, vec!["kiro-core".to_owned()])
    }

    let mut ws = self_workspace();
    let feature = ws.load("kiro-core").clone();

    // Every component named in the Files by Component table has at least
    // one file (the design stays parseable by its own parser).
    let fileless: Vec<String> = feature
        .design
        .as_ref()
        .map(|design| {
            design
                .components
                .iter()
                .filter(|c| c.files.is_empty())
                .map(|c| c.name.clone())
                .collect()
        })
        .unwrap_or_else(|| vec!["<design did not parse>".to_owned()]);
    assert!(
        fileless.is_empty(),
        "every component owns at least one file: {fileless:?}"
    );

    // Tasks produce no unknown-boundary finding.
    assert!(
        feature
            .findings
            .iter()
            .all(|f| f.code != codes::TASK_BOUNDARY_UNKNOWN),
        "self tasks reference only known components: {:?}",
        feature.findings
    );

    // Two runs of the merged findings are identical (requirement 12.5).
    let first = ws.findings();
    let mut second = self_workspace();
    second.load("kiro-core");
    assert_eq!(first, second.findings());
}
