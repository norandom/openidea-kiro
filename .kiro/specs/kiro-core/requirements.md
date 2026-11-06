# Requirements Document

## Introduction

kiro-core is the shared engine for version 2 of spec constraint checking on the cc-sdd / kiro layout. It turns the text of a `.kiro` tree — `spec.json`, `requirements.md`, `design.md`, `tasks.md` per feature and `.kiro/steering/roadmap.md` — into a plain, inspectable model: features with phases, approvals and topics; requirements with acceptance criteria identified as `N.M`; a task tree with checkbox state, parallel markers, requirement, dependency and boundary annotations; design components mapped to files; Mermaid blocks; and the roadmap's feature list. On top of the model it provides current-spec resolution (which feature is "current" given an explicit name, the active editor file and the known feature list), the file ↔ component ↔ task join that navigation and visualization are built on, a single finding shape, and Mermaid/text renderers with size caps.

The engine takes all content as strings through a caller-supplied read abstraction and an explicit list of known feature names. It performs no file system, network, environment, clock or thread operations, so the same engine runs unchanged inside the OpenIDEA plugin sandbox and in the native CLI gate. Parsers accept the grammars seen in real cc-sdd specs, return partial data plus findings on malformed input, and never panic.

## Boundary Context

- **In scope**: the read abstraction and workspace; parsers for the five document types; the spec model with line spans; current-spec resolution including the superseded → successor rule; the file ↔ component ↔ task join; the finding type and its stable codes; generic Mermaid and text renderers with caps; robustness and determinism guarantees; documentation of the accepted grammar.
- **Out of scope**: vocabulary loading and term resolution (vocabulary-grounding); EARS shape, declared-before-use and contradiction checks (spec-constraints); any concrete I/O implementation such as reading from the IDE host or the file system (plugin-core, cli-gate); command and presentation design such as notification clipping or editor insertion (plugin-core and feature specs); the choice of which nodes and edges a visualization shows (spec-visualization); writing or editing any file.
- **Adjacent expectations**: plugin-core and cli-gate supply the read abstraction and the known feature names; vocabulary-grounding, spec-navigation, spec-visualization and spec-constraints consume the model, the join and the renderers without re-parsing documents; spec-constraints may attach additional annotation conventions to requirements and criteria, which this engine captures generically without interpreting them.

## Requirements

### Requirement 1: Read Abstraction and Workspace
**Objective:** As a frontend author (plugin or CLI), I want the engine to obtain all content through a string-read abstraction and an explicit list of known feature names, so that the same engine runs in the sandbox and natively without drift.

#### Acceptance Criteria
1. The kiro-core engine shall obtain every document it processes through a caller-supplied read operation keyed by project-relative path, and shall perform no file system, network, environment, clock or thread operation of its own.
2. When a workspace is created, the kiro-core engine shall accept an explicit list of known feature names from the caller and shall not attempt to discover features by listing directories.
3. When a feature is loaded, the kiro-core engine shall request that feature's `spec.json`, `requirements.md`, `design.md` and `tasks.md` by project-relative path under `.kiro/specs/<feature>/` and shall request each path at most once per workspace.
4. If a requested document is unavailable, the kiro-core engine shall record a finding with code `SPEC_DOC_MISSING` naming the path, with severity `warning` when the feature's phase implies the document should exist and severity `info` otherwise, and shall continue with the documents that are available.
5. The kiro-core engine shall expose each loaded feature as plain data — metadata, requirements, design, tasks and the findings produced while loading — that the caller can inspect without further reads.
6. When a feature name outside the known list is loaded explicitly, the kiro-core engine shall load it and add it to the workspace's known feature names.

### Requirement 2: Spec Metadata
**Objective:** As a consumer of the model, I want `spec.json` phase, approvals, topics and notes available as structured data, so that status, resolution and topic-scoped checks can rely on one reading of the file.

