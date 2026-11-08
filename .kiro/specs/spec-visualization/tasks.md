# Implementation Plan

- [x] 1. Foundation: engine view support
- [x] 1.1 Create the engine's visualization module and its shared view support
  - Prerequisite: `crates/kiro-core` from the kiro-core spec is implemented (model, render, `fixtures/minimal`, `tests/common`)
  - Module declared in the engine crate root with a crate-doc paragraph naming the three views; the three view submodules (status, graph, tasks) declared as documented empty stubs whose ownership passes to 2.1–2.3; after this task the shared module file is frozen for the parallel view tasks
  - Label seam: the one-method preferred-label trait taking the raw component name and the spec's declared topics, and the identity implementation that always answers "use the raw name"
  - Shared rules: label truncation at 48 characters with an ellipsis (character-based, whitespace runs collapsed), the task label (checkbox, optional star, id, parallel marker, truncated description), the phase label (`pub use crate::spec::phase_label`, shared with the plugin's current-spec report), the bare document file-name helper
  - Output forms: the diagram body as a fenced Mermaid block carrying the engine's truncation counts and the caps it was rendered with; the complete node table (kind, node, links to) over an uncapped graph that tolerates dangling edges; composition of summary lines, one blank line and the body; the caps constant of 200 nodes / 400 edges; the shared truncation summary line built from a body's omitted counts and caps (absent when not truncated) so neither graph view formats it on its own
  - Observable: unit tests cover truncation at 47/48/49 characters and multibyte text, the task-label matrix (checked × optional × parallel), phase-label cases (absent, empty, padded), the fence shape with propagated truncation and caps, the node table with labelled, unlabelled and dangling edges, composition with an empty summary, and the truncation line present/absent; engine clippy with warnings denied and fmt pass
  - _Boundary: VizCommon_
  - _Requirements: 1.4, 2.2, 2.3, 4.3, 6.2, 6.3, 7.1, 7.2, 8.6_

- [ ] 2. Core: the three views
- [x] 2.1 (P) Build the status view and its fixed text
  - Touches only the status view file; nothing in the shared module file
  - Status model from one loaded feature: spec name, phase label, ready flag, approvals (all defaults when `spec.json` is unreadable), task totals (checked/total, parallel, optional) with one row per major task, requirement and criteria counts, component/file counts with the unassigned count, finding counts over the feature's findings only
  - Successor chain line derived from the engine's chain value: the path as given with the terminal appended when it is not already last, an empty path starting with the spec name, cycle shown as the path as given, and a superseded spec without a chain shown as its own name with no known successor
  - Text exactly in the design's status format: header block (with the chain line only when superseded), approvals table, tasks block with integer percentage (0 for 0/0) and the per-major table, figures block with `missing` for unreadable documents
  - Observable: in-module unit tests over hand-built feature and chain values cover defaults with unreadable `spec.json`, the 0/0 percentage, chain terminal (with and without the terminal already in the path), cycle and no-successor by direct construction, `missing` figures, and the full text of one hand-built feature against the design's format line by line (the fixture comparison belongs to 2.4)
  - _Boundary: StatusView_
  - _Depends: 1.1_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [ ] 2.2 (P) Build the spec graph view and its summary
  - Touches only the graph view file; nothing in the shared module file
  - Nodes layer by layer (requirements, criteria, leaf tasks — sub-tasks plus majors without sub-tasks — components, files assigned to a component) in document order, each added once, with the design's labels, kind markers and raw identities (`req:`, `crit:`, `task:`, `comp:`, `file:`); component labels obtained through the label seam scoped to the spec's declared topics, identity unchanged
  - Edges in layer order: requirement to criterion; criterion or whole requirement to task from `_Requirements:_` entries; task to component from `_Boundary:_` entries when `design.md` is present (unresolved boundary names on leaf tasks are counted from `join::Coverage.unknown_boundary_tasks` filtered to leaf tasks when `design.md` is present, amendment 2026-11-04, gap-analysis issue 8); component to file from the File Structure Plan; left-to-right orientation
  - Counting: entries unresolved against a present document counted as unresolved references, entries whose target document is absent neither drawn nor counted, unscoped leaf tasks, unassigned files, missing documents listed in document order
  - Nothing to draw when all three documents are absent; otherwise a partial graph from the readable documents
  - Summary lines per the design (counts and edge total; unscoped/unassigned/unresolved; missing documents by bare file name; the shared truncation line from 1.1)
  - Observable: unit tests cover the leaf-task rule, whole-requirement versus criterion references, unresolved counting only against present documents, the layer order of nodes and edges, a test labeler that changes a component's label but not its node identity, the orientation, the summary lines, and the nothing-to-draw case
  - _Boundary: SpecGraph_
  - _Depends: 1.1_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 5.8, 6.1, 6.4, 7.1_

- [ ] 2.3 (P) Build the task dependency view and its summary
  - Touches only the tasks view file; nothing in the shared module file
  - Every task (majors and sub-tasks) as a node in document order with the major/sub kind markers and done markers; top-down orientation
  - Edges in order: containment from each major to its sub-tasks; ordering edges between consecutive top-level entries and between consecutive sub-tasks of a major, skipped when the later task is parallel-capable (tasks the parser demoted to top level take part in the top-level chain); explicit `depends` edges from `_Depends:_` entries with unknown targets counted and a self-reference drawn as a self-loop
  - Statistics and summary lines per the design (done/total, majors, parallel, optional, explicit dependencies with unknown count, ordering and containment edge counts, the shared truncation line from 1.1)
  - Observable: unit tests cover ordering skips at both levels, containment, known, unknown and self-referencing dependencies, the orientation, the summary lines and every statistic
  - _Boundary: TaskGraph_
  - _Depends: 1.1_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.8, 7.1_

- [ ] 2.4 Write the engine's golden fixtures and the output lints
  - Golden files for the status view (`demo`, `old`), the spec graph and the task graph (diagram and table forms) generated once from the engine over the minimal fixture through the in-memory source and workspace (successor chain included for `old`), reviewed line by line against the design's formats and label catalogue, then committed
  - Mermaid shape lint (orientation line, node/class/edge line shapes, safe ids, escaped labels, endpoints among declared ids) and table lint (header, separator shape, equal cell counts) implemented in the new engine test file and applied to every golden diagram and table
  - Observable: `cargo test -p kiro-core` passes with the new test file; the six golden comparisons are byte-identical and every golden passes both lints
  - _Boundary: VizCoreTests_
  - _Depends: 2.1, 2.2, 2.3_
  - _Requirements: 7.3, 8.1, 8.3_

- [ ] 2.5 Write the engine's over-cap, partial, no-panic and determinism tests
  - Over-cap test built from generated requirements text (250 criteria) and tasks text (300 sub-tasks referencing them) loaded through the in-memory source and workspace: truncated body, the engine's ellipsis node, the truncation line with figures from the caps, a table form containing every node, and both lints passing on the truncated diagram
  - Partial graphs from the minimal fixture with the design document removed (no component or file nodes, `Missing: design.md`), with only the tasks document present, and with nothing present (nothing to draw)
  - Every view over garbage documents without panicking, using a minimal fixed-seed garbage generator copied into the new test file (no edit to any existing engine test); two builds of every view byte-identical
  - Observable: `cargo test -p kiro-core` passes with these cases added to the viz test file
  - _Boundary: VizCoreTests_
  - _Requirements: 4.7, 7.1, 7.2, 7.4, 7.6, 8.2_

- [ ] 3. Integration: plugin feature module
- [ ] 3.1 (P) Implement the `viz` feature module
  - Prerequisite: `plugins/kiro` from the plugin-core spec is implemented (dispatch, context, presentation, in-memory host)
  - Three command declarations (status, graph, tasks: spec-addressable, insert-capable, first token names the spec) with the design's titles; option parsing accepting `text` for graph and tasks and nothing for status, rejecting unknown options with the documented message before any read; the labeler constructor returning the identity labeler as the single substitution point for a vocabulary-backed labeler
  - Command bodies: status (resolve, load, successor chain when superseded, view, title with phase, level from the feature's findings); graph (view or the no-document error, body rendered with the caps constant, summary, title with drawn node count, level from missing documents, unresolved references and truncation); tasks (missing-tasks error, view, body, summary, title with done/total, level)
  - The module file declared in the plugin crate root with a crate-doc paragraph; no host call from the module; not yet in the dispatch module table (3.2 wires it), so the existing manifest test stays green
  - Observable: in-module unit tests cover the option table (empty, `text`, repeated `text`, `txt`, any option on status), the three titles and the level rule; plugin clippy with warnings denied and fmt pass
  - _Boundary: VizCommands_
  - _Depends: 2.1, 2.2, 2.3_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.4, 2.5, 2.6, 2.7, 3.4, 4.7, 5.7, 6.2, 7.5_

- [ ] 3.2 Wire the module into dispatch, the manifest and the README
  - Integration task across the plugin's dispatch and package files: one entry for the `viz` module in the dispatch module table; six manifest entries (three base commands, three insert variants) in the static-list order with the documented titles and the `Kiro` category; README rows for the three commands with their spec-addressable and insert-capable flags, the argument example `viz.graph <spec> text`, and the note that options follow the spec name and cannot be passed from the IDE's UI
  - Observable: the existing manifest consistency test passes with the six entries, the README command table equals the manifest's static list, and running `viz.status` through dispatch over the in-memory host with the minimal fixture produces a notification (smoke check only; the committed end-to-end scenarios are 3.3)
  - _Boundary: VizCommands_
  - _Requirements: 1.1, 8.5_

- [ ] 3.3 Write the plugin end-to-end tests for the `viz` commands
  - Over the in-memory host loaded with the engine's minimal fixture tree: activation registers the six static `viz` ids with their titles; the graph command with the active design file notifies the documented title with a body starting with the graph summary and returns a text containing the Mermaid fence; afterwards the per-spec variants of all three commands exist for both fixture specs; the task-graph insert variant produces exactly one insertion equal to the returned text and one notification; the `text` option returns the node table without a fence; an unknown option fails with the documented message and no notification; `viz.graph text` without an explicit spec and `viz.graph.demo` with argument `text` fail with plugin-core's documented errors; no active file with two known specs fails with the candidates message; the superseded fixture without `tasks.md` fails with the missing-document message; its status shows `Successor chain: old -> demo (terminal)`; the `Phase:` line of the status view equals that of the plugin's current-spec report for the same spec
  - Observable: `cargo test` from the plugin package passes with the new test file
  - _Boundary: VizPluginTests_
  - _Requirements: 1.1, 1.2, 1.4, 1.5, 2.1, 2.4, 2.5, 3.4, 4.7, 5.7, 8.4, 8.5_

- [ ] 4. Run the quality gate on both targets and verify the views in the host
  - Engine: tests, clippy with warnings denied, fmt check, `wasm32-wasip2` build; plugin package: tests, clippy for the native and wasm targets with warnings denied, fmt check, release component build
  - Host check (manual, non-blocking for the automated gate): rebuild the component, reload the dev plugin, open plugin-core's design document, run Spec Status and Spec Graph (notification titles name the spec), run Task Graph — Insert at Cursor in a scratch Markdown buffer and open the Markdown preview to confirm the fenced diagram renders with bracketed task labels and class lines
  - Observable: every automated command above succeeds; the manual preview check is recorded as done or deferred
  - _Boundary: VizCoreTests, VizPluginTests, VizCommands_
  - _Requirements: 7.3, 8.5, 8.6_
