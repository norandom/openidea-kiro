# Research & Design Decisions

## Summary
- **Feature**: `spec-navigation`
- **Discovery Scope**: Extension (light discovery) — adds the `nav` feature module to the plugin package defined by plugin-core, consuming kiro-core's join, outline and task model unchanged. No new runtime dependency.
- **Key Findings**:
  - plugin-core's command grammar allows exactly one modifier segment (`insert` or `<spec>`), command names are static, and the IDE passes no argument. A per-task command variant is therefore impossible; "filter to one task" can only be an argument form, so the UI-reachable per-task view must list every task.
  - plugin-core's `deliver` inserts the same text it notifies (`Output.text`). The reference line for `nav.context.insert` must therefore *be* the command's output text on the insert path, not a side channel.
  - kiro-core already supplies everything navigation needs as data: `join::owners_of`, `join::coverage`, `DesignDoc::{component, files_of, unassigned_files}`, `RequirementsDoc::{criterion, requirement}`, `TasksDoc::{all, task}`, each document's `outline: Vec<Heading>`, `Feature::path(Doc)`, and `resolve::feature_of_path`. The plugin adds no parsing or matching.
  - `Owners.covered` is defined as "at least one component matched"; a spec without a readable design yields `covered == false` just like a genuinely uncovered file, so the plugin must distinguish `design.is_none()` before interpreting `Owners` (requirement 2.5).

## Research Log

### Extension points in plugin-core
- **Context**: Where does a feature module plug in, and what does it receive?
- **Sources Consulted**: `.kiro/specs/plugin-core/design.md` — Commands (`FeatureModule`, `modules()`), Context (`CommandContext`), Args (`Request`), Present (`Output`, `deliver`), Registry (`CommandSpec`, `Positional`, budget), Manifest; `.kiro/specs/plugin-core/requirements.md` 9.1–9.4.
- **Findings**:
  - A module implements `FeatureModule { group, commands, run(ctx, cmd, req) -> Result<Output, String> }`, is listed once in `commands::modules()`, and declares its static ids in `plugin.toml` (checked by `tests/manifest.rs`).
  - `CommandContext` exposes `active_path: Option<ProjectPath>` (normalised), `known_names()` in roadmap order, `resolve(explicit)` with the two standard unresolved errors, `load(name) -> &Feature` (cached, four reads), and `is_readable`.
  - `Request` carries `spec` (validated), `insert`, and `rest` (tokens after the spec for `Positional::Spec`). With `Positional::Spec` the first token is always the spec; a variant plus an argument therefore requires the argument to repeat the spec name (`nav.task-files.demo` + `demo 2.1`) or the base command with `demo 2.1`.
  - Before `run`, Commands has already rejected an explicit spec whose `spec.json` is unreadable and has synchronised per-spec variants; the module never registers commands or calls the host.
  - `deliver` shows one notification (`clip_notification(text)`), inserts `clip_insert(text)` when the insert flag is set, and returns the full text. The level comes from `Output.level`.
- **Implications**: The nav module is pure presentation over `CommandContext` and kiro-core values; it owns no host call, no registration, no clipping. All six commands use `Positional::Spec` so the argument path mirrors the variants.

### Engine contracts consumed
- **Context**: Which kiro-core APIs cover requirements 2, 4, 5 and 6 without re-implementation?
- **Sources Consulted**: `.kiro/specs/kiro-core/design.md` — Join, DesignParser, TasksParser, RequirementsParser, Workspace, Resolver, MarkdownScanner; `.kiro/specs/kiro-core/requirements.md` 9.1–9.9.
- **Findings**:
  - `owners_of(design, tasks, query) -> Owners { covered, components, tasks: Vec<TaskRef>, criteria: Vec<String> }` — components in design order, tasks in document order, criteria sorted and deduplicated as written. Path matching (equality or suffix at a `/` boundary) lives in `path_matches`.
  - `coverage(design, tasks) -> Coverage { components: Vec<ComponentCoverage { name, files, tasks }>, unscoped_tasks, unassigned_files }` — unscoped = tasks declaring no boundary (any level; kiro-core 9.9).
  - A task whose boundary names a component the design does not define is neither in a component's task list nor in `unscoped_tasks`; core reports it as `TASK_BOUNDARY_UNKNOWN`. The plugin detects the case by `DesignDoc::component(name).is_none()` per boundary entry — a lookup, not a join.
  - `DesignFile.responsibility` is only reachable through `DesignDoc.files` / `files_of`; `ComponentCoverage.files` carries paths only.
  - `RequirementsDoc::criterion(id)` and `::requirement(id)` give the line for `N.M` and whole-requirement references `N`; a missing id is "not found" (requirement 2.6) and is already a core finding (`TASK_REQ_UNKNOWN`) that `ws.current` shows.
  - Outlines: every document carries `outline: Vec<Heading { level, text, line }>` built by the scanner outside fences; `TasksDoc::all()` iterates majors then subs in document order with `line`, `checked`, `sub`.
  - `resolve::feature_of_path(relative)` identifies a spec document from the active path (requirement 2.9); `Feature::path(Doc)` builds the document paths used in output.
