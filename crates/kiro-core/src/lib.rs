//! # kiro-core
//!
//! The shared engine for spec constraint checking on the cc-sdd / kiro
//! layout. It turns the text of a `.kiro` tree into a plain, inspectable
//! model, resolves the current spec, joins files to components and tasks,
//! reports findings in one stable shape and renders graphs as Mermaid or
//! text. The crate performs no file system, network, environment, clock or
//! thread operations: content enters through the `source` module's read
//! port, so the same code runs natively and under `wasm32-wasip2`.
//!
//! # Accepted grammars
//!
//! Every parser is total: malformed input yields partial data plus
//! findings, never a panic (requirement 12.1). Fenced code blocks are
//! ignored by the Markdown parsers; an unclosed fence runs to the last
//! line and is reported once.
//!
//! ## `spec.json`
//!
//! A JSON object. Recognised keys: `feature_name` and `language`
//! (strings), `phase` (case-insensitive one of `initialized`,
//! `requirements-generated`, `design-generated`, `tasks-generated`,
//! `superseded`, `reserved`; anything else warns `SPEC_PHASE_UNKNOWN`,
//! absent or blank warns `SPEC_PHASE_MISSING`), `approvals` (object with
//! `requirements`/`design`/`tasks`, each `{ "generated": bool,
//! "approved": bool }`), `ready_for_implementation` (bool), `topics`
//! (array of strings), `notes` (string), `created_at` and `updated_at`
//! (strings). Non-object JSON yields `SPEC_JSON_INVALID` with default
//! metadata; unknown keys are ignored.
//!
//! ## `requirements.md`
//!
//! Level-3 requirement headings in four grammars, each with criteria
//! identified as `N.M`:
//!
//! - **G1 canonical**: `### Requirement N: Title`, an optional
//!   `**Objective:**` line, then `#### Acceptance Criteria` with `1.` or
//!   `1)` numbered items; continuation lines join the item above.
//! - **G1 dotted**: criterion items carry their own `N.M` ids anywhere in
//!   the requirement block.
//! - **G2 R-style**: `### R<N> — Title` with `- R<N>.<M> text` bullets;
//!   ids canonicalise to plain `N.M`.
//! - **G3 amendments**: `### Amendment A<K> — Title (extends Requirement
//!   N)` with `A<K>.<M>` criteria, yielding ids `A<K>.<M>`.
//!
//! Italic annotation lines `_Key: value_` attach at the requirement and
//! the criterion level. Duplicate ids keep the first definition
//! (`REQ_DUPLICATE_ID`); a heading without a number warns
//! `REQ_NON_NUMERIC_ID` and the block is skipped; a baseline requirement
//! without criteria warns `REQ_NO_CRITERIA`.
//!
//! ## `tasks.md`
//!
//! Checkbox lines form a two-level tree: `- [ ] N. Major` at the left
//! margin and indented `- [ ] N.M Sub`; `[x]` marks a task done, `*`
//! immediately after the checkbox marks it optional (`- [ ]* 2.`), and
//! `(P)` at the start of the description marks it parallel. Indented
//! detail lines attach to the open task. Indented annotation lines are
//! interpreted when the key is `Requirements` (comma-separated `N`,
//! `N.M` or `A<K>.<M>` refs), `Depends` (task ids) or `Boundary`
//! (component names); every other `_Key: value_` line is kept as a
//! generic annotation. Depth beyond two components and sub-tasks without
//! their major warn `TASK_HIERARCHY`; duplicate ids keep the first
//! (`TASK_DUPLICATE_ID`); malformed requirement refs warn
//! `TASK_REQ_REF_INVALID`.
//!
//! ## `design.md`
//!
//! A `## File Structure Plan` section in three shapes: a two-or-more
//! column table (`| Component | File |` with backticked paths), a bullet
//! list, or an indented tree. Paths are normalised: backticks stripped,
//! backslashes to slashes, one leading `./` removed, repeated slashes
//! collapsed. Components also come from `#### Name` headings in a
//! components region and from every table row; a file cell without a
//! component marks the file unassigned (`DESIGN_FILE_UNASSIGNED`), a
//! component without files warns `DESIGN_COMPONENT_NO_FILES`, and a
//! missing plan section warns `DESIGN_NO_FILE_PLAN`. Fenced blocks with
//! info string `mermaid` are captured with their line spans anywhere in
//! the document.
//!
//! ## `.kiro/steering/roadmap.md`
//!
//! The list under a heading whose text starts with `Specs` and contains
//! `dependency order` (case-insensitive). Entries have the form
//! `- [ ] name -- description. Dependencies: a, b` (separator `--` or an
//! em dash; `none` or an absent part means no dependencies). A dependency
//! naming no entry warns `ROADMAP_DEP_UNKNOWN`; a missing section yields
//! an empty list and `ROADMAP_NO_SPECS_SECTION`.
//!
//! # Finding codes
//!
//! Every code the engine can produce, with its default severity (the
//! workspace overrides `SPEC_DOC_MISSING` per the phase table; an absent
//! `spec.json` is always a warning):
//!
//! | Code | Severity | Description |
//! |------|----------|-------------|
//! | SPEC_DOC_MISSING | info | A feature document could not be read |
//! | SPEC_JSON_INVALID | error | spec.json is not a JSON object |
//! | SPEC_PHASE_UNKNOWN | warning | phase is not a recognised value |
//! | SPEC_PHASE_MISSING | warning | phase absent or empty |
//! | SPEC_SUPERSEDED_DANGLING | warning | Superseded feature names no known successor |
//! | SPEC_SUPERSEDED_CYCLE | warning | Successor chain revisits a feature |
//! | MD_FENCE_UNCLOSED | info | A fenced block runs to the end of the document |
//! | REQ_NO_CRITERIA | warning | Baseline requirement without criteria |
//! | REQ_DUPLICATE_ID | warning | Requirement or criterion id defined twice |
//! | REQ_NON_NUMERIC_ID | warning | ### Requirement heading without a number |
//! | TASK_HIERARCHY | warning | Depth > 2 or sub-task without its major |
//! | TASK_REQ_REF_INVALID | warning | _Requirements:_ entry not N, N.M or A\<K\>.M |
//! | TASK_DUPLICATE_ID | warning | Task id defined twice |
//! | TASK_BOUNDARY_UNKNOWN | warning | _Boundary:_ names no design component |
//! | TASK_REQ_UNKNOWN | warning | _Requirements:_ names an undefined criterion |
//! | TASK_DEPENDS_UNKNOWN | warning | _Depends:_ names an undefined task |
//! | DESIGN_COMPONENT_NO_FILES | info | Component with no mapped file |
//! | DESIGN_FILE_UNASSIGNED | info | Plan file with no component |
//! | DESIGN_NO_FILE_PLAN | warning | No File Structure Plan section |
//! | ROADMAP_DEP_UNKNOWN | warning | Dependency names no roadmap entry |
//! | ROADMAP_NO_SPECS_SECTION | info | No `## Specs (dependency order)` section |
//!
//! The table equals the [`CODES`] catalogue, asserted by this doc test:
//!
//! ```
//! use kiro_core::{CODES, Severity};
//!
//! let documented: &[(&str, Severity, &str)] = &[
//!     ("SPEC_DOC_MISSING", Severity::Info, "A feature document could not be read"),
//!     ("SPEC_JSON_INVALID", Severity::Error, "spec.json is not a JSON object"),
//!     ("SPEC_PHASE_UNKNOWN", Severity::Warning, "phase is not a recognised value"),
//!     ("SPEC_PHASE_MISSING", Severity::Warning, "phase absent or empty"),
//!     ("SPEC_SUPERSEDED_DANGLING", Severity::Warning, "Superseded feature names no known successor"),
//!     ("SPEC_SUPERSEDED_CYCLE", Severity::Warning, "Successor chain revisits a feature"),
//!     ("MD_FENCE_UNCLOSED", Severity::Info, "A fenced block runs to the end of the document"),
//!     ("REQ_NO_CRITERIA", Severity::Warning, "Baseline requirement without criteria"),
//!     ("REQ_DUPLICATE_ID", Severity::Warning, "Requirement or criterion id defined twice"),
//!     ("REQ_NON_NUMERIC_ID", Severity::Warning, "### Requirement heading without a number"),
//!     ("TASK_HIERARCHY", Severity::Warning, "Depth > 2 or sub-task without its major"),
//!     ("TASK_REQ_REF_INVALID", Severity::Warning, "_Requirements:_ entry not N, N.M or A<K>.M"),
//!     ("TASK_DUPLICATE_ID", Severity::Warning, "Task id defined twice"),
//!     ("TASK_BOUNDARY_UNKNOWN", Severity::Warning, "_Boundary:_ names no design component"),
//!     ("TASK_REQ_UNKNOWN", Severity::Warning, "_Requirements:_ names an undefined criterion"),
//!     ("TASK_DEPENDS_UNKNOWN", Severity::Warning, "_Depends:_ names an undefined task"),
//!     ("DESIGN_COMPONENT_NO_FILES", Severity::Info, "Component with no mapped file"),
//!     ("DESIGN_FILE_UNASSIGNED", Severity::Info, "Plan file with no component"),
//!     ("DESIGN_NO_FILE_PLAN", Severity::Warning, "No File Structure Plan section"),
//!     ("ROADMAP_DEP_UNKNOWN", Severity::Warning, "Dependency names no roadmap entry"),
//!     ("ROADMAP_NO_SPECS_SECTION", Severity::Info, "No `## Specs (dependency order)` section"),
//! ];
//! assert_eq!(CODES.len(), documented.len(), "no undocumented or stale codes");
//! for (info, (code, severity, description)) in CODES.iter().zip(documented) {
//!     assert_eq!(info.code, *code);
//!     assert_eq!(info.severity, *severity);
//!     assert_eq!(info.description, *description);
//! }
//! ```
//!
//! # A workspace load, a resolution and a render
//!
//! Using only the crate-root re-exported names:
//!
//! ```
//! use kiro_core::{
//!     Caps, Direction, Edge, Graph, MemorySource, Node, Resolution, ResolveInput, Workspace,
//!     graph_text, mermaid,
//! };
//!
//! let source = MemorySource::new()
//!     .with(".kiro/specs/demo/spec.json", r#"{"phase": "tasks-generated"}"#)
//!     .with(".kiro/specs/demo/tasks.md", "- [ ] 1. Set up\n");
//! let mut workspace = Workspace::new(source, ["demo".to_owned()]);
//! let feature = workspace.load("demo");
//! assert!(
//!     feature
//!         .findings
//!         .iter()
//!         .any(|f| f.file == ".kiro/specs/demo/requirements.md"),
//!     "tasks-generated: the missing requirements document is reported"
//! );
//!
//! let resolution = workspace.resolve_current(&ResolveInput {
//!     explicit: None,
//!     active_file: Some(".kiro/specs/demo/tasks.md"),
//!     workspace_root: None,
//! });
//! assert!(matches!(resolution, Resolution::Feature { name, .. } if name == "demo"));
//!
//! let graph = Graph {
//!     direction: Direction::LeftRight,
//!     nodes: vec![Node { id: "demo".into(), label: "Demo".into(), class: None }],
//!     edges: vec![Edge { from: "demo".into(), to: "demo".into(), label: None }],
//! };
//! let caps = Caps { max_nodes: 10, max_edges: 10 };
//! assert!(mermaid(&graph, &caps).text.starts_with("graph LR"));
//! assert!(graph_text(&graph, &caps).text.contains("Demo [demo]"));
//! ```
//!
//! # Modules
//!
//! One module per seam, declared up front in the order of the File Structure
//! Plan in `.kiro/specs/kiro-core/design.md`, so the crate builds before any
//! module has content (each stub file is owned by its module's task):
//!
//! ```
//! const MODULES: [&str; 11] = [
//!     "finding", "md", "spec", "requirements", "tasks", "design", "roadmap",
//!     "join", "source", "resolve", "render",
//! ];
//! assert_eq!(MODULES.len(), 11);
//! ```
//!
//! `render` groups the two renderer submodules `render::mermaid` and
//! `render::text`. The `vocab` and `checks` seams of the steering module map
//! belong to the vocabulary-grounding and spec-constraints specs and are
//! deliberately absent from this crate.
#![forbid(unsafe_code)]

