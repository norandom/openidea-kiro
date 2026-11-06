# Requirements Document

## Introduction

spec-navigation adds the `nav.*` command group to the Kiro plugin for OpenIDEA. Developers who run cc-sdd / kiro specs can read `design.md`, `tasks.md` and `requirements.md` in the editor, but nothing connects those documents to the code they describe: from a code file there is no way to learn which spec, component, task or requirement owns it, and from a spec document there is no way to see which files and tasks it covers. The host sandbox cannot open files or list directories, so this navigation has to be computed from readable documents and surfaced through notifications, the command's return message and, on request, a single reference line inserted at the cursor.

This spec answers both directions. Code → spec: `nav.context` reports every known spec whose design maps a file to the active editor file, with the owning components, the tasks bound to them and the requirement criteria those tasks reference; `nav.context.insert` writes a one-line reference in the active file's comment syntax. Spec → code: `nav.files` lists the design's files grouped by component with the implementing tasks, `nav.task-files` lists the files each task's boundary covers, and `nav.to-requirements`, `nav.to-design` and `nav.to-tasks` report the sibling document's path and heading outline with line numbers so the user can quick-open and jump. Every path is project-relative; every position is `path:line`.

Two upstream facts shape the command set. The IDE's own UI passes no argument to plugin commands and discards the command's return message, so each distinct choice the user can make from Search Everywhere must be its own command or per-spec variant, and the notification is the primary output. The file ↔ component ↔ task join and current-spec resolution belong to the engine and the plugin core; this spec only selects which specs to ask, asks, and presents.

## Boundary Context

- **In scope**: the six base commands `nav.context`, `nav.files`, `nav.task-files`, `nav.to-requirements`, `nav.to-design`, `nav.to-tasks` and the `nav.context.insert` variant; selecting which specs to query (one explicit spec or every known spec); presentation of ownership, coverage and outline results as text; the comment-syntax table for the inserted reference; degradation for files not covered, tasks without boundary, boundaries naming no component, files without component, and missing documents; native tests for presentation and comment syntax and end-to-end tests over a simulated host; the manifest entries for the navigation commands.
- **Out of scope**: opening files or moving the cursor (the host offers no such operation); creating or editing any project file; path matching and the file ↔ component ↔ task join, heading outlines and task trees (engine); command dispatch, per-spec variant registration, the command budget, argument tokenisation, current-spec resolution, notification clipping and delivery (plugin core); diagrams (spec-visualization); vocabulary labels and term checks (vocabulary-grounding, spec-constraints).
- **Adjacent expectations**: the engine provides, per spec, the owners of a file (components, tasks, criteria), the coverage listing (components with files and tasks, unscoped tasks, unknown-boundary tasks, unassigned files), the heading outline of each document and the task tree with line numbers; the plugin core routes `nav.*` commands to this feature, hands it the project context (root, normalised active file, known specs in roadmap order, documents loaded on request) and the parsed arguments, registers per-spec variants for spec-addressable commands within the command limit, resolves the current spec with its standard errors, and shows one notification per command clipped at a line boundary; the manifest consistency test of the plugin core covers the navigation commands once they are declared.

## Requirements

### Requirement 1: Navigation Command Set
**Objective:** As a developer using OpenIDEA, I want the navigation commands available from Search Everywhere and the keymap without typing any argument, so that moving between a spec and its code is one action.

#### Acceptance Criteria
1. The Kiro plugin shall provide the base commands `nav.context`, `nav.files`, `nav.task-files`, `nav.to-requirements`, `nav.to-design` and `nav.to-tasks`, each with a title starting with `Kiro:` and the category `Kiro`, declared through the plugin's feature-module contract so that dispatch, spec resolution, argument parsing and presentation are those of the plugin core.
2. The Kiro plugin shall make every navigation base command spec-addressable, so that a per-spec variant `nav.<name>.<spec>` exists for each known spec and behaves exactly as the base command with that spec named explicitly.
3. The Kiro plugin shall make `nav.context` insert-capable and no other navigation command insert-capable, so that `nav.context.insert` is the only navigation command that writes into the editor.
4. The Kiro plugin shall declare the navigation commands' project-independent ids — the six base commands and `nav.context.insert` — in the plugin manifest with the same titles and category as at registration.
5. The Kiro plugin shall contribute exactly seven project-independent command ids and exactly six command ids per known spec, so that the plugin's command limit is predictable for the plugin's other command groups.
6. If a navigation command is run with argument tokens beyond those it defines (a spec name for every command, and one task id after the spec name for `nav.task-files`), the Kiro plugin shall return an error naming the first unexpected token and shall produce no navigation output.

### Requirement 2: Code to Spec Context
**Objective:** As a developer editing a code file, I want to know which spec, components, tasks and requirements own it, so that I can find the governing documents without leaving the editor.

