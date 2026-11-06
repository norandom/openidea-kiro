# Brief: spec-visualization

## Problem

Kiro specs are text; their structure (phases and approvals, requirements → criteria → tasks → components → files, task dependencies) is invisible without reading everything. The plugin sandbox has no UI panels, so visualizations must be emitted as text (Mermaid / Markdown tables) through the command message, notifications, or editor insertion into a scratch buffer.

## Current State

- `kiro-core` provides the model (`spec.json` phase + approvals, criteria `N.M`, task tree with `_Requirements:_`/`_Depends:_`/`_Boundary:_`, File Structure Plan) and `render` (Mermaid/text backends with size caps and safe ids).
- `plugin-core` provides resolution, dispatch and `present`.
- cc-sdd puts Mermaid into `design.md` already; nothing renders the cross-document graph.

## Desired Outcome

`plugin.kiro.viz.*` commands that render, for the current feature (or one passed as arg): (a) `viz.status` — phase, approvals, task completion counts, and the superseded → successor chain resolved against known features; (b) `viz.graph` — Mermaid requirements → criteria → tasks → components → files; (c) `viz.tasks` — Mermaid task dependency graph from `_Depends:_` and ordering, marking `(P)` tasks and completion. Output goes to the command message (full) and notification (summary); `insert` as arg pastes the Mermaid block at the cursor. The diagram is the default output and the table form is available via the `text` option; the `.insert` variant (a distinct registered command id, reachable even though the IDE passes no argument) carries the full text, because the host discards the command's `Ok` message — the three-tier output contract of plugin-core (amendment 2026-11-04, confirmed by the round-2 review).

## Approach

- Thin command layer over `kiro_core::render`; this spec owns the graph *shapes* (which nodes/edges, labels, styling classes) and the text fallbacks, core owns Mermaid syntax safety and caps.
- Labels use vocabulary preferred labels when vocabulary-grounding is present (optional dependency through core's `vocab` API; absent vocabulary → raw names).
- Insertion only on explicit `insert` arg; never writes spec files.

## Scope

- **In**: `viz.status`, `viz.graph`, `viz.tasks`; graph shape definitions; text-table fallbacks; `insert` handling; golden-output tests on fixtures (including capped output and Mermaid validity checked by a small syntax lint in tests).
- **Out**: navigation (spec-navigation); term checks (spec-constraints); rendering code internals; host/UI changes.

## Boundary Candidates

- Graph shape (model → node/edge lists) — this spec.
- Mermaid/text emission — core `render`.

## Out of Boundary

- Code → spec lookup UX (spec-navigation).
- Vocabulary/ontology checks (vocabulary-grounding, spec-constraints).
- Writing diagrams into spec files.

## Upstream / Downstream

- **Upstream**: kiro-core (model, join, `render`), plugin-core; vocabulary-grounding (labels, optional).
- **Downstream**: users reviewing spec state in the IDE; review workflows (kiro-review / validate-impl) reference the same `N.M` and task ids the graph shows; cli-gate may expose the same renders as `kiro-cli viz`.

## Existing Spec Touchpoints

- **Extends**: none. **Adjacent**: spec-navigation (shared join), vocabulary-grounding (labels), cli-gate (same renders natively).

## Constraints

- Notification 16 KiB / insert 1 MiB: cap nodes/edges with an ellipsis node and say so in the output.
- No panics; large specs degrade to text tables within the 5 s budget.
- Mermaid validity: ids without `.`/spaces, quoted labels, no raw `<`/`>`; phase vocabulary is cc-sdd's (`initialized | requirements-generated | design-generated | tasks-generated`) plus `superseded`/`reserved` from the successor rule.
