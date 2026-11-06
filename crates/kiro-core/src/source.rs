//! The read port and workspace: the `Source` trait, `MemorySource`, the
//! `Workspace` with its lazy, cached loading of the four feature documents
//! and the roadmap, missing-document and dangling-successor findings, and
//! the plain `Feature` value handed to callers.
//!
//! Design component `Workspace` (requirements 1.1-1.6, 8.8, 10.4).

use std::collections::{BTreeMap, BTreeSet};

use crate::design::{self, DesignDoc};
use crate::finding::{Finding, Severity, codes, sort_findings};
use crate::join;
use crate::requirements::{self, RequirementsDoc};
use crate::roadmap::{self, RoadmapDoc};
use crate::spec::{self, Doc, Phase, SpecMeta};
use crate::tasks::{self, TasksDoc};

/// Fixed directory of the spec tree and the roadmap path.
const SPECS_DIR: &str = ".kiro/specs";
const ROADMAP_PATH: &str = ".kiro/steering/roadmap.md";

/// Text of a document by project-relative path with `/` separators, or
/// `None` when the document is absent or unreadable. The only port of the
/// engine: implementations must not panic (requirement 1.1).
pub trait Source {
    /// Text of the file at a project-relative path with '/' separators,
    /// or `None` when absent/unreadable.
    fn read(&self, project_relative_path: &str) -> Option<String>;
}

/// In-memory [`Source`] for tests and frontends; exact key lookup only,
/// no path normalisation (that lives in `resolve`).
#[derive(Debug, Default, Clone)]
pub struct MemorySource {
    files: BTreeMap<String, String>,
}

impl MemorySource {
    /// An empty source.
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    /// Builder-style insertion.
    pub fn with(mut self, path: impl Into<String>, text: impl Into<String>) -> Self {
        self.insert(path, text);
        self
    }

    /// Inserts or replaces one file.
    pub fn insert(&mut self, path: impl Into<String>, text: impl Into<String>) {
        self.files.insert(path.into(), text.into());
    }
}

impl Source for MemorySource {
    fn read(&self, project_relative_path: &str) -> Option<String> {
        self.files.get(project_relative_path).cloned()
    }
}

/// One loaded feature as plain data: metadata, the three parsed documents
/// when present, and every finding recorded while loading (parse, link,
/// missing-document, dangling-successor), sorted (requirement 1.5).
#[derive(Debug, Clone)]
pub struct Feature {
    /// Feature name as loaded.
    pub name: String,
    /// `.kiro/specs/<name>`.
    pub dir: String,
    /// `None` only when `spec.json` is absent.
    pub meta: Option<SpecMeta>,
    /// Parsed `requirements.md` when present.
    pub requirements: Option<RequirementsDoc>,
    /// Parsed `design.md` when present.
    pub design: Option<DesignDoc>,
    /// Parsed `tasks.md` when present.
    pub tasks: Option<TasksDoc>,
    /// Parse + link + missing-doc + dangling findings, sorted, all carrying
    /// the feature name.
    pub findings: Vec<Finding>,
}

impl Feature {
    /// Classified phase; `Phase::Missing` when `meta` is `None`.
    pub fn phase(&self) -> Phase {
        self.meta.as_ref().map_or(Phase::Missing, |m| m.phase)
    }

    /// Project-relative path of one of the feature's documents.
    pub fn path(&self, doc: Doc) -> String {
        format!("{}/{}", self.dir, doc_file(doc))
    }