#### Acceptance Criteria
1. When `spec.json` is well-formed, the kiro-core engine shall expose the feature name, language, phase, the generated and approved flags for requirements, design and tasks, the ready-for-implementation flag, the declared topics (empty when absent), the notes text (empty when absent) and the created and updated timestamps as unmodified strings.
2. The kiro-core engine shall recognise the phases `initialized`, `requirements-generated`, `design-generated`, `tasks-generated`, `superseded` and `reserved`, compared case-insensitively after trimming.
3. If the phase value is present but not one of the recognised phases, the kiro-core engine shall record a finding with code `SPEC_PHASE_UNKNOWN` and severity `warning`, and shall keep the raw phase string available.
4. If the phase field is absent or empty, the kiro-core engine shall record a finding with code `SPEC_PHASE_MISSING` and severity `warning`.
5. If `spec.json` is not valid JSON or is not a JSON object, the kiro-core engine shall record a finding with code `SPEC_JSON_INVALID` and severity `error` naming the file, and shall still load the feature's other documents.
6. If an approval flag, the topics list or the notes text is absent, the kiro-core engine shall treat the flag as false, the list as empty or the text as empty without recording a finding.
7. The kiro-core engine shall ignore fields in `spec.json` that it does not recognise without recording a finding.

### Requirement 3: Common Markdown Handling
**Objective:** As a consumer of the model, I want every Markdown document to yield a heading outline, fenced blocks and 1-based line positions under one set of rules, so that navigation, findings and parsers agree on where things are.

#### Acceptance Criteria
1. The kiro-core engine shall report every position in a document as a 1-based line number, counting lines identically for LF and CRLF line endings.
2. If a document starts with a byte-order mark, the kiro-core engine shall ignore the mark when recognising the first line's markers.
3. When a Markdown document is parsed, the kiro-core engine shall expose an outline of all ATX headings with their level, text and line, where a `#` run is a heading only when followed by a space or the end of the line.
4. While inside a fenced code block (opened by three or more backticks or tildes and closed by a fence of the same character and at least the same length), the kiro-core engine shall treat no line as a heading, list item, table row or annotation.
5. When a fenced code block is encountered, the kiro-core engine shall record it with its info string, its first and last line and its body text.
6. If a fenced code block is not closed before the end of the document, the kiro-core engine shall treat the block as ending at the last line and record a finding with code `MD_FENCE_UNCLOSED` and severity `info`.

### Requirement 4: Requirements Document
**Objective:** As a reviewer or check author, I want every acceptance criterion in `requirements.md` identified as `N.M` with its text and line span, under all requirement grammars seen in real cc-sdd specs, so that findings and traceability point at the right line.

#### Acceptance Criteria
1. When a heading of the form `### Requirement N: title` (separator colon, hyphen, en dash or em dash; `Requirement` case-insensitive) is encountered, the kiro-core engine shall record requirement `N` with its title and heading line.
2. When a numbered item `M.` or `M)` appears under an `Acceptance Criteria` heading inside a requirement block, the kiro-core engine shall record criterion `N.M` with the text after the marker, its start line and its end line.
3. When a list item whose marker is the dotted identifier `N.M` of the enclosing requirement appears anywhere inside that requirement block, with or without a leading bullet and with or without a leading `R`, the kiro-core engine shall record it as criterion `N.M`.
4. When a heading of the form `### R<N> — title` (requiring a dash or colon separator after the number) is encountered, the kiro-core engine shall record requirement `N` so that `- R<N>.<M>` bullets become criteria `N.M`.
5. When a heading of the form `### Amendment A<K> — title (extends Requirement X / Requirement Y)` is encountered, the kiro-core engine shall record an amendment `A<K>` with the extended requirement numbers, and shall record its numbered items as criteria `A<K>.M` without requiring an `Acceptance Criteria` heading.
6. While a criterion is open, the kiro-core engine shall append non-blank, non-marker lines to its text joined by single spaces and extend its end line, and shall close the criterion at a blank line, the next marker or the next heading.
7. When a `**Objective:**` line appears inside a requirement block, the kiro-core engine shall record its text as the requirement's objective.
8. When a line of the form `_Key: value_` appears inside a requirement block or indented under a criterion, the kiro-core engine shall record it as an annotation with its key, raw value and line on that requirement or criterion, without interpreting the key.
9. If a requirement block ends without any criterion, the kiro-core engine shall record a finding with code `REQ_NO_CRITERIA` and severity `warning` at the heading line.
10. If a requirement number or criterion identifier is defined more than once in a document, the kiro-core engine shall keep the first definition and record a finding with code `REQ_DUPLICATE_ID` and severity `warning` at the later line.
11. If a `### Requirement` heading has no numeric identifier, the kiro-core engine shall record a finding with code `REQ_NON_NUMERIC_ID` and severity `warning` and shall not record criteria for that block.
12. When a requirements document contains no requirement headings, the kiro-core engine shall yield an empty requirement list without recording a finding.

