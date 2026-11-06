# Requirements Document

## Introduction

spec-visualization adds the `viz.*` command group to the Kiro plugin for OpenIDEA. Kiro specs are text spread over four documents; their structure — phase and approvals, requirements → criteria → tasks → components → files, and the order and explicit dependencies between tasks — is invisible without reading everything. The plugin sandbox has no UI panels, so this spec renders that structure as text: Markdown tables for the status view and Mermaid diagrams (with complete table fallbacks) for the two graph views, delivered through the plugin's notification, the command's return message and, on explicit request, insertion at the editor cursor.

Three commands are provided for the current spec (or one named explicitly): `viz.status` reports phase, approvals, task completion and the superseded → successor chain; `viz.graph` draws requirements → criteria → tasks → components → files; `viz.tasks` draws the task dependency graph from `_Depends:_` annotations and task ordering, marking parallel-capable and completed tasks. This spec owns the *shapes* of those views (which nodes, edges, labels and kind markers appear, in which order, and how they degrade) and their text fallbacks. Mermaid syntax safety, identifier sanitising and size caps are owned by the engine (kiro-core) and are consumed, not re-implemented. Spec selection, argument interpretation, per-spec command variants, notification clipping and insertion are owned by plugin-core and are consumed through its feature-module contract.

A verified host fact shapes the output: the IDE's UI never passes an argument to a plugin command and discards the command's return message, showing only errors. The notification is therefore the primary channel, which is why every view begins with summary lines that survive clipping, and why the `text` option is documented as reachable only by argument (programmatic callers and tests).

## Boundary Context

- **In scope**: the three commands `viz.status`, `viz.graph`, `viz.tasks`, each with an insert variant and per-spec variants; the content, ordering and kind markers of the status view and the two graphs; label rules including truncation and the optional vocabulary preferred-label hook; the summary lines, titles and notification levels; the table form (`text` option) as the complete fallback for both graphs; truncation behaviour when a graph exceeds the engine's caps; error behaviour for missing documents and unknown options; golden-output, cap, Mermaid-validity and end-to-end tests; registration of the commands in the plugin's manifest and dispatch.
- **Out of scope**: Mermaid emission, identifier sanitising, label escaping and node/edge caps (kiro-core `render`); spec resolution, argument grammar, variant registration, notification clipping and editor insertion mechanics (plugin-core); the file ↔ component ↔ task join and the successor rule (kiro-core); loading the vocabulary and resolving terms (vocabulary-grounding); navigation commands (spec-navigation); term checks (spec-constraints); writing diagrams or anything else into project files; any change to the OpenIDEA host.
- **Adjacent expectations**: plugin-core resolves the spec (explicit name, active file, only known spec), parses `insert` and the spec token, registers `viz.<name>.<spec>` variants within the shared 256-command budget, clips the notification below 16 KiB and the insertion below 1 MiB at line boundaries, and returns the complete text as the command message; kiro-core supplies the spec model with line spans, task counts, the join, the successor chain, and renderers that produce valid Mermaid, apply caps with an ellipsis node and render Markdown tables; vocabulary-grounding may supply a topic-scoped preferred-label lookup that this spec uses when present and ignores when absent; cli-gate may reuse the same views natively, so the view content must not depend on anything only the IDE provides.

## Requirements

### Requirement 1: Visualization Commands and Spec Selection
**Objective:** As a developer using OpenIDEA, I want three visualization commands that act on the spec I am working in, so that I can see a spec's structure without leaving the editor or typing a spec name the IDE cannot ask for.

