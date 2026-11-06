# Research & Design Decisions

## Summary
- **Feature**: `spec-visualization`
- **Discovery Scope**: Extension (light discovery). The feature plugs a `viz` feature module into plugin-core's dispatch and builds view models over kiro-core's spec model and renderers. No new library, no host change, no new I/O.
- **Key Findings**:
  - **plugin-core delivers one result text to every channel.** `present::deliver(host, out, insert)` shows `clip_notification(out.text)`, inserts `clip_insert(out.text)` when the insert flag is set, and returns `out.text` unclipped. The brief's "notification (summary) / command message (full)" split cannot be two different texts without changing plugin-core; it becomes an ordering rule instead: summary lines first, blank line, body. A clipped notification therefore always shows the summary, and the fenced Mermaid block is what gets inserted.
  - **The IDE UI passes no argument** (plugin-core research, verified against the host's `bridge.ts`/`plugin.rs`). Per-spec variants carry the spec name; nothing in the UI can carry the `text` option. The table form is still worth having (complete output for programmatic callers, tests and the future `kiro-cli viz`), but the design documents it as argument-only and keeps the capped diagram as the UI default.
  - **kiro-core's `render` never imports the model**, and kiro-core's boundary explicitly leaves "graph shapes for status/graph/tasks views and vocabulary labels" to this spec. Sibling specs already add modules to kiro-core (`vocab`, `checks`), so a pure `kiro_core::viz` module is the established place for the shapes: it keeps the plugin thin (steering: "engine first, frontends thin") and lets cli-gate reuse the views byte for byte.
  - **Argument grammar constraint**: with `Positional::Spec`, the first remaining token is the spec; an option token must follow it (`viz.graph demo text`). A variant id plus a lone `text` token yields plugin-core's "spec named twice" error — accepted as-is, documented in the README rows this spec adds.

## Research Log

### Output channels and the summary-first rule
- **Context**: The brief routes the full output to the command message and a summary to the notification; plugin-core's presentation contract has one text.
- **Sources Consulted**: `.kiro/specs/plugin-core/design.md` (Present: `Output`, `deliver`, `clip_notification`, `clip_insert`; Commands: `run_command` returns `Ok(Some(full text))`), `.kiro/specs/plugin-core/research.md` (UI discards `Ok`, passes `null`), `.kiro/specs/kiro-core/design.md` (`clip_lines` cuts at the last newline before the limit and appends a trailer).
- **Findings**: One `Output { title, level, text }` per command; clipping is line-based, so leading lines survive. Insertion inserts the same text.
- **Implications**: Every view's text is `summary lines + "\n" + body`. The body of a diagram is a fenced ```` ```mermaid ```` block so that insertion into Markdown renders. The title carries the key figure because titles are never clipped below 256 bytes.

### Where the shape builders live
- **Context**: The brief says "this spec owns the graph shapes; core owns Mermaid syntax safety and caps". Steering says all rendering logic is in kiro-core and frontends only adapt I/O; cli-gate "may expose the same renders as `kiro-cli viz`".
- **Sources Consulted**: `.kiro/steering/tech.md` (Key Technical Decisions), `.kiro/steering/structure.md` (module map), `.kiro/specs/kiro-core/design.md` (Out of Boundary: `vocab` and `checks` modules are created by sibling specs; `render` depends on `finding` only; Allowed Dependencies), `.kiro/specs/plugin-core/design.md` (feature modules "know their commands but no host calls"; allowed kiro-core modules).
- **Findings**: kiro-core's dependency direction ends at `resolve`; a `viz` module placed to the right of `resolve` may import the model, `source::Feature`, `resolve::SuccessorChain` and `render` without violating any rule. The plugin's module would then be a thin `FeatureModule` (command declarations, option parsing, titles, levels).
- **Implications**: Shapes, summaries and the table fallback are pure functions in `kiro_core::viz`; the plugin module `plugins/kiro/src/viz.rs` adapts them to `Output`. Both are owned by this spec. kiro-core's `lib.rs` gains one `pub mod viz;` line (same pattern as `vocab`/`checks`).

### Mermaid label and class behaviour relied upon
- **Context**: Task labels carry `[x]`/`[ ]` and `(P)`; kind markers are emitted as `class <id> <name>` lines without `classDef`.
- **Sources Consulted**: kiro-core MermaidRenderer contract (`    <id>["<label>"]`, escaping of `"`, `<`, `>`; `class <id> <class>` after the node); Mermaid flowchart syntax documentation (https://mermaid.js.org/syntax/flowchart.html — quoted node text, `class` statement).
- **Findings**: Mermaid accepts brackets and parentheses inside quoted node text; a `class` statement naming a class without a `classDef` is accepted and renders unstyled. Neither point was verified by running Mermaid during this design.
- **Implications**: Labels may contain `[x]`, `(P)` and `:`; kind markers are semantic hooks only (no styling is emitted; design-principles ask for pure Mermaid anyway). Follow-up: the golden tests include a lint for the engine's shape rules (7.3), and the host check in the task plan renders one inserted diagram in a Markdown preview to confirm both assumptions.

### Model facts the shapes depend on
- **Sources Consulted**: `.kiro/specs/kiro-core/design.md` — TasksParser (`Task { id, major, sub, checked, optional, parallel, requirements, depends, boundary, subtasks, counts }`, `TasksDoc::all()` majors-then-subs in document order, `counts` = `{1, checked}` for a major without sub-tasks), RequirementsParser (`Requirement { id "1" | "A1", kind, title, criteria }`, `Criterion { id "N.M", text }`, `criterion(id)`, `requirement(id)`), DesignParser (`Component { name, files }`, `DesignFile { path, component }`, `component(name)` case-insensitive, `files_of`, `unassigned_files`), SpecMetaParser (`Phase`, `Approvals`, `phase_raw`, `topics`), Workspace (`Feature { meta, requirements, design, tasks, findings }`, `phase()`), Resolver (`SuccessorChain { path, terminal, cyclic, findings }`, `Workspace::successor_chain(&mut self, feature)`), RenderGraph (`Graph`, `Node { id, label, class }`, `Edge { from, to, label }`, `Direction`, `Caps` default 200/400, `IdMap` sanitising, `Rendered { text, truncated, omitted_nodes, omitted_edges }`), TextRenderer (`table`, `graph_text`).
- **Findings**: Everything the three views need is already in the model; the join's `names_equal` rule (trimmed, ASCII case-insensitive) is what `DesignDoc::component` uses, so boundary → component matching needs no new code. `successor_chain` needs `&mut Workspace`, which `CommandContext.workspace` exposes as a public field.
- **Implications**: No kiro-core API gap. Node identity uses raw ids (`req:1`, `crit:1.2`, `task:2.1`, `comp:<design name>`, `file:<path>`); `IdMap` makes them Mermaid-safe and unique, so the views never sanitise ids themselves.

### Vocabulary label hook
- **Context**: vocabulary-grounding is being specified in parallel; its brief promises "preferred labels exposed for navigation/visualization through the same scoped API" but its API shape is not final.
- **Sources Consulted**: `.kiro/specs/vocabulary-grounding/brief.md`; kiro-core design (`SpecMeta.topics`).
- **Findings**: The only stable facts are: labels are looked up by raw name within the spec's declared topics; absence (no file, no topics, no feature) must degrade to raw names silently.
- **Implications**: `kiro_core::viz::Labeler` is a one-method trait (`component_label(raw, topics) -> Option<String>`) with `RawLabeler` returning `None`. The plugin constructs the labeler in one function; vocabulary-grounding (or a later integration task) replaces that constructor with a vocabulary-backed implementation without touching the shapes. Node ids stay raw, so labels cannot merge nodes.

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Shapes in the plugin module only | Build `Graph` values inside `plugins/kiro/src/viz.rs` | Fewest files touched | Not reusable by cli-gate; plugin stops being thin; harder to golden-test under the wasm runner | Rejected |
| Shapes as a kiro-core module (`viz`) consumed by a thin plugin module | Pure view builders + summaries + table fallback in core; plugin adapts to `Output` | Steering-compliant, CLI reuse, native golden tests, no I/O | One extra `pub mod` line in kiro-core's `lib.rs` | **Selected** |
| Extra base commands for the table form (`viz.graph-text`) | Make the table reachable from the UI | UI-reachable fallback | Two more base commands, two insert variants and two per-spec variants per spec against the shared budget; deviates from the agreed command list | Rejected; `text` stays an argument option |

## Design Decisions

### Decision: One result text, summary first, fenced diagram body
- **Context**: plugin-core's single-text presentation vs the brief's summary/full split.
- **Alternatives Considered**:
  1. Change plugin-core's `Output` to carry a separate notification text — out of boundary.
  2. Summary lines first, blank line, body; fenced Mermaid body — selected.
- **Selected Approach**: `viz::compose(summary, body)`; diagram bodies wrapped in ```` ```mermaid ```` fences by `render_body`.
- **Rationale**: Works with the existing contract, survives clipping, inserts as renderable Markdown.
- **Trade-offs**: The notification shows Mermaid source after the summary (acceptable: the brief expects text output).
- **Follow-up**: End-to-end test asserts the notification body starts with the summary and that the inserted text equals the returned text up to clipping.

### Decision: Leaf tasks in `viz.graph`, all tasks in `viz.tasks`
- **Context**: Majors with sub-tasks carry no annotations in cc-sdd; drawing them in the spec graph adds orphan nodes, while the task graph needs them for containment and ordering.
- **Selected Approach**: `viz.graph` draws sub-tasks plus majors without sub-tasks (the same rule `Task.counts` uses); `viz.tasks` draws every task with containment edges.
- **Trade-offs**: A major that carries annotations *and* has sub-tasks is not drawn in `viz.graph`; its annotations are ignored there (documented).

### Decision: Ordering edges follow the task-generation rule
- **Context**: "ordering" is informal in `tasks.md`; the project's task rules define `(P)` as "no dependency on its immediately preceding peers".
- **Selected Approach**: Chain edges `order` between consecutive siblings (sub-tasks within a major; majors at top level) except into a `(P)` task; explicit `_Depends:_` edges `depends`; both labelled so the diagram distinguishes them.
- **Trade-offs**: Transitive ordering is implied by the chain, not drawn. Tasks the parser demoted to top level (`TASK_HIERARCHY`) are treated as top-level entries in the chain — the view follows the parser's placement rather than re-deriving hierarchy.

### Decision: Caps as a viz constant, counts carried in the body
- **Context**: kiro-core documents `Caps` defaults (200 / 400) in a comment but declares no `Default` impl; the truncation line must name the caps used.
- **Selected Approach**: `viz::CAPS` constant; `Body` carries the `Caps` it was rendered with; summaries read figures from the body.
- **Rationale**: No reliance on an undeclared impl; the text can never disagree with the caps applied.

### Decision: Findings counted over the feature only
- **Context**: `SuccessorChain.findings` may carry `SPEC_SUPERSEDED_CYCLE`; counting it in the status line while the level ignores it would make line and level disagree.
- **Selected Approach**: `StatusView` counts `feature.findings` only — the same set `ws.current` and the level use; the chain's own state is shown by the `(cycle)` marker.

### Review log
- Independent design review (fresh reviewer, 2026-11-03): NEEDS_FIXES with 10 local findings (chain path derivation, `Caps::default` absent upstream, findings set vs level, `phase_label` duplication, `Doc` display name, option reachability, `join` import, golden derivability, over-cap test construction, demoted-task ordering). All repaired in the design and verified by the same reviewer in a second pass (PASS, no new contradiction) before the design was approved.

### Decision: Truncation keeps the capped diagram; the complete table is opt-in
- **Context**: Brief asks for an ellipsis node *and* text-table degradation.
- **Selected Approach**: Diagram form = engine caps (200/400) with the engine's ellipsis node and a summary line naming the omission and the `text` option; table form = `node_table` over the uncapped graph (one row per node, outgoing links), subject only to delivery clipping.
- **Rationale**: Deterministic, no hidden mode switch, both forms golden-tested.

### Decision: Kind markers as node classes without styling
- **Selected Approach**: Classes `req`, `crit`, `task`, `task_done`, `comp`, `file` (spec graph) and `major`, `major_done`, `task`, `task_done` (task graph); no `classDef`.
- **Rationale**: Core's `Graph` has no styling slot; design principles require pure Mermaid; classes remain usable by anyone pasting a `classDef` block.

## Risks & Mitigations
- Mermaid rejects a label shape the engine escapes incorrectly — the lint in `tests/viz.rs` checks the engine's shape rules on every produced diagram, and the host check renders one inserted diagram in a Markdown preview.
- Large specs exceed the notification budget — the summary comes first; the diagram is capped; the table form is complete in the return message and the insertion.
- Vocabulary API shape changes — the `Labeler` seam isolates the shapes; only the plugin's one constructor changes.
- Borrow juggling around `successor_chain(&mut)` in the plugin — compute the chain before taking the `&Feature` used by `status_view`; `load` is cached, so the second call performs no reads.

## References
- `.kiro/specs/kiro-core/design.md` — model, join, resolver, render contracts consumed verbatim.
- `.kiro/specs/plugin-core/design.md` — `FeatureModule`, `CommandSpec`, `Request`, `CommandContext`, `Output`, `deliver`, message catalogue, budget worked example.
- `.kiro/specs/vocabulary-grounding/brief.md` — preferred labels promised through a topic-scoped API.
- `.opencode/skills/kiro-spec-tasks/rules/tasks-generation.md` — the `(P)` and ordering semantics the task graph draws.
- Mermaid flowchart syntax — https://mermaid.js.org/syntax/flowchart.html (quoted node text, `class` statement).