    /// Parses the four optional document texts into a feature value.
    fn build(
        name: &str,
        spec_text: Option<String>,
        requirements_text: Option<String>,
        design_text: Option<String>,
        tasks_text: Option<String>,
        known: &[String],
    ) -> Feature {
        let dir = format!("{SPECS_DIR}/{name}");
        let spec_path = format!("{}/{}", dir, doc_file(Doc::SpecJson));
        let mut findings: Vec<Finding> = Vec::new();

        let meta = match spec_text {
            Some(text) => {
                let (meta, parsed) = spec::parse(&text, &spec_path);
                findings.extend(parsed);
                Some(meta)
            }
            None => {
                findings.push(missing_finding(&spec_path, Severity::Warning, name));
                None
            }
        };
        let phase = meta.as_ref().map_or(Phase::Missing, |m| m.phase);
        let required = spec::required_docs(phase);

        let requirements = parse_doc(
            requirements_text,
            &format!("{}/{}", dir, doc_file(Doc::Requirements)),
            doc_severity(Doc::Requirements, required),
            name,
            &mut findings,
            requirements::parse,
        );
        let design = parse_doc(
            design_text,
            &format!("{}/{}", dir, doc_file(Doc::Design)),
            doc_severity(Doc::Design, required),
            name,
            &mut findings,
            design::parse,
        );
        let tasks = parse_doc(
            tasks_text,
            &format!("{}/{}", dir, doc_file(Doc::Tasks)),
            doc_severity(Doc::Tasks, required),
            name,
            &mut findings,
            tasks::parse,
        );

        if let Some(tasks_doc) = &tasks {
            findings.extend(join::link(
                name,
                requirements.as_ref(),
                design.as_ref(),
                tasks_doc,
            ));
        }

        if let Some(m) = &meta
            && m.phase == Phase::Superseded
            && spec::successors(m, known).is_empty()
        {
            findings.push(
                Finding::new(
                    codes::SPEC_SUPERSEDED_DANGLING,
                    &spec_path,
                    format!("superseded spec '{name}' names no known successor"),
                )
                .severity(Severity::Warning)
                .feature(name),
            );
        }

        for finding in &mut findings {
            finding.feature = Some(name.to_owned());
        }
        sort_findings(&mut findings);

        Feature {
            name: name.to_owned(),
            dir,
            meta,
            requirements,
            design,
            tasks,
            findings,
        }
    }
}

/// The workspace: owns the source, the known feature names, a read cache
/// that also remembers absent paths, the loaded features and the roadmap.
/// Cache and features are append-only; there is no invalidation — a
/// frontend builds a new workspace per command.
pub struct Workspace<S: Source> {
    source: S,
    known: BTreeSet<String>,
    cache: BTreeMap<String, Option<String>>,
    features: BTreeMap<String, Feature>,
    roadmap: Option<RoadmapDoc>,
}

impl<S: Source> Workspace<S> {
    /// Creates a workspace from a source and an explicit list of known
    /// feature names (requirement 1.2); no directory listing ever happens.
    pub fn new(source: S, known_features: impl IntoIterator<Item = String>) -> Self {
        Self {
            source,
            known: known_features.into_iter().collect(),
            cache: BTreeMap::new(),
            features: BTreeMap::new(),
            roadmap: None,
        }
    }

    /// The underlying source.
    pub fn source(&self) -> &S {
        &self.source
    }

    /// Known feature names, sorted.
    pub fn known_features(&self) -> Vec<String> {
        self.known.iter().cloned().collect()
    }

    /// Adds a known feature name; returns whether it was new.
    pub fn add_known(&mut self, name: &str) -> bool {
        self.known.insert(name.to_owned())
    }

    /// Cached raw read: each path reaches the source at most once per
    /// workspace, and absent paths are remembered as absent.
    pub fn read(&mut self, project_relative_path: &str) -> Option<&str> {
        if !self.cache.contains_key(project_relative_path) {
            let text = self.source.read(project_relative_path);
            self.cache.insert(project_relative_path.to_owned(), text);
        }
        self.cache
            .get(project_relative_path)
            .and_then(|text| text.as_deref())
    }

    /// Loads a feature: reads its four documents once each through the
    /// cache, parses them, records missing-document and dangling-successor
    /// findings, runs the join's link findings when tasks exist, and adds
    /// the name to the known features (requirement 1.6). Idempotent.
    pub fn load(&mut self, feature: &str) -> &Feature {
        let key = feature.to_owned();
        // Reads always go through the cache, so an already-loaded feature
        // costs four cache hits and no source reads; `or_insert` keeps the
        // first build, keeping `load` idempotent. (An early
        // `if let Some(f) = self.features.get(..) { return f; }` hits the
        // NLL conditional-return borrow limitation E0502.)
        let spec_text = self.read_owned(&doc_path(&key, Doc::SpecJson));
        let requirements_text = self.read_owned(&doc_path(&key, Doc::Requirements));
        let design_text = self.read_owned(&doc_path(&key, Doc::Design));
        let tasks_text = self.read_owned(&doc_path(&key, Doc::Tasks));
        let known: Vec<String> = self.known.iter().cloned().collect();
        let built = Feature::build(
            &key,
            spec_text,
            requirements_text,
            design_text,
            tasks_text,
            &known,
        );
        self.add_known(&key);
        self.features.entry(key).or_insert(built)
    }