#### Acceptance Criteria
1. When `nav.context` runs with an active file inside the project and no spec named explicitly, the Kiro plugin shall check every known spec and report, for each spec whose design maps a file to the active file: the spec name; the owning components in design order; the tasks bound to those components with id, completion state, description and `tasks.md` line; and the requirement criteria those tasks reference with their `requirements.md` line when the criterion exists.
2. When `nav.context` runs with a spec named explicitly by argument or per-spec variant, the Kiro plugin shall check only that spec.
3. When the active file is mapped by more than one component of a spec, or by more than one spec, the Kiro plugin shall report every match.
4. If no checked spec maps the active file, the Kiro plugin shall report that the file is not covered, name the specs that were checked, and use warning level.
5. If a checked spec has no readable design document, the Kiro plugin shall list that spec as unverifiable because its design is missing, separately from the specs that do not cover the file.
6. When a reported task references a criterion id that does not exist in the spec's requirements document, the Kiro plugin shall list the id as written and mark it as not found.
7. If no editor is focused when `nav.context` runs, the Kiro plugin shall return an error telling the user to open a file first.
8. If the active file lies outside the project root when `nav.context` runs, the Kiro plugin shall return an error naming the file as outside the project.
9. When the active file is itself a document under `.kiro/specs/<spec>/`, the Kiro plugin shall report which spec and document it is and point to `nav.files` instead of reporting the document as not covered.
10. If no spec is known and no spec is named explicitly, the Kiro plugin shall return the plugin's standard "no spec is known" error.
11. The Kiro plugin shall title the `nav.context` notification with the active file's name followed by the outcome: the names of the covering specs, or "not covered".

### Requirement 3: Reference Insertion
**Objective:** As a developer, I want a one-line reference to the owning spec inserted at the cursor in my file's comment syntax, so that the code carries a pointer back to its spec, tasks and requirements.

#### Acceptance Criteria
1. When `nav.context` runs with the insert flag and at least one checked spec covers the active file, the Kiro plugin shall insert at the cursor one line per covering spec, each terminated by a line break, stating the spec name, the owning component names, the bound task ids and the referenced criterion ids, and shall show the inserted text in the notification.
2. The Kiro plugin shall wrap each inserted line in the comment syntax selected by the active file's extension, compared case-insensitively: `//` for C, C++, C#, Java, Kotlin, Swift, Go, Rust, JavaScript, TypeScript, Dart, Scala, Groovy, PHP, Zig, WIT and Protocol Buffers sources; `#` for Python, Ruby, shell, PowerShell, TOML, YAML, Perl, R, CMake, Elixir, Nim and Terraform sources; `--` for SQL, Lua, Haskell, Elm, Ada and VHDL sources; `<!-- -->` for Markdown, HTML, XML, SVG, Vue, Svelte, XAML and MSBuild project sources.
3. When the active file has no extension and its name is `Makefile`, `Dockerfile` or `Justfile`, the Kiro plugin shall use the `#` comment syntax.
4. If the extension is absent or not in the comment-syntax table and the name is not one of the listed extension-less names, the Kiro plugin shall insert the line without comment markers, so that the reference is still inserted.
5. If no checked spec covers the active file when the insert flag is set, the Kiro plugin shall return an error stating that there is nothing to insert and naming the file, and shall insert nothing.
6. While the insert flag is set, the Kiro plugin shall apply the same spec selection, active-file checks and coverage rules as `nav.context` without the flag.

### Requirement 4: Files by Component
**Objective:** As a developer reading a spec, I want the files its design covers listed by component together with the tasks that implement each component, so that I see the implementation layout at a glance.

#### Acceptance Criteria
1. When `nav.files` runs and the current spec resolves, the Kiro plugin shall report the spec name and its design path, then for each component in design order: its files as project-relative paths with the responsibility the design states for each, and the tasks whose boundary names the component with id, completion state, description and `tasks.md` line.
2. The Kiro plugin shall list components that map no file, marked as having no files.
3. The Kiro plugin shall list design files assigned to no component under a separate unassigned-files heading.
4. The Kiro plugin shall list tasks that declare no boundary under a separate unscoped-tasks heading with id, completion state, description and line.
5. The Kiro plugin shall list tasks whose boundary names no component of the design under a separate unknown-boundary heading with the names as written.
6. The Kiro plugin shall begin the `nav.files` output with counts of components, files, tasks, unscoped tasks and unassigned files.
7. If the resolved spec's design document is missing, the Kiro plugin shall report its expected path as missing with warning level and shall still list the tasks when the tasks document is readable.
8. If the resolved spec's tasks document is missing, the Kiro plugin shall report its expected path as missing with warning level and shall still list components and files.
9. If the current spec cannot be resolved, the Kiro plugin shall return the plugin's standard unresolved-spec error, so that the per-spec variants are offered.