### Requirement 5: Tasks Document
**Objective:** As a consumer of the model, I want `tasks.md` as a two-level task tree with checkbox state, parallel markers and parsed annotations, so that status, navigation and visualization can reason about tasks without re-reading the text.

#### Acceptance Criteria
1. When a list item of the form `- [ ] N. text` or `- [ ] N.M text` is encountered, the kiro-core engine shall record a task with its identifier, description, line and checked state, where `[x]` and `[X]` mean checked and `[ ]` means unchecked.
2. When the checkbox is immediately followed by `*` (as in `- [ ]* 2.3 text`), the kiro-core engine shall mark the task as optional.
3. When `(P)` immediately follows the task identifier, the kiro-core engine shall mark the task as parallel-capable.
4. The kiro-core engine shall nest a task `N.M` under major task `N` and shall expose major tasks in document order with their sub-tasks in document order.
5. If a task identifier has more than two numeric components or a sub-task appears with no preceding major task of the same number, the kiro-core engine shall record a finding with code `TASK_HIERARCHY` and severity `warning` and shall keep the task at the top level.
6. When indented lines follow a task item before the next task item or heading, the kiro-core engine shall attach them to that task as detail lines.
7. When a detail line of the form `_Requirements: a, b_` is encountered, the kiro-core engine shall record the comma-separated entries, trimmed, as the task's requirement references.
8. When a detail line of the form `_Depends: a, b_` is encountered, the kiro-core engine shall record the comma-separated entries, trimmed, as the task's dependency references.
9. When a detail line of the form `_Boundary: a, b_` is encountered, the kiro-core engine shall record the comma-separated entries, trimmed, as the task's boundary component names.
10. When a detail line of the form `_Key: value_` with any other key is encountered, the kiro-core engine shall record it as a generic annotation on the task.
11. If a requirement reference is not of the form `N`, `N.M` or `A<K>.M`, the kiro-core engine shall record a finding with code `TASK_REQ_REF_INVALID` and severity `warning` at the annotation line and shall keep the raw entry.
12. If a task identifier is defined more than once, the kiro-core engine shall keep the first definition and record a finding with code `TASK_DUPLICATE_ID` and severity `warning` at the later line.
13. The kiro-core engine shall expose the total and checked task counts for the document and per major task.

### Requirement 6: Design Document
**Objective:** As a navigation or visualization consumer, I want `design.md` to yield its components, the files each component owns via the File Structure Plan, and its Mermaid blocks, so that file ownership can be computed without reading prose.

#### Acceptance Criteria
1. When a heading whose text is `File Structure Plan` (case-insensitive, any level) is encountered, the kiro-core engine shall treat the content up to the next heading of the same or higher level as the file structure plan.
2. When a Markdown table inside the file structure plan has header cells matching `Component` and `File`, `Files` or `Path` (case-insensitive), the kiro-core engine shall record each row as component → file mappings, splitting multiple backticked paths in one cell into separate files.
3. When a list item inside the file structure plan has the form `- **Component**: ` followed by one or more backticked paths, the kiro-core engine shall record each path as owned by that component.
4. When a fenced code block inside the file structure plan contains a directory tree (lines with tree-drawing characters or indentation followed by a name), the kiro-core engine shall reconstruct each file's full path from its ancestors, record a trailing `#` comment as its responsibility, and skip entries ending in `/` as directories.
5. When a `####` heading appears under a heading whose text is `Components and Interfaces` (case-insensitive), or a table in that section has a `Component` column, the kiro-core engine shall record the component name.
6. If a recorded component has no file mapped to it, the kiro-core engine shall record a finding with code `DESIGN_COMPONENT_NO_FILES` and severity `info` at the line where the component was recorded.
7. If a file appears in the file structure plan without a component mapping, the kiro-core engine shall expose it as an unassigned design file and record a finding with code `DESIGN_FILE_UNASSIGNED` and severity `info`.
8. If a design document has no file structure plan section, the kiro-core engine shall record a finding with code `DESIGN_NO_FILE_PLAN` and severity `warning`.
9. When a fenced block with info string `mermaid` is encountered anywhere in the design document, the kiro-core engine shall expose it as a Mermaid block with its line span and body.
10. The kiro-core engine shall normalise every design file path by converting backslashes to slashes, removing a leading `./` and collapsing repeated slashes before storing it.