    /// An already-loaded feature, if any.
    pub fn feature(&self, name: &str) -> Option<&Feature> {
        self.features.get(name)
    }

    /// Every loaded feature, by name.
    pub fn loaded(&self) -> impl Iterator<Item = &Feature> {
        self.features.values()
    }

    /// Loads `.kiro/steering/roadmap.md` once; `None` when absent.
    pub fn load_roadmap(&mut self) -> Option<&RoadmapDoc> {
        if self.roadmap.is_none()
            && let Some(text) = self.read_owned(ROADMAP_PATH)
        {
            self.roadmap = Some(roadmap::parse(&text, ROADMAP_PATH));
        }
        self.roadmap.as_ref()
    }

    /// Loads the roadmap and adopts its names as known features; returns
    /// how many names were new.
    pub fn adopt_roadmap_features(&mut self) -> usize {
        let names = self
            .load_roadmap()
            .map(|doc| doc.names())
            .unwrap_or_default();
        names
            .into_iter()
            .filter(|name| self.known.insert(name.clone()))
            .count()
    }

    /// Findings of all loaded features plus the roadmap, merged and sorted
    /// (requirement 10.4).
    pub fn findings(&self) -> Vec<Finding> {
        let mut all: Vec<Finding> = Vec::new();
        for feature in self.features.values() {
            all.extend(feature.findings.iter().cloned());
        }
        if let Some(roadmap) = &self.roadmap {
            all.extend(roadmap.findings.iter().cloned());
        }
        sort_findings(&mut all);
        all
    }

    fn read_owned(&mut self, path: &str) -> Option<String> {
        self.read(path).map(str::to_owned)
    }
}

/// File name of one document kind.
fn doc_file(doc: Doc) -> &'static str {
    match doc {
        Doc::SpecJson => "spec.json",
        Doc::Requirements => "requirements.md",
        Doc::Design => "design.md",
        Doc::Tasks => "tasks.md",
    }
}

/// Project-relative path of a feature document.
fn doc_path(feature: &str, doc: Doc) -> String {
    format!("{SPECS_DIR}/{feature}/{}", doc_file(doc))
}

/// Missing-document severity: `warning` when the phase requires the
/// document (an absent `spec.json` is always a warning), else `info`.
fn doc_severity(doc: Doc, required: &[Doc]) -> Severity {
    if matches!(doc, Doc::SpecJson) || required.contains(&doc) {
        Severity::Warning
    } else {
        Severity::Info
    }
}

/// A `SPEC_DOC_MISSING` finding naming the path (requirement 1.4).
fn missing_finding(path: &str, severity: Severity, feature: &str) -> Finding {
    Finding::new(
        codes::SPEC_DOC_MISSING,
        path,
        format!("document '{path}' is missing"),
    )
    .severity(severity)
    .feature(feature)
}

/// Parses one optional document, appending its parser findings or a
/// missing-document finding.
fn parse_doc<T>(
    text: Option<String>,
    path: &str,
    severity: Severity,
    feature: &str,
    findings: &mut Vec<Finding>,
    parse: fn(&str, &str) -> T,
) -> Option<T>
where
    T: AsFindings,
{
    match text {
        Some(text) => {
            let mut doc = parse(&text, path);
            findings.append(doc.findings_mut());
            Some(doc)
        }
        None => {
            findings.push(missing_finding(path, severity, feature));
            None
        }
    }
}

/// Documents that expose their parser findings for merging.
trait AsFindings {
    fn findings_mut(&mut self) -> &mut Vec<Finding>;
}

impl AsFindings for RequirementsDoc {
    fn findings_mut(&mut self) -> &mut Vec<Finding> {
        &mut self.findings
    }
}

impl AsFindings for DesignDoc {
    fn findings_mut(&mut self) -> &mut Vec<Finding> {
        &mut self.findings
    }
}

