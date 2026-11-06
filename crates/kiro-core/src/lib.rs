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
//! Crate-root re-exports of the public model, workspace, resolution, join,
//! finding and render types (task 5.1). The accepted grammar per document
//! type and the finding-code catalogue with their doc tests are written in
//! task 5.4.
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
