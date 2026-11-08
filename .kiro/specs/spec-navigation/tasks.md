# Implementation Plan

- [x] 1. Foundation: nav module scaffold and fixture tree
- [x] 1.1 Create the nav module scaffold with the command table, final dispatch, shared helpers and view stubs
  - Prerequisite: the plugin package from plugin-core builds natively with its module contract, command context, argument request and output types in place
  - Directory module for navigation declared once in the crate root — the single crate-root line this task touches, so that the scaffold compiles; every other registration file (dispatch table, manifest, README) stays with 3.1
  - The module's root file holds the six command declarations with their titles and category (every command spec-addressable, only the context command insert-capable, every command reading its first argument token as the spec), the constants stating the static id count (seven) and the per-spec id count (six), the shared helpers, and the final module-contract implementation that routes `context` to the context view's run function, `files` and `task-files` to the coverage view's two functions and the three outline commands to the outline view's run function with the respective document kind
  - The four view files (comment syntax, context, coverage, outlines) are created here containing only the public signatures from the design, each body returning a single-line "not implemented" error or a placeholder value; tasks 2.1–2.6 replace the bodies of their own file only and never edit the module root file
  - Shared helpers: rejection of unexpected argument tokens with the message naming the command id and the first extra token, checkbox text (`[x]` / `[ ]`), the task line format `<id> <checkbox> <description> (<tasks path>:<line>)` from both a task reference and a task, and file-name extraction after the last slash
  - Observable: unit tests prove every declaration validates under the plugin's command rules, the static list derived from the declarations has exactly seven ids and exactly six declarations are spec-addressable, the unexpected-token message, the task line format and file-name extraction; `cargo build`, clippy with warnings denied and fmt pass from the plugin package with the stubs in place
  - _Boundary: NavModule, NavRegistration_
  - _Requirements: 1.1, 1.2, 1.3, 1.5, 1.6, 7.1, 7.5_

- [x] 1.2 (P) Create the navigation fixture tree and a loading smoke test
  - Roadmap listing `alpha`, `beta`, `gamma` in dependency order
  - `alpha`: tasks-generated metadata; requirements with criteria 1.1 and 1.2; design with components `Alpha` (two files including the shared file, one with a responsibility) and `Empty` (no files) in a table plan, plus a tree fence adding one unassigned file, and component headings under Components and Interfaces; tasks with a container major and an unscoped sub-task, a completed sub-task bound to `Alpha` referencing 1.1, 1.2 and a non-existent 9.9, an open sub-task bound to `Empty`, and a sub-task bound to the undefined component `Glue`
  - `beta`: tasks-generated; design mapping the shared file and one own file to `Beta`; one open task bound to `Beta` referencing 1.1
  - `gamma`: tasks-generated with no design document; requirements with 1.1; one task bound to `Gamma`
  - Observable: a smoke test in the end-to-end test file (created here, extended by 4.1 and 4.2) embeds the tree into the fake host and asserts through the engine that alpha and beta load with no missing-document finding and gamma reports exactly its design as missing
  - _Boundary: NavFixtures, NavTests_
  - _Requirements: 8.4_

- [ ] 2. Core: comment syntax and the three views
- [x] 2.1 (P) Implement comment-syntax selection and wrapping
  - Replaces the body of the comment-syntax file only
  - Five styles (double slash, hash, double dash, HTML, bare); the four extension tables and the extension-less basename table from the design; selection by the file name after the last slash and its last extension (a leading dot is not an extension), compared in ASCII lowercase, extension tables first, then basenames, else bare
  - Wrapping of one line per style without a trailing newline; HTML wrapping replaces every `--` inside the text with `- -`
  - Observable: unit tests cover one representative per table class, an uppercase extension, `Makefile` and `DOCKERFILE`, `.gitignore`, `data.json`, `archive.tar.gz`, a name without extension, and the `--` replacement
  - _Boundary: CommentSyntax_
  - _Depends: 1.1_
  - _Requirements: 3.2, 3.3, 3.4, 8.3_