pub mod design;
pub mod finding;
pub mod join;
pub mod md;
pub mod render;
pub mod requirements;
pub mod resolve;
pub mod roadmap;
pub mod source;
pub mod spec;
pub mod tasks;

// Crate-root re-exports as listed in the design's CrateScaffold contract.
pub use design::{Component, DesignDoc, DesignFile, MermaidBlock, normalize_design_path};
pub use finding::{CODES, CodeInfo, Finding, Severity, codes, sort_findings};
pub use join::{
    ComponentCoverage, Coverage, Owners, TaskRef, coverage, link, names_equal, owners_of,
    path_matches,
};
pub use md::Annotation;
pub use render::mermaid::mermaid;
pub use render::text::{findings_text, graph_text, table};
pub use render::{Caps, DEFAULT_CAPS, Direction, Edge, Graph, IdMap, Node, Rendered, clip_lines};
pub use requirements::{Criterion, ReqKind, Requirement, RequirementsDoc};
pub use resolve::{
    ProjectPath, Resolution, ResolveInput, ResolvedVia, SuccessorChain, feature_of_path,
    normalize_relative, resolve_current, to_project_relative,
};
pub use roadmap::{RoadmapDoc, RoadmapEntry};
pub use source::{Feature, MemorySource, Source, Workspace};
pub use spec::{Approval, Approvals, Doc, Phase, SpecMeta, phase_label, required_docs, successors};
pub use tasks::{Counts, Task, TasksDoc};