### Requirement 7: Roadmap Document
**Objective:** As the plugin, I want the roadmap's `## Specs (dependency order)` list as data, so that known feature names and their dependencies are available without listing directories.

#### Acceptance Criteria
1. When a heading whose text starts with `Specs` and contains `dependency order` (case-insensitive) is encountered, the kiro-core engine shall treat the list items up to the next heading of the same or higher level as roadmap entries.
2. When a roadmap list item has the form `- [ ] name -- description. Dependencies: a, b` (separator `--` or em dash), the kiro-core engine shall record the feature name, the checked state, the description, the dependency names and the line.
3. When the dependencies part is absent or reads `none` (case-insensitive), the kiro-core engine shall record an empty dependency list.
4. If a dependency names a feature that is not in the roadmap list, the kiro-core engine shall record a finding with code `ROADMAP_DEP_UNKNOWN` and severity `warning` at that line.
5. If the roadmap has no specs section, the kiro-core engine shall yield an empty list and record a finding with code `ROADMAP_NO_SPECS_SECTION` and severity `info`.
6. The kiro-core engine shall expose the roadmap feature names in document order for use as the workspace's known feature names.

### Requirement 8: Current-Spec Resolution
**Objective:** As a plugin command, I want to know which feature is current given an explicit argument, the active editor file and the known features, so that every command works from an open code file without directory listing.

#### Acceptance Criteria
1. When an explicit non-empty feature name is supplied, the kiro-core engine shall resolve to that feature regardless of the active file.
2. When no explicit name is supplied and the active file, after normalisation, lies under `.kiro/specs/<feature>/`, the kiro-core engine shall resolve to `<feature>`.
3. When no explicit name is supplied, the active file does not identify a feature and exactly one feature is known, the kiro-core engine shall resolve to that feature.
4. When no explicit name is supplied, the active file does not identify a feature and the known features number zero or more than one, the kiro-core engine shall return an unresolved result listing the candidate features.
5. When an active file path is absolute, the kiro-core engine shall normalise it to a project-relative path by removing the workspace root prefix, comparing the prefix case-insensitively on Windows-style paths and treating backslashes and slashes as equal.
6. If an active file path is absolute and not under the workspace root, the kiro-core engine shall report it as outside the project rather than guessing a relative path.
7. When a feature's phase is `superseded`, the kiro-core engine shall resolve its successors as the known feature names that appear as whole words in its notes text, preferring longer names when one name is a prefix of another.
8. If a superseded feature names no known successor, the kiro-core engine shall record a finding with code `SPEC_SUPERSEDED_DANGLING` and severity `warning`.
9. When successor resolution is followed through a chain of superseded features, the kiro-core engine shall stop at the first feature that is not superseded or at a repeated feature, and shall record a finding with code `SPEC_SUPERSEDED_CYCLE` and severity `warning` in the repeated case.

### Requirement 9: File, Component and Task Join
**Objective:** As a navigation or visualization consumer, I want to ask which components, tasks and criteria own a code file, and which files and tasks a feature covers, so that the same join serves both directions and both frontends.

#### Acceptance Criteria
1. When asked for the owners of a project-relative file path within a feature, the kiro-core engine shall return the components whose design files match, the tasks whose boundary names those components, and the criteria referenced by those tasks' requirement references.
2. The kiro-core engine shall consider a design file to match a queried path when their normalised forms are equal, or when one is a suffix of the other starting at a path-segment boundary.
3. When more than one component matches a queried path, the kiro-core engine shall return all matching components.
4. When no component matches a queried path, the kiro-core engine shall return an empty owner set marked as not covered by the feature's design, without recording a finding.
5. The kiro-core engine shall compare boundary names and component names after trimming and case-insensitively.
6. If a task's boundary names a component that the feature's design does not define, the kiro-core engine shall record a finding with code `TASK_BOUNDARY_UNKNOWN` and severity `warning` at the task line.
7. If a task's requirement reference names a criterion that the feature's requirements do not define, the kiro-core engine shall record a finding with code `TASK_REQ_UNKNOWN` and severity `warning` at the annotation line.
8. If a task's dependency reference names a task that the document does not define, the kiro-core engine shall record a finding with code `TASK_DEPENDS_UNKNOWN` and severity `warning` at the annotation line.
9. When asked for a feature's coverage, the kiro-core engine shall return every component with its files and the tasks bound to it, plus the tasks that declare no boundary listed as unscoped.
10. When asked for a feature's coverage, the kiro-core engine shall also return the tasks whose `_Boundary:_` contains one or more names matching no component of the feature's design, each paired with those unmatched boundary names as written; when the design document is absent, every task with a non-empty boundary shall be returned with all of its boundary names (amendment 2026-11-04).