- [x] 2.2 (P) Implement the ownership report of the context command
  - Replaces the report half of the context file; the reference half stays a stub until 2.3, so the insert-flag path keeps returning the stub's not-implemented error until 2.3 lands
  - Active-file checks: no active file and file outside the project produce their single-line errors; an active file under a spec directory produces the spec-document notice naming the document kind and pointing to the files command
  - Spec selection: the explicitly named spec alone, otherwise every known spec in roadmap order; with no known spec the plugin's standard resolution error is returned unchanged; unexpected tokens rejected before any document is loaded
  - Gathering: each checked spec loaded once through the context; a spec without a readable design classified as "no design"; otherwise the engine's owners of the file decide covered or not covered; for covered specs the task and requirement document paths and, per referenced criterion id, the line from the requirements document (criterion or whole requirement) or "not found"
  - Report text in the design's layout with the summary lines (file, checked list, covered by, not covered by, no design) before the per-spec blocks listing components, task lines with `tasks.md:line` and criteria with `requirements.md:line`; title from the file name and the covering spec names or "not covered"; warning level when nothing covers the file or any checked spec lacks a design
  - Observable: unit tests cover report text for covered / not covered / no design / several specs / several components, a missing criterion marked "not found", the title variants and the level rule; the active file is never read (only spec documents are loaded)
  - _Boundary: ContextView_
  - _Depends: 1.1_
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 7.2, 7.4, 7.6, 8.2, 8.3_

- [x] 2.3 Implement the reference line and the insert path of the context command
  - Blocked by 2.1 (uses the comment styles) and 2.2 (same file, same gathered report), so not parallel with its preceding peers
  - One `kiro-spec: …; components: …; tasks: …; requirements: …` line per covering spec (`none` for empty lists) wrapped in the active file's comment style, each ending with a line break, returned as the output text when the insert flag is set so that the notification shows exactly the inserted text
  - Errors when the insert flag is set and nothing can be inserted: the file is a spec document, or no checked spec covers it (naming the file and the checked specs); all other selection and active-file rules identical to the report path
  - Observable: unit tests cover the reference line with full and empty lists, the wrapped multi-spec text, and both nothing-to-insert errors
  - _Boundary: ContextView_
  - _Depends: 2.1_
  - _Requirements: 3.1, 3.5, 3.6, 8.3_

- [ ] 2.4 (P) Implement the by-component coverage view
  - Replaces the by-component half of the coverage file, including the shared boundary helpers; the by-task half stays a stub until 2.5
  - Unknown-boundary sections read from `join::Coverage.unknown_boundary_tasks` (amendment 2026-11-04, gap-analysis issue 8 — no local re-derivation); boundary labels `Name`, `Name (no files)`, `name (unknown)`, `unscoped`
  - Command path: no extra tokens allowed, current spec resolved through the context with its standard errors, the spec loaded once
  - Text in the design's layout: spec, design path, tasks path, counts line, then components in design order with their files (responsibility appended when the design states one) and the tasks bound to them, followed by the unassigned files, unscoped tasks and unknown-boundary sections when non-empty; missing design or tasks document marked `(missing)` with the dependent sections omitted and warning level
  - Observable: unit tests over in-memory documents matching the alpha fixture cover the counts line first, a component without files, an unassigned file, an unscoped task, an unknown boundary, a missing design and a missing tasks document, and the boundary labels
  - _Boundary: CoverageViews_
  - _Depends: 1.1_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 7.2, 7.4, 8.3_

- [ ] 2.5 Implement the by-task coverage view
  - Blocked by 2.4 (same file, reuses the boundary helpers), so not parallel with its preceding peer
  - Command path: at most one extra token (the task id), current spec resolved through the context with its standard errors, the spec loaded once
  - Text in the design's layout: spec, tasks path, design path, count line with the optional filter, then majors with indented sub-tasks, each with its boundary labels and the files of its known components (deduplicated, in boundary order); the filter narrows to one major with its sub-tasks or to one sub-task; an unknown id is an error naming the id and the tasks path; a missing tasks document gives the summary only at warning level; a missing design marks every boundary as unknown at warning level
  - Observable: unit tests cover the full task tree, the major filter, the sub-task filter, the unknown-id error, a missing tasks document and a missing design
  - _Boundary: CoverageViews_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 7.2, 7.4, 8.3_

- [ ] 2.6 (P) Implement the document outlines
  - Replaces the body of the outline file only
  - One outline function over a loaded spec and a document kind: headings become entries with their line and `#`-prefixed text; for the tasks document every task item becomes an entry with its line, checkbox, id and description, sub-tasks indented, merged with the headings by line; a missing document yields no outline
  - Outline text in the design's layout: spec, document path (`(missing)` when absent), entry count or `empty`, blank line, then `<line>: <text>` entries; warning level when the document is missing
  - Command path for the three outline commands: unexpected tokens rejected, current spec resolved through the context, title from the spec name and the document file name
  - Observable: unit tests cover a headings-only document, a tasks document with merged and indented task items, an empty document and a missing document
  - _Boundary: OutlineView_
  - _Depends: 1.1_
  - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 7.2, 7.4, 8.3_