#### Acceptance Criteria
1. The Kiro plugin shall provide the commands `viz.status`, `viz.graph` and `viz.tasks`, each spec-addressable (so that a per-spec variant `viz.<name>.<spec>` exists for every known spec) and each insert-capable (so that a project-independent variant `viz.<name>.insert` exists), declared through the plugin's extension contract so that six project-independent commands and three per-spec variants per known spec count against the shared command limit.
2. When a `viz.*` command runs, the Kiro plugin shall act on the spec selected by the plugin's resolution order — the explicitly named spec, else the spec whose directory contains the active file, else the only known spec — and shall return the plugin's unresolved-spec error when none applies.
3. When a `viz.*` command runs with an argument, the Kiro plugin shall take the first token (after the `insert` flag) as the spec name and every remaining token as an option of the command.
4. When `viz.graph` or `viz.tasks` receives the option `text`, the Kiro plugin shall produce the table form of the view instead of the diagram form.
5. If a `viz.*` command receives an option it does not accept, the Kiro plugin shall return an error naming the option and listing the accepted options (`text` for `viz.graph` and `viz.tasks`; none for `viz.status`) and shall produce no notification.
6. The Kiro plugin shall never create, modify or delete a project file while running a `viz.*` command; insertion into the editor happens only when the insert flag is set.

### Requirement 2: Output Channels, Insertion and Levels
**Objective:** As a developer, I want each view readable in the notification, pasteable into a Markdown document and complete in the command message, so that the same result serves quick inspection, documentation and automation.

