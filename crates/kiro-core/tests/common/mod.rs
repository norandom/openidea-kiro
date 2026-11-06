//! Shared integration-test harness: in-memory sources built from embedded
//! fixture text so no test touches the file system, imported through the
//! crate-root re-exports (design component `IntegrationTests`).

use kiro_core::{MemorySource, Workspace};

const DEMO_SPEC: &str = include_str!("../../fixtures/minimal/.kiro/specs/demo/spec.json");
const DEMO_REQUIREMENTS: &str =
    include_str!("../../fixtures/minimal/.kiro/specs/demo/requirements.md");
const DEMO_DESIGN: &str = include_str!("../../fixtures/minimal/.kiro/specs/demo/design.md");
const DEMO_TASKS: &str = include_str!("../../fixtures/minimal/.kiro/specs/demo/tasks.md");
const OLD_SPEC: &str = include_str!("../../fixtures/minimal/.kiro/specs/old/spec.json");
const ROADMAP: &str = include_str!("../../fixtures/minimal/.kiro/steering/roadmap.md");

/// The minimal fixture tree as an in-memory source.
pub fn minimal_source() -> MemorySource {
    MemorySource::new()
        .with(".kiro/specs/demo/spec.json", DEMO_SPEC)
        .with(".kiro/specs/demo/requirements.md", DEMO_REQUIREMENTS)
        .with(".kiro/specs/demo/design.md", DEMO_DESIGN)
        .with(".kiro/specs/demo/tasks.md", DEMO_TASKS)
        .with(".kiro/specs/old/spec.json", OLD_SPEC)
        .with(".kiro/steering/roadmap.md", ROADMAP)
}

/// A workspace over the minimal tree with both feature names known.
pub fn minimal_workspace() -> Workspace<MemorySource> {
    Workspace::new(minimal_source(), vec!["demo".to_owned(), "old".to_owned()])
}