- [ ] 3. Integration: register the navigation module in the plugin package
- [ ] 3.1 Add the module to the dispatch table, the manifest and the package documentation
  - Module table gains the navigation module at its canonical position (`ws, nav, viz, vocab, check`)
  - Manifest gains the seven project-independent navigation entries (six base commands then the insert variant) with the same titles and category as the declarations
  - README gains the command rows, the argument forms (`nav.context <spec>`, `nav.task-files <spec> [task-id]`, and that a per-spec variant combined with an argument must repeat the spec name), the comment-syntax table and the manual host check for the context command
  - Observable: the plugin's manifest consistency test passes with the navigation entries; running `nav.context` through the dispatch path reaches the view (no longer the "not available in this build" error)
  - _Boundary: NavRegistration_
  - _Depends: 2.3, 2.5, 2.6_
  - _Requirements: 1.1, 1.4, 8.5, 8.7_

- [ ] 4. Validation: end-to-end behaviour and the sandbox build
- [ ] 4.1 Write the end-to-end tests for activation, the context command and the insert variant
  - Extends the smoke test file from 1.2
  - Activation registers the seven navigation static ids among the static list
  - Context over the shared file: title names both specs, body holds alpha's task line with its `tasks.md` line, criteria 1.1 and 1.2 with `requirements.md` lines and 9.9 marked not found, gamma under "no design", warning level, and no read event for the code file; the per-spec variant restricts to one spec
  - Context error and notice paths: an uncovered file (not covered, checked list, warning), no active file, a file outside the root, a spec document as the active file, no known spec (roadmap removed) yielding the plugin's standard error, and an unexpected token
  - Insert variant: a Rust file yields one insert event equal to the two `// kiro-spec: …` lines and to the notification body; a Markdown file yields HTML-wrapped lines; an uncovered file yields the nothing-to-insert error with no insert event
  - Observable: every scenario asserts the notification title, level and body or the error line, the insert events and the recorded reads; the suite passes natively from the plugin package
  - _Boundary: NavTests_
  - _Depends: 1.2, 3.1_
  - _Requirements: 1.4, 1.6, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.10, 2.11, 3.1, 3.2, 3.5, 3.6, 7.3, 7.6, 8.1, 8.4_

- [ ] 4.2 Write the end-to-end tests for the coverage views, the outlines and notification clipping
  - Blocked by 4.1 (same test file), so not parallel with its preceding peer
  - Component view for alpha (counts line, `Empty` without files, unassigned file, unscoped task, unknown boundary `Glue`), the missing-design case via gamma's variant, the missing-tasks case with beta's tasks document removed from the host, and the unresolved-spec error with no active file and three known specs
  - Task view for alpha in full, filtered by a major, filtered by a sub-task, with an unknown id, and for gamma with every boundary unknown at warning level
  - The three outlines: alpha's design, alpha's tasks with merged and indented task items, gamma's requirements, gamma's missing design at warning level, and an empty requirements document reporting an empty outline
  - A synthetic spec with 400 design files whose notification body stays within the plugin's notification budget (asserted against the plugin's budget constant) with the omitted-lines trailer while the return message holds the full text
  - Observable: every scenario asserts the notification title, level and body or the error line; the suite passes natively from the plugin package
  - _Boundary: NavTests_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7, 6.1, 6.2, 6.3, 6.4, 6.5, 7.2, 7.3, 7.4, 8.1, 8.4_

- [ ] 4.3 Verify the sandbox build and lints, and run the manual host check
  - Release build for the sandbox target, clippy with warnings denied for the native and sandbox targets, and fmt check from the plugin package all pass with the navigation module included — these three automated checks are the completion condition
  - Manual check as documented in the README when a host is available: dev plugin reloaded in this repository, `crates/kiro-core/src/md.rs` open, the context command shows a notification naming `kiro-core` and `MarkdownScanner`; the insert variant adds a `// kiro-spec: kiro-core; …` line at the cursor; when no host is available the manual check is recorded as deferred in the completion note
  - Observable: the built component file is produced and all three automated checks exit zero; the manual check outcome (passed or deferred) is recorded in the task's completion note
  - _Boundary: NavRegistration_
  - _Requirements: 8.6, 8.7_