#### Acceptance Criteria
1. When a `viz.*` command succeeds, the Kiro plugin shall produce one result text that is shown as one notification (clipped by the plugin at a line boundary) and returned complete as the command's return message.
2. The Kiro plugin shall begin every result text with the view's summary lines, followed by one blank line and the view body, so that a clipped notification still shows the summary.
3. When the diagram form is produced, the Kiro plugin shall emit the body as one fenced Mermaid block (a ```` ```mermaid ```` opening fence, the diagram, and a closing fence) so that inserting the result into a Markdown document renders as a diagram.
4. When the insert flag is set, the Kiro plugin shall insert the complete result text (clipped by the plugin at a line boundary below its insertion limit) at the cursor of the active editor and shall still show the notification.
5. When a `viz.*` command succeeds, the Kiro plugin shall title the notification with the spec name and the view's key figure: the phase for `viz.status`, the number of drawn nodes for `viz.graph`, and done/total tasks for `viz.tasks`.
6. When a `viz.graph` or `viz.tasks` result reports a missing document, unresolved references or truncation, the Kiro plugin shall show the notification at warning level; otherwise at info level.
7. When a `viz.status` result is shown, the Kiro plugin shall set the notification level from the spec's findings as the plugin does for its own spec report (error when any finding is an error, warning when any is a warning, info otherwise).

### Requirement 3: Status View (`viz.status`)
**Objective:** As a developer, I want a one-screen status of a spec — where it is in the workflow, how much of it is done and where it leads if superseded — so that I can judge its state before opening its documents.

#### Acceptance Criteria
1. When `viz.status` runs, the Kiro plugin shall report the spec name, the phase as written in `spec.json` (or `missing` when `spec.json` is unreadable or has no phase), whether the spec is marked ready for implementation, and a table with one row per document kind (requirements, design, tasks) and columns for generated and approved.
2. When `viz.status` runs and `tasks.md` is readable, the Kiro plugin shall report completed/total tasks with an integer percentage, the number of parallel-capable tasks and the number of optional tasks, followed by a table with one row per major task giving its number, its completed/total count and its description.
3. When `viz.status` runs, the Kiro plugin shall report the number of requirements and criteria, the number of design components and files and how many files are not assigned to a component, and shall print `missing` in place of the figures of any unreadable document.
4. While the spec's phase is `superseded`, the Kiro plugin shall report the successor chain resolved against the known specs as the spec names joined by `->`, ending with `(terminal)` when the chain ends at a non-superseded spec, `(cycle)` when it revisits a spec, or `(no known successor)` when the spec names no known successor.
5. When `viz.status` runs, the Kiro plugin shall report the number of the spec's findings per severity (errors, warnings, info).
6. If `spec.json` is unreadable, the Kiro plugin shall still produce the status view with phase `missing`, an approvals table of `no` values, and the figures of whichever documents are readable.

### Requirement 4: Spec Graph (`viz.graph`)
**Objective:** As a developer reading a design, I want the requirements → criteria → tasks → components → files chain as a diagram, so that I can see at a glance which criteria are implemented by which tasks, components and files.

#### Acceptance Criteria
1. When `viz.graph` runs, the Kiro plugin shall draw one node per requirement, per criterion, per leaf task (a sub-task, or a major task without sub-tasks), per design component, and per design file assigned to a component, each carrying a kind marker from the set `req`, `crit`, `task`, `task_done`, `comp`, `file`.
2. When `viz.graph` runs, the Kiro plugin shall draw an edge from each requirement to each of its criteria; from a criterion to each leaf task whose `_Requirements:_` names that criterion; from a requirement to each leaf task whose `_Requirements:_` names the whole requirement; from a leaf task to each design component its `_Boundary:_` names (matched case-insensitively); and from a component to each file the File Structure Plan assigns to it.
3. The Kiro plugin shall label requirement nodes as `R<number>: <title>` (amendments as `<id>: <title>`), criterion nodes as `<id>: <text>`, task nodes as `[x]` or `[ ]` (with `*` appended for optional tasks) followed by the task id, `(P)` for parallel-capable tasks and the description, component nodes with the component's display name (Requirement 6), and file nodes with the project-relative path; titles, criterion texts and descriptions are cut at 48 characters with a trailing ellipsis, paths are never cut.
4. The Kiro plugin shall order nodes layer by layer — requirements, criteria, tasks, components, files — in document order within each layer, and edges in the same layer order, so that when the diagram is truncated the upper layers are kept first.
5. If a task's `_Requirements:_` or `_Boundary:_` entry names no requirement, criterion or component of the spec, the Kiro plugin shall draw no edge for it and shall count it as an unresolved reference in the summary.
6. When `viz.graph` runs, the Kiro plugin shall report in the summary the node count per layer and the edge count, the number of leaf tasks without a boundary, the number of files not assigned to a component, the number of unresolved references, and the name of each of the three documents that was unreadable.
7. If none of `requirements.md`, `design.md` and `tasks.md` is readable for the spec, the Kiro plugin shall return an error naming the spec; otherwise it shall draw the graph from the readable documents.

### Requirement 5: Task Dependency Graph (`viz.tasks`)
**Objective:** As a developer planning or reviewing implementation, I want the task graph with explicit dependencies, implicit ordering, parallel markers and completion, so that I can see what can run next and what is blocked.

#### Acceptance Criteria
1. When `viz.tasks` runs, the Kiro plugin shall draw one node per task (major tasks and sub-tasks) in document order, labelled as in 4.3 for tasks, with the kind marker `major` or `major_done` for major tasks and `task` or `task_done` for sub-tasks according to completion.
2. When `viz.tasks` runs, the Kiro plugin shall draw a containment edge from each major task to each of its sub-tasks.
3. When `viz.tasks` runs, the Kiro plugin shall draw, for each `_Depends:_` entry that names a task of the spec, an edge labelled `depends` from the named task to the depending task.
4. When `viz.tasks` runs, the Kiro plugin shall draw an edge labelled `order` from each sub-task to the next sub-task of the same major task unless the next one is parallel-capable, and from each major task to the next major task unless the next one is parallel-capable.
5. If a `_Depends:_` entry names no task of the spec, the Kiro plugin shall draw no edge for it and shall count it as an unknown dependency in the summary.
6. When `viz.tasks` runs, the Kiro plugin shall report in the summary the total and completed task counts, the number of major tasks, the number of parallel-capable and optional tasks, the number of explicit dependency edges and unknown dependencies, and the number of ordering edges.
7. If `tasks.md` is unreadable for the spec, the Kiro plugin shall return an error naming the spec and the document path.
8. When the diagram form is produced for `viz.tasks`, the Kiro plugin shall orient it top-down so that prerequisites appear above the tasks that depend on them; `viz.graph` is oriented left-to-right so that the layers read from requirements to files.

### Requirement 6: Labels and Vocabulary
**Objective:** As a team with an agreed vocabulary, I want component names shown with their preferred labels when a vocabulary is available, so that diagrams use the agreed words — and plain names otherwise, so that visualization never depends on the vocabulary feature.

#### Acceptance Criteria
1. Where a topic-scoped preferred-label lookup is available to the plugin, the Kiro plugin shall label each component node with the preferred label found for the component name within the spec's declared topics, and with the raw component name when no preferred label is found.
2. While no preferred-label lookup is available (the vocabulary feature is not part of the build, the vocabulary file is absent, or the spec declares no topics), the Kiro plugin shall label component nodes with their raw names and shall produce no finding, warning or error about the absence.
3. The Kiro plugin shall never label a node with a preferred label that belongs to a topic the spec does not declare.
4. The Kiro plugin shall keep the raw component name as the node's identity, so that preferred labels change only the displayed text and never merge or split nodes.

### Requirement 7: Size Limits, Validity, Robustness and Determinism
**Objective:** As a developer with large specs, I want the views to stay valid and complete within the host's limits, so that a big spec never produces an unreadable or broken diagram or disables the plugin.

#### Acceptance Criteria
1. When a graph exceeds the engine's caps (200 nodes or 400 edges), the Kiro plugin shall produce the diagram with the first nodes and edges within the caps, the engine's ellipsis node stating how many were omitted, and a summary line stating that the diagram is truncated and that the `text` option yields the complete table.
2. When the table form is produced, the Kiro plugin shall list every node with its kind and the labels of the nodes it links to, with no node or edge cap, subject only to the plugin's delivery clipping (the return message stays complete).
3. The Kiro plugin shall produce diagrams that are valid Mermaid flowcharts: node identifiers of letters, digits and underscores only; every label quoted with no raw `<`, `>` or unescaped `"`; every edge referencing drawn nodes.
4. The Kiro plugin shall return an error message rather than panicking or trapping for every `viz.*` input, including empty, malformed and oversized documents and malformed annotations.
5. When a `viz.*` command runs over a spec whose four documents total up to 1 MiB and whose roadmap lists up to 64 specs, the Kiro plugin shall complete within the host's per-call budget of 5 seconds.
6. The Kiro plugin shall produce byte-identical `viz.*` output for identical documents across runs and across the native and sandbox targets.

### Requirement 8: Verification
**Objective:** As a maintainer, I want the view shapes pinned by golden outputs and the commands proven end to end, so that changes to the engine or the plugin cannot silently alter what users see.

#### Acceptance Criteria
1. The Kiro plugin shall ship golden-output tests that compare the status view, the spec graph and the task graph — in diagram and table form — byte for byte against expected files for the engine's minimal fixture tree.
2. The Kiro plugin shall ship a test over a synthetic spec exceeding the caps that verifies the ellipsis node, the truncation summary line and the complete table form.
3. The Kiro plugin shall ship a test that checks every produced diagram line against the Mermaid validity rules of 7.3.
4. The Kiro plugin shall ship end-to-end tests over the plugin's in-memory host that verify: the six project-independent `viz` commands are registered at activation; per-spec `viz` variants exist after a command has run; the insert flag produces exactly one insertion and one notification; the `text` option switches the form; an unknown option, an unresolved spec and a spec without the needed documents produce the documented errors.
5. When the `viz` commands are added, the plugin's manifest consistency test and the engine's existing tests shall continue to pass.
6. The Kiro plugin and the engine shall build, pass lints with warnings denied and pass formatting checks for both the native and the sandbox target with the `viz` additions.