impl AsFindings for TasksDoc {
    fn findings_mut(&mut self) -> &mut Vec<Finding> {
        &mut self.findings
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;

    use super::{MemorySource, Source, Workspace};
    use crate::finding::{Severity, codes};
    use crate::spec::Doc;

    const SPEC_TASKS: &str = r#"{"phase": "tasks-generated"}"#;
    const SPEC_RESERVED: &str = r#"{"phase": "reserved"}"#;
    const REQ_MIN: &str = "### Requirement 1: Core\n\n#### Acceptance Criteria\n\n1. It works.\n";
    const DESIGN_MIN: &str = "# Design\n\n## File Structure Plan\n\n| Component | File |\n|---|---|\n| Alpha | `src/a.rs` |\n";
    const TASKS_MIN: &str = "# Tasks\n\n- [ ] 1. Setup\n- [ ] 1.1 Do the work\n";

    fn alpha_source() -> MemorySource {
        MemorySource::new()
            .with(".kiro/specs/alpha/spec.json", SPEC_TASKS)
            .with(".kiro/specs/alpha/requirements.md", REQ_MIN)
            .with(".kiro/specs/alpha/design.md", DESIGN_MIN)
            .with(".kiro/specs/alpha/tasks.md", TASKS_MIN)
    }

    /// Wraps a source, counting every read per path.
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

    fn severity_of(findings: &[crate::finding::Finding], path: &str) -> Option<Severity> {
        findings.iter().find(|f| f.file == path).map(|f| f.severity)
    }

    #[test]
    fn each_document_is_read_exactly_once_across_repeated_loads() {
        let src = CountingSource::new(alpha_source());
        let mut ws = Workspace::new(src, ["alpha".to_owned()]);
        {
            let f = ws.load("alpha");
            assert_eq!(f.name, "alpha");
            assert_eq!(f.dir, ".kiro/specs/alpha");
        }
        {
            let f = ws.load("alpha");
            assert!(
                f.findings.is_empty(),
                "complete feature has no findings: {:?}",
                f.findings
            );
        }
        // `feature` and a third `load` must not reach the source again.
        let still = ws.feature("alpha").is_some();
        assert!(still);
        {
            let _ = ws.load("alpha");
        }
        for doc in [
            ".kiro/specs/alpha/spec.json",
            ".kiro/specs/alpha/requirements.md",
            ".kiro/specs/alpha/design.md",
            ".kiro/specs/alpha/tasks.md",
        ] {
            assert_eq!(ws.source().count(doc), 1, "path {doc} read exactly once");
        }
    }

    #[test]
    fn missing_document_severity_follows_the_phase_table() {
        let mut src = MemorySource::new();
        src.insert(".kiro/specs/alpha/spec.json", SPEC_TASKS);
        src.insert(".kiro/specs/beta/spec.json", SPEC_RESERVED);
        src.insert(".kiro/specs/gamma/requirements.md", REQ_MIN);
        let mut ws = Workspace::new(src, Vec::new());

        // tasks-generated: requirements, design and tasks are required -> warning.
        let alpha = ws.load("alpha").clone();
        assert!(matches!(
            severity_of(&alpha.findings, ".kiro/specs/alpha/requirements.md"),
            Some(Severity::Warning)
        ));
        assert!(matches!(
            severity_of(&alpha.findings, ".kiro/specs/alpha/design.md"),
            Some(Severity::Warning)
        ));
        assert!(matches!(
            severity_of(&alpha.findings, ".kiro/specs/alpha/tasks.md"),
            Some(Severity::Warning)
        ));

        // reserved: nothing beyond spec.json is required -> info.
        let beta = ws.load("beta").clone();
        assert!(matches!(
            severity_of(&beta.findings, ".kiro/specs/beta/requirements.md"),
            Some(Severity::Info)
        ));
        assert!(matches!(
            severity_of(&beta.findings, ".kiro/specs/beta/design.md"),
            Some(Severity::Info)
        ));
        assert!(matches!(
            severity_of(&beta.findings, ".kiro/specs/beta/tasks.md"),
            Some(Severity::Info)
        ));

        // no spec.json: the metadata document itself is always a warning,
        // and with no metadata the other missing documents are info.
        let gamma = ws.load("gamma").clone();
        assert!(matches!(
            severity_of(&gamma.findings, ".kiro/specs/gamma/spec.json"),
            Some(Severity::Warning)
        ));
        assert!(matches!(
            severity_of(&gamma.findings, ".kiro/specs/gamma/design.md"),
            Some(Severity::Info)
        ));
        assert!(matches!(
            severity_of(&gamma.findings, ".kiro/specs/gamma/tasks.md"),
            Some(Severity::Info)
        ));
        assert!(gamma.meta.is_none());

        // The missing-document finding names the path.
        let msg = alpha
            .findings
            .iter()
            .find(|f| f.code == codes::SPEC_DOC_MISSING && f.file.ends_with("requirements.md"))
            .map(|f| f.message.clone())
            .unwrap_or_default();
        assert!(msg.contains(".kiro/specs/alpha/requirements.md"));
    }

    #[test]
    fn every_finding_on_a_loaded_feature_carries_its_name() {
        let mut src = MemorySource::new();
        src.insert(".kiro/specs/delta/spec.json", SPEC_TASKS);
        src.insert(".kiro/specs/delta/requirements.md", REQ_MIN);
        src.insert(
            ".kiro/specs/delta/tasks.md",
            "# Tasks\n\n- [ ] 1. Setup\n- [ ] 1.1 Do the work\n  - _Requirements: 1.1_\n  - _Depends: 2.2_\n",
        );
        let mut ws = Workspace::new(src, Vec::new());
        let f = ws.load("delta").clone();
        assert!(
            f.findings
                .iter()
                .all(|x| x.feature.as_deref() == Some("delta")),
            "every finding carries the feature name"
        );
        let codes_found: Vec<&str> = f.findings.iter().map(|x| x.code).collect();
        assert!(codes_found.contains(&codes::SPEC_DOC_MISSING));
        assert!(codes_found.contains(&codes::TASK_DEPENDS_UNKNOWN));
        let merged = ws.findings();
        assert!(
            merged.iter().all(|x| x.feature.as_deref() == Some("delta")),
            "merged findings keep the feature name"
        );
    }

    #[test]
    fn superseded_dangling_finding_only_when_no_known_successor() {
        let mut src = MemorySource::new();
        src.insert(
            ".kiro/specs/old/spec.json",
            r#"{"phase": "superseded", "notes": "Superseded by new-hotness."}"#,
        );
        src.insert(
            ".kiro/specs/old2/spec.json",
            r#"{"phase": "superseded", "notes": "Superseded by mystery."}"#,
        );
        let mut ws = Workspace::new(src, ["new-hotness".to_owned()]);

        let old = ws.load("old").clone();
        assert!(
            !old.findings
                .iter()
                .any(|f| f.code == codes::SPEC_SUPERSEDED_DANGLING),
            "known successor named in notes: no dangling finding"
        );

        let old2 = ws.load("old2").clone();
        let dangling: Vec<&crate::finding::Finding> = old2
            .findings
            .iter()
            .filter(|f| f.code == codes::SPEC_SUPERSEDED_DANGLING)
            .collect();
        assert_eq!(dangling.len(), 1);
        assert_eq!(
            dangling.first().map(|f| f.file.as_str()),
            Some(".kiro/specs/old2/spec.json")
        );
        assert!(matches!(
            dangling.first().map(|f| f.severity),
            Some(Severity::Warning)
        ));
        assert_eq!(
            dangling.first().and_then(|f| f.feature.as_deref()),
            Some("old2")
        );
    }

    #[test]
    fn roadmap_load_adopt_and_merged_sorted_findings() {
        let mut src = MemorySource::new();
        src.insert(
            ".kiro/steering/roadmap.md",
            "# Roadmap\n\n## Specs (dependency order)\n\n\
             - [ ] alpha -- First feature. Dependencies: none\n\
             - [ ] beta -- Second feature. Dependencies: alpha\n\
             - [ ] gamma -- Third feature. Dependencies: missing-feature\n",
        );
        src.insert(".kiro/specs/old2/spec.json", SPEC_RESERVED);
        let mut ws = Workspace::new(src, Vec::new());

        let roadmap = ws.load_roadmap().cloned();
        assert_eq!(roadmap.map(|r| r.entries.len()), Some(3));
        assert!(ws.feature("alpha").is_none(), "nothing loaded yet");

        let added = ws.adopt_roadmap_features();
        assert_eq!(added, 3);
        assert_eq!(ws.known_features(), vec!["alpha", "beta", "gamma"]);

        // old2 (reserved, spec.json only) yields three info missing-document
        // findings that sort before the roadmap finding (specs < steering).
        let _ = ws.load("old2");
        let merged = ws.findings();
        assert_eq!(merged.len(), 4);
        let file_of = |i: usize| merged.get(i).map(|f| f.file.as_str());
        let code_of = |i: usize| merged.get(i).map(|f| f.code);
        assert_eq!(
            merged.first().map(|f| f.file.as_str()),
            Some(".kiro/specs/old2/design.md")
        );
        assert_eq!(
            merged.first().map(|f| f.code),
            Some(codes::SPEC_DOC_MISSING)
        );
        assert_eq!(file_of(1), Some(".kiro/specs/old2/requirements.md"));
        assert_eq!(file_of(2), Some(".kiro/specs/old2/tasks.md"));
        assert_eq!(file_of(3), Some(".kiro/steering/roadmap.md"));
        assert_eq!(code_of(3), Some(codes::ROADMAP_DEP_UNKNOWN));
        assert!(
            merged.get(3).map(|f| f.feature.is_none()).unwrap_or(false),
            "roadmap has no feature"
        );
        assert!(
            merged
                .first()
                .and_then(|f| f.feature.as_deref())
                .is_some_and(|n| n == "old2"),
            "feature findings keep the feature name after merging"
        );
    }

    #[test]
    fn known_features_are_sorted_extendable_and_extended_by_load() {
        let src = alpha_source();
        let mut ws = Workspace::new(src, ["zeta".to_owned(), "alpha".to_owned()]);
        assert_eq!(ws.known_features(), vec!["alpha", "zeta"]);
        assert!(ws.add_known("mid"));
        assert!(!ws.add_known("alpha"));

        // Loading a name outside the known list loads it and adds it (1.6).
        let fresh = ws.load("fresh").clone();
        assert!(
            fresh
                .findings
                .iter()
                .all(|f| f.code == codes::SPEC_DOC_MISSING)
        );
        assert_eq!(fresh.findings.len(), 4);
        assert_eq!(ws.known_features(), vec!["alpha", "fresh", "mid", "zeta"]);
    }

    #[test]
    fn feature_paths_and_phase_helpers() {
        let mut src = alpha_source();
        src.insert(".kiro/specs/beta/requirements.md", REQ_MIN);
        let mut ws = Workspace::new(src, Vec::new());

        let alpha = ws.load("alpha").clone();
        assert_eq!(alpha.path(Doc::SpecJson), ".kiro/specs/alpha/spec.json");
        assert_eq!(
            alpha.path(Doc::Requirements),
            ".kiro/specs/alpha/requirements.md"
        );
        assert_eq!(alpha.path(Doc::Design), ".kiro/specs/alpha/design.md");
        assert_eq!(alpha.path(Doc::Tasks), ".kiro/specs/alpha/tasks.md");
        assert_eq!(alpha.phase(), crate::spec::Phase::TasksGenerated);

        // Without spec.json the phase is Missing and meta is None.
        let beta = ws.load("beta").clone();
        assert!(beta.meta.is_none());
        assert_eq!(beta.phase(), crate::spec::Phase::Missing);
        let loaded: Vec<&str> = ws.loaded().map(|f| f.name.as_str()).collect();
        assert_eq!(loaded, vec!["alpha", "beta"]);
    }

    #[test]
    fn read_caches_absent_paths() {
        let src = CountingSource::new(MemorySource::new());
        let mut ws = Workspace::new(src, Vec::new());
        let path = ".kiro/specs/x/spec.json";
        assert!(ws.read(path).is_none());
        assert!(ws.read(path).is_none());
        assert!(ws.read(path).is_none());
        assert_eq!(ws.source().count(path), 1, "absence is cached too");
    }

    #[test]
    fn memory_source_is_exact_key_lookup() {
        let src = MemorySource::new().with("a/b.md", "text");
        assert_eq!(src.read("a/b.md").as_deref(), Some("text"));
        assert!(
            src.read("a/./b.md").is_none(),
            "no path logic in MemorySource"
        );
    }
}