- **Implications**: No path logic, matching, parsing or heading detection in the plugin. The only derived computations are lookups (`component(name)`, `criterion(id)`) and text formatting.

### Comment syntax for the inserted reference
- **Context**: Requirement 3.2–3.4 — choose comment markers from the file name.
- **Sources Consulted**: language conventions (single-line comment leaders); the project's own file types (`.rs`, `.toml`, `.md`, `.ps1`, `.wit`, `.json`).
- **Findings**: Four marker families cover every mainstream language; files without line comments (JSON, CSV, plain text) and unknown extensions fall back to a bare line. Extension-less build files (`Makefile`, `Dockerfile`, `Justfile`) are the only basename cases worth a rule. HTML-style comments cannot contain `--`; the reference text contains spec names with single dashes and ids, so a defensive replacement of `--` by `- -` keeps the comment well-formed.
- **Implications**: A constant table in one small module, tested per extension; no library.

### Notification size and caps
- **Context**: The brief suggests per-list caps with "… and K more" and the full list in the return message.
- **Sources Consulted**: plugin-core Present (`NOTIFICATION_BYTES = 15 KiB`, `clip_notification` via `clip_lines`, full text returned); plugin-core requirement 7.2.
- **Findings**: plugin-core already clips the notification at a line boundary and returns the full text as the command message, which is exactly the brief's two-tier behaviour. A module-level cap would either duplicate it or make the return message incomplete (one `Output.text`). Realistic sizes: kiro-core's design maps about 60 files → the by-component view is about 5 KiB; `nav.context` over 7 specs is under 1 KiB.
- **Implications**: No module-level caps. Every view prints its summary lines first so a clipped notification remains useful (requirement 7.2).

### Command budget
- **Context**: Requirement 1.5 and plugin-core's 256-id limit.
- **Sources Consulted**: plugin-core Registry (`static_commands`, `sync_variants`, budget arithmetic) and its worked example (which assumed 5 nav commands).
- **Findings**: nav contributes 6 base ids + 1 insert id = 7 static ids, and 6 variant ids per known spec (all six commands spec-addressable). Together with ws (3 static, 1 per spec): 10 + 7·n. Projected total across all groups (ws 2, nav 6, viz 3, vocab 3, check 4 base = 18; insert variants 7; spec-addressable 17 per spec): 25 + 17·n → 144 ids for the 7 roadmap specs; 13 specs fit under the 256 limit and the 14th is skipped, after which plugin-core skips later specs in roadmap order and reports them in `ws.specs`.
- **Implications**: The command set fits with margin; the count is stated in the design and asserted by a unit test (requirement 1.5).

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| Feature module over `CommandContext` (chosen) | One `nav` module implementing plugin-core's `FeatureModule`; pure formatting over engine values | Matches plugin-core's extension contract; no host calls; natively testable through `FakeHost` | Depends on plugin-core's `Output` shape (one text for notify and insert) | Same pattern as `ws` |
| Separate crate for navigation | `plugins/kiro-nav` linked into the plugin | Isolation | Duplicates the package scaffold; plugin-core's module table expects an in-crate module | Rejected |
| Navigation in kiro-core (text views) | Render owners/coverage text in the engine, plugin only forwards | Shared with CLI | The CLI has no "active file" concept; presentation strings would leak into the engine; steering says frontends own presentation | Rejected |

## Design Decisions

### Decision: `nav.context` queries every known spec unless one is named
- **Context**: The active code file lives outside `.kiro/specs/`, so plugin-core's resolution yields "only known spec" or an unresolved error; but the question is precisely *which* spec owns the file.
- **Alternatives Considered**:
  1. Use `ctx.resolve` like the other commands — fails with candidates in every multi-spec project.
  2. Query all known specs in roadmap order; restrict to one spec when named by argument or variant.
- **Selected Approach**: Option 2. `known_names()` drives the loop; each spec is loaded through `ctx.load` (four cached reads) and asked `owners_of`.
- **Rationale**: Answers the user's question directly; the variants still offer a single-spec view for shared files (e.g. the root `Cargo.toml`, which both kiro-core and plugin-core touch).
- **Trade-offs**: Up to 4·n reads; within the 5 s budget for n ≤ 64 (plugin-core 8.3 arithmetic).
- **Follow-up**: End-to-end test with two covering specs and one spec without design.

