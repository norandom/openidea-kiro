# Brief: spec-navigation

## Problem

Inside OpenIDEA there is no way to move between a kiro spec's phases (requirements / design / tasks) and the code those documents describe. From a code file you cannot tell which task or requirement owns it; from `design.md` you cannot see which files implement it. The plugin sandbox cannot open files or list directories, so this navigation has to be computed from readable files and surfaced through notifications, the command message, and optional editor insertion.

## Current State

- `kiro-core` provides the model and the file ↔ component ↔ task join: `design.md`'s File Structure Plan maps components to files; `tasks.md` `_Boundary:_` lines name components (not files, and only on tasks that need them); `_Requirements: N.M_` links tasks to criteria.
- `plugin-core` provides `host_source`, current-spec resolution (active file / arg / roadmap), dispatch and `present`.
- Nothing connects the active editor file to that join yet.

## Desired Outcome

`plugin.kiro.nav.*` commands answering both directions: "which spec / component / task / requirements own this code file?" and "which components, files and tasks does this spec cover?", presenting project-relative paths the user can open with quick-open, plus an optional reference line inserted at the cursor.

## Approach

- Code → spec: resolve the current spec; `owners_of(active_file)` from core returns components (File Structure Plan), tasks (`_Boundary:_`) and criteria (`_Requirements:_`); report as a notification and command message; `nav.context insert` inserts a reference using the active file's comment syntax (by extension: `//`, `#`, `--`, `<!-- -->`); fallback: bare path line — the specced behaviour for extensions without a known comment syntax (requirement 3.4, `CommentStyle::Bare`), confirmed by the round-2 review 2026-11-04).
- Spec → code: with a spec doc active or a feature arg, `nav.files` lists File Structure Plan files grouped by component (optionally filtered to one task's boundary); `nav.to-requirements|to-design|to-tasks` reports the sibling document path and its heading TOC with line numbers so the user can quick-open and jump.
- Degradation: files not in any File Structure Plan → "not covered by <feature>'s design"; tasks without `_Boundary:_` → listed as "unscoped"; a file matched by several components → all reported.

## Scope

- **In**: commands `nav.context`, `nav.files`, `nav.to-requirements`, `nav.to-design`, `nav.to-tasks`; presentation of join results; comment-syntax table for insertion; TOC extraction (from core's heading model); unit tests for presentation and comment syntax; fixture-based tests of end-to-end command output over `host_source` doubles.
- **Out**: opening files (host cannot); editing spec files; the join itself (kiro-core); diagrams (spec-visualization); term checks (vocabulary-grounding, spec-constraints).

## Boundary Candidates

- Presentation of owners/files/TOC — this spec.
- Join + path normalisation — core (`resolve`), shared with spec-visualization.

## Out of Boundary

- Generating diagrams (spec-visualization).
- Validating terms (spec-constraints).
- Creating/editing any file.

## Upstream / Downstream

- **Upstream**: kiro-core (`resolve`, model), plugin-core (dispatch, `host_source`, `present`).
- **Downstream**: users navigating specs in the IDE; spec-visualization reuses the same join; vocabulary-grounding supplies preferred labels for component/term names where available.

## Existing Spec Touchpoints

- **Extends**: none. **Adjacent**: plugin-core, spec-visualization (shared join), vocabulary-grounding (labels).

## Constraints

- No `read_dir`: candidate features come from current-spec resolution only.
- Active file may be unsaved — `read-file` returns buffer contents, which is what we want.
- Notification ≤ 16 KiB: lists are clipped by plugin-core's presentation budgets (nav has no caps of its own); the full list is in the command message.
- All command paths return `Err(String)` instead of panicking.