### Requirement 10: Findings
**Objective:** As a frontend author, I want one finding shape with stable codes and deterministic order, so that the IDE and CI report identical results.

#### Acceptance Criteria
1. The kiro-core engine shall represent every finding with a severity of `error`, `warning` or `info`, a code in SCREAMING_SNAKE form, an optional feature name, a project-relative file path, an optional 1-based line and a human-readable message.
2. The kiro-core engine shall order findings by file path, then line (absent lines first), then code, then message, so that identical input yields identical order.
3. The kiro-core engine shall expose the complete list of finding codes it can produce, each with its default severity and a one-line description.
4. When a workspace's findings are requested, the kiro-core engine shall return the findings of all loaded features and the roadmap, merged and ordered as in criterion 10.2.

### Requirement 11: Rendering
**Objective:** As the visualization and CLI specs, I want Mermaid and text output produced by one engine with safe identifiers and size caps, so that output is valid and fits the host's limits.

#### Acceptance Criteria
1. When a graph of nodes (identifier, label, optional class) and edges (from, to, optional label) is rendered as Mermaid, the kiro-core engine shall emit node identifiers containing only letters, digits and underscores, replacing other characters and keeping distinct input identifiers distinct.
2. When a label is rendered in Mermaid, the kiro-core engine shall quote it and escape double quotes, `<` and `>` so that the output contains no raw `<`, `>` or unescaped `"` inside labels.
3. When a graph exceeds the caller's maximum node or edge count, the kiro-core engine shall render the first nodes and edges within the cap in input order, add one ellipsis node stating how many were omitted, and report that the output was truncated.
4. When a graph is rendered as text, the kiro-core engine shall emit one line per node followed by its outgoing edges, indented, applying the same caps as the Mermaid form.
5. When findings are rendered as text, the kiro-core engine shall emit one line per finding in the form `path:line: SEVERITY CODE message` (omitting `:line` when absent) in the order defined by 10.2.
6. When rows are rendered as a text table, the kiro-core engine shall emit a Markdown table with the given headers and escape `|` inside cells.
7. When an output byte limit is supplied, the kiro-core engine shall truncate the rendered text at a line boundary below the limit and append a final line stating how many lines were omitted.
8. The kiro-core engine shall produce byte-identical output for identical input across runs and targets.

### Requirement 12: Robustness, Portability and Documentation
**Objective:** As the plugin host and the CI operator, I want the engine to be total, fast and portable, so that a malformed spec cannot disable the plugin or hang the gate.

#### Acceptance Criteria
1. The kiro-core engine shall return a result for every input string, including empty, binary-looking and deeply nested text, without panicking or aborting.
2. When input is malformed, the kiro-core engine shall return the structure it could recognise together with findings describing what it could not, rather than discarding the document.
3. The kiro-core engine shall process each document in a single forward pass with per-line work bounded by the line length, so that a 16 MiB document parses within 2 seconds on the native target in a release build.
4. The kiro-core engine shall build and pass its tests from one source tree for both the native target and the plugin sandbox target.
5. The kiro-core engine shall produce output that depends only on its inputs, with no timestamps, randomness or environment-dependent ordering.
6. The kiro-core engine shall document, in its public API documentation, the exact grammars accepted for each document type and the full list of finding codes.
7. The repository's CI shall run the native test suites of the engine and the CLI in parallel with `cargo nextest run` (process-per-test) and the doctests with `cargo test --doc`, per the Testing standard in `steering/tech.md`, so that a full gate run is wall-clock bounded; locally `cargo test` remains valid.