### Decision: the insert path's output text is the reference
- **Context**: plugin-core's `deliver` inserts `Output.text`; the reference must be a comment line, not the ownership report.
- **Alternatives Considered**:
  1. A separate command `nav.reference` (+ `.insert`) — one more base id and one more variant per spec, and a redundant non-insert form.
  2. `nav.context` insert-capable; when `req.insert` is set the module returns the reference text as `Output.text` — the notification then shows exactly what was inserted.
- **Selected Approach**: Option 2.
- **Rationale**: Keeps plugin-core's invariant "notified text == inserted text", matches the brief's `nav.context insert`, and saves ids.
- **Trade-offs**: The insert form does not show the detailed report; the user runs the base command for it.
- **Follow-up**: Test that `nav.context.insert` produces one `Insert` event whose text equals the notification body.

### Decision: per-task filter as an argument on a distinct all-tasks command
- **Context**: "Filter to one task" cannot be a registered variant (one modifier segment, static names, and per-task ids would exhaust the budget).
- **Alternatives Considered**:
  1. Only `nav.files <spec> <task>` — unreachable from the UI.
  2. `nav.task-files`: UI form lists every task with its boundary files; argument form `nav.task-files <spec> <task-id>` narrows to one task (major tasks include their sub-tasks).
- **Selected Approach**: Option 2; `nav.files` stays the by-component transposition without a filter.
- **Rationale**: Both views are reachable from Search Everywhere; the argument form is the only per-task filter the host can express.
- **Trade-offs**: One more base command (+1 variant per spec).
- **Follow-up**: Document the argument form in the README command table.

### Decision: no module-level caps; summary first
- See "Notification size and caps" above. Recorded here because it departs from the brief's wording.

### Decision: module layout `src/nav/` with four files
- **Context**: plugin-core says each sibling adds "one module file"; the nav code has four separable concerns (command table and shared helpers, ownership + reference, coverage views, outlines) plus the comment table.
- **Selected Approach**: A directory module `src/nav/{mod.rs, comment.rs, context.rs, files.rs, docs.rs}` declared by one `pub mod nav;` line in `lib.rs`. Dependency direction inside: `comment` → `context`; `mod` (helpers) → `context`, `files`, `docs`; the three command files never import each other.
- **Rationale**: One declaration in `lib.rs` as plugin-core expects, while tasks for context, files and docs can run in parallel on disjoint files.

### Decision: reuse of plugin-core's "no spec is known" wording
- **Context**: `nav.context` without an explicit spec and with no known spec needs the same error as plugin-core's `resolve`, but it does not call `resolve` (it does not want a single spec).
- **Selected Approach**: The nav module calls `ctx.resolve(None)` in that one case and returns its error unchanged (with no known spec and no active spec document, `resolve` can only fail with the "no spec is known" message). No duplicated string.

## Synthesis Outcomes
- **Generalization**: `nav.files` and `nav.task-files` are two renderings of one `Coverage` value; `nav.to-*` are one outline renderer parameterised by `Doc`. The task-item merge for `tasks.md` is the only document-specific outline rule.
- **Build vs. adopt**: Everything structural is adopted from kiro-core and plugin-core; the only built pieces are the comment table and text formatting.
- **Simplification**: No module-level caps; no separate reference command; no `nav.files` task filter (lives on `nav.task-files`); no own error-message catalogue beyond five messages.

## Risks & Mitigations
- kiro-core's `Coverage` omits tasks with an unknown boundary — the plugin lists them from `TasksDoc` using `DesignDoc::component`; if kiro-core later adds them to `Coverage`, the plugin's lookup becomes redundant but not wrong (revalidation trigger).
- `Positional::Spec` makes `nav.task-files.<spec>` + `"2.1"` fail as "spec named twice" — documented argument form `nav.task-files <spec> <task-id>`; end-to-end test covers it.
- Large task lists in `nav.task-files` for a UI user — summary first; plugin-core clips at 15 KiB; the argument form narrows.
- Component names in the HTML comment style — `--` replaced by `- -`; unit test.

## References
- `.kiro/specs/plugin-core/design.md` — FeatureModule, CommandContext, Request, Output, Registry budget, message catalogue.
- `.kiro/specs/kiro-core/design.md` — Join (`owners_of`, `coverage`), DesignParser, TasksParser, RequirementsParser, Resolver (`feature_of_path`), Workspace (`Feature::path`).
- `.kiro/steering/tech.md` — host API facts (no argument from the UI, `Ok` message discarded, 16 KiB / 1 MiB clips, 256 commands).