### Requirement 5: Files by Task
**Objective:** As a developer picking up a task, I want the files that the task's boundary covers, so that I know what to touch and what to leave alone.

#### Acceptance Criteria
1. When `nav.task-files` runs without a task id and the current spec resolves, the Kiro plugin shall list every task in document order — major tasks followed by their indented sub-tasks — with id, completion state, description, `tasks.md` line and boundary component names, and under each boundary component the component's files as project-relative paths.
2. When `nav.task-files` runs with a task id after the spec name, the Kiro plugin shall list only that task and, for a major task, its sub-tasks.
3. If the given task id does not exist in the resolved spec's tasks document, the Kiro plugin shall return an error naming the id and the tasks path.
4. The Kiro plugin shall mark tasks without a boundary as unscoped, boundary names that match no design component as unknown, and boundary components that map no file as having no files.
5. The Kiro plugin shall begin the `nav.task-files` output with the spec name, the tasks path and counts of tasks, completed tasks and unscoped tasks.
6. If the resolved spec's tasks document is missing, the Kiro plugin shall report its expected path as missing with warning level and no task list.
7. If the resolved spec's design document is missing, the Kiro plugin shall report its expected path as missing with warning level and shall list every boundary name as unknown.

### Requirement 6: Document Outlines
**Objective:** As a developer, I want the path and outline of the current spec's requirements, design or tasks document with line numbers, so that I can quick-open the document and jump to a section.

#### Acceptance Criteria
1. When `nav.to-requirements`, `nav.to-design` or `nav.to-tasks` runs and the current spec resolves, the Kiro plugin shall report the spec name, the project-relative path of the respective document, and its outline as one line per heading with the heading's 1-based line number, level and text, in document order.
2. When `nav.to-tasks` runs, the Kiro plugin shall include every task item in the outline with its line, id, completion state and description, sub-tasks indented under their major task, merged with the headings in line order.
3. If the requested document is missing, the Kiro plugin shall report its expected path as missing with warning level and no outline.
4. When the requested document has no heading (and, for the tasks document, no task), the Kiro plugin shall state that the outline is empty.
5. The Kiro plugin shall title each outline notification with the spec name and the document's file name.

### Requirement 7: Presentation and Degradation
**Objective:** As a developer, I want navigation output I can act on with quick-open and that stays readable when clipped, so that large specs remain navigable.

#### Acceptance Criteria
1. The Kiro plugin shall present every file path in navigation output as project-relative with `/` separators and every document position as `<path>:<line>`, so that the IDE's quick-open accepts it as typed.
2. The Kiro plugin shall place the summary lines of every navigation output (spec, document path, counts, outcome) before its detail lists, so that a notification clipped by the plugin's line-boundary budget still shows the summary.
3. When a navigation command succeeds, the Kiro plugin shall produce exactly one notification and return the complete text as the command's return message; only `nav.context` with the insert flag shall additionally write to the editor.
4. The Kiro plugin shall set the notification level to warning when a navigation output reports a missing document, a file not covered or a spec whose design is missing, and to info otherwise.
5. The Kiro plugin shall show task completion state as `[x]` for completed and `[ ]` for open tasks in every navigation output.
6. When a navigation command runs, the Kiro plugin shall read only the roadmap and the documents of the specs it checks, and shall never read the content of the active code file.

### Requirement 8: Robustness and Verification
**Objective:** As a maintainer, I want navigation to be safe inside the sandbox and verified natively, so that a malformed spec never disables the plugin and regressions are caught without an IDE session.

#### Acceptance Criteria
1. The Kiro plugin shall return an error message rather than panicking or trapping for every navigation input, including missing or malformed documents, files with unusual names, unknown task ids and malformed arguments.
2. When `nav.context` runs in a project with up to 64 known specs whose documents total up to 1 MiB, the Kiro plugin shall complete within the host's per-call budget of 5 seconds.
3. The Kiro plugin shall provide native unit tests for comment-syntax selection by extension and by extension-less name including case-insensitivity and the bare fallback, reference-line composition, outline extraction including the merge of task items, and the text of the ownership, by-component and by-task views including every degradation case of Requirements 2, 4 and 5.
4. The Kiro plugin shall provide native end-to-end tests that run every navigation command and the insert variant against a simulated host serving an in-memory project with at least two specs, verifying notifications, insertions, return messages, error messages, and that no read of the active code file occurs.
5. The Kiro plugin shall pass the plugin's manifest consistency test with the navigation commands declared.
6. The Kiro plugin shall build for the host's sandbox target and pass the project's lint and format checks with the navigation commands included.
7. When the built component is loaded by the host as a dev plugin in this repository and `nav.context` is run with `crates/kiro-core/src/md.rs` open, the Kiro plugin shall show a notification naming `kiro-core` and `MarkdownScanner`.
