# Requirements Document

## Introduction

plugin-core delivers the OpenIDEA plugin package for kiro: a sandboxed WebAssembly plugin, installed as a dev plugin, that makes cc-sdd / kiro spec state reachable from inside the IDE. Developers who run kiro specs today must switch to a terminal agent to learn which spec is current, what phase it is in and what its documents contain; OpenIDEA offers a plugin API but has no kiro awareness.

This spec provides everything that is common to every kiro command in the IDE: the package and its manifest, building and installing it as a dev plugin, declaring and registering every `plugin.kiro.*` command, discovering which specs exist without listing directories, resolving which spec a command acts on, interpreting command arguments, reading project content through the host (so unsaved editor contents count), presenting results as notifications, optional editor insertion and the command's return message, and turning every failure into an error message instead of a crash that would disable the plugin for the session. It ships two commands of its own, `ws.specs` and `ws.current`, which exercise that path end to end. Navigation, visualization, vocabulary and constraint-check commands are sibling specs that plug their commands into this dispatch.

A verified host fact shapes this spec: the IDE's own UI never passes an argument to a plugin command (Search Everywhere, shortcuts and the Plugins page all invoke commands without one; only another feature calling the action programmatically can pass a string), and the IDE discards a command's return message, showing only errors. The plugin therefore treats the active editor file and per-spec command variants as its primary way to select a spec, and the notification as its primary output channel.

## Boundary Context

- **In scope**: the plugin package (manifest, build, dev-plugin installation and its documentation); command ids, titles, static declaration and runtime registration within the host's limits; known-spec discovery from the roadmap, the active file and command arguments; argument interpretation and current-spec resolution; per-spec command variants as the fallback for the missing argument channel; reading project documents through the host; presentation (notification clipping, editor insertion, return message, findings text, severity levels); error mapping and sandbox robustness; the `ws.specs` and `ws.current` commands; the contract through which sibling specs add commands; native and host-side verification.
- **Out of scope**: parsing, the spec model, resolution rules and rendering (kiro-core); the `nav.*`, `viz.*`, `vocab.*` and `check.*` behaviours (spec-navigation, spec-visualization, vocabulary-grounding, spec-constraints); any change to the OpenIDEA host, its plugin API or its UI; writing or editing spec, vocabulary or any other project file; the CLI (cli-gate).
- **Adjacent expectations**: kiro-core supplies the read abstraction, known-feature workspace, resolution, finding type and text renderers that this plugin adapts; sibling specs declare their commands through this spec's extension contract and receive the resolved project context and parsed arguments from it; cli-gate presents findings in the same text form so IDE and CI output can be compared line by line; the host loads dev plugins from the open project's `.openidea/plugins/` directory and shows command errors as `<title> failed: <message>`.

## Requirements

### Requirement 1: Plugin Package, Build and Installation
**Objective:** As a developer using OpenIDEA, I want the Kiro plugin to be a self-contained package I can build and install as a dev plugin, so that I can run it against my project without changing the IDE.

#### Acceptance Criteria
1. The Kiro plugin shall be delivered as one package directory containing a manifest that identifies the plugin as `kiro`, targets host API version `0.1`, declares exactly the permissions `project-read`, `editor-read` and `editor-write`, and references the built component file inside the package.
2. When the package is built with the host repository's plugin build script given the package's absolute path, or with the documented direct build command from the package directory, the Kiro plugin shall produce the component file at the location the manifest references.
3. When the package directory, or a directory junction pointing to it, is placed under the open project's dev-plugin directory (`<project>/.openidea/plugins/`) and the project is trusted, the Kiro plugin shall load with status Active and its declared commands shall appear in the IDE's action list without an IDE restart.
4. When the component has been rebuilt and the IDE's Reload Plugins action is run, the Kiro plugin shall run the rebuilt component without an IDE restart.
5. The Kiro plugin package shall include documentation of the build steps, the Windows dev-plugin installation steps (creating the dev-plugin directory and the junction), the reload step, and the alternative installation from disk.
6. If the host grants the Kiro plugin fewer permissions than the three it requires, the Kiro plugin shall fail activation with a message naming the missing permissions instead of failing later while running a command.
7. The Kiro plugin package shall build independently of the repository's root build workspace, so that building or testing the root workspace does not require the plugin's sandbox target.

### Requirement 2: Command Declaration and Registration
**Objective:** As a developer, I want every Kiro command visible and runnable from Search Everywhere and the keymap, so that spec state is one action away.

#### Acceptance Criteria
1. The Kiro plugin shall identify every command with an id of the form `<group>.<name>` or `<group>.<name>.<modifier>`, where `<group>` is one of `ws`, `nav`, `viz`, `vocab` and `check`, and the full id matches the host's command-id rule (`[a-z0-9][a-z0-9._-]*`, at most 64 characters).
2. The Kiro plugin shall give every command a title that starts with `Kiro:` and is at most 128 characters long, and the category `Kiro`, so that all commands group together in Search Everywhere and the keymap editor.
3. The Kiro plugin shall declare every project-independent command statically in the manifest and register the same commands with identical ids, titles and category when activated, so that the command list is identical before and after activation.
4. When activated, the Kiro plugin shall register only project-independent commands, because the host provides no project context at activation.
5. The Kiro plugin shall keep the total number of registered commands, including per-spec variants, at or below 256, and shall never request a registration that would exceed this limit or that uses an invalid id.
6. If a command id that the Kiro plugin did not register is run, the Kiro plugin shall return an error message naming the id.
7. The Kiro plugin shall provide the commands `ws.specs` (list known specs and refresh per-spec commands) and `ws.current` (report the current spec) as its own base commands.

### Requirement 3: Spec Discovery Without Directory Listing
**Objective:** As a developer, I want the plugin to know which specs exist even though the sandbox cannot list directories, so that commands work in any project that keeps a steering roadmap.

#### Acceptance Criteria
1. When any command runs, the Kiro plugin shall determine the known specs as the union of the names listed in the roadmap's dependency-order section (`.kiro/steering/roadmap.md`), the spec whose directory contains the active file (`.kiro/specs/<spec>/`), and the spec named explicitly by the command, without listing any directory.
2. If the roadmap is absent or has no dependency-order section, the Kiro plugin shall continue with the remaining sources and shall state the absence in the output of `ws.specs`.
3. When `ws.specs` runs, the Kiro plugin shall report every known spec with the source or sources it was discovered from, whether its `spec.json` was readable, and its phase.
4. When `ws.specs` runs and no spec is known, the Kiro plugin shall report that no spec was found and name the roadmap path it looked for.

### Requirement 4: Command Arguments and Current-Spec Resolution
**Objective:** As a developer, I want each command to act on the right spec whether I run it from Search Everywhere, a shortcut or another feature passing an argument, so that I never have to type a spec name the IDE cannot ask for.

#### Acceptance Criteria
1. When a command is run with an argument string, the Kiro plugin shall interpret it as whitespace-separated tokens: the token `insert` sets the insert flag; for a spec-addressable command the first remaining token names the spec; all other tokens are passed unchanged to the command's owning feature.
2. When a command is run without an argument, the Kiro plugin shall treat the insert flag as unset and the spec as not explicitly named.
3. When a per-spec variant `<group>.<name>.<spec>` is run, the Kiro plugin shall behave exactly as the base command run with `<spec>` as the explicitly named spec, and when a variant `<group>.<name>.insert` is run, exactly as the base command run with the insert flag set.
4. If a spec is named both by a variant id and by the argument and the two names differ, the Kiro plugin shall return an error naming both.
5. If an explicitly named spec contains a path separator or a `..` segment, or does not match `[a-z0-9][a-z0-9._-]*`, the Kiro plugin shall return an error and shall not read any file for it.
6. The Kiro plugin shall resolve the current spec in this order: the explicitly named spec; the spec whose directory contains the active file; the only known spec when exactly one spec is known.
7. When the active file is absolute and lies outside the project root, or no editor is focused, the Kiro plugin shall treat the active file as absent for resolution rather than failing.
8. If the current spec cannot be resolved, the Kiro plugin shall return an error that lists the candidate specs and tells the user to run the command's per-spec variant.
9. If no project is open in the invoking window, the Kiro plugin shall return the error `no project is open` for every command that needs project content.

### Requirement 5: Per-Spec Command Variants
**Objective:** As a developer, I want a per-spec version of each spec-addressable command, so that I can choose the spec from Search Everywhere when the active file does not identify it.

#### Acceptance Criteria
1. When a command has determined the known specs, the Kiro plugin shall register, for every spec-addressable base command and every known spec whose name is a valid id segment (`[a-z0-9][a-z0-9_-]*`, at most 64 characters, not equal to `insert`), a variant command `<group>.<name>.<spec>` titled `<base title> — <spec>`.
2. The Kiro plugin shall register variants before running the requested command, so that an unresolved-spec error is immediately followed by usable per-spec commands.
3. When a variant is registered again with identical id, title and category, the Kiro plugin shall produce no visible change.
4. If registering the variants for all known specs would exceed the command limit, the Kiro plugin shall register variants for specs in roadmap order until the limit is reached, skip the remaining specs, and name the skipped specs in the `ws.specs` output and in the IDE log.
5. If a known spec's name is not a valid id segment, the Kiro plugin shall skip its variants, keep the spec addressable by argument, and name it in the `ws.specs` output.
6. When a variant names a spec whose `spec.json` cannot be read in the current project, the Kiro plugin shall return an error stating that the spec was not found in this project.
7. The Kiro plugin shall declare the `insert` variants (`<group>.<name>.insert`) of insert-capable commands as project-independent commands.

### Requirement 6: Reading Project Content
**Objective:** As a developer, I want the plugin to see the files I am editing, including unsaved changes, and never touch my project files, so that results are current and the plugin is safe.

#### Acceptance Criteria
1. The Kiro plugin shall read every project document through the host's read operation using project-relative paths with `/` separators, so that unsaved editor contents take precedence over the file on disk.
2. If a read fails because the file is absent, is not a text file, exceeds the host's size limit or lies outside the project root, the Kiro plugin shall treat the document as absent and continue the command, leaving the resulting finding to the engine.
3. When a command runs, the Kiro plugin shall read each document path at most once.
4. The Kiro plugin shall read only the documents a command needs: the roadmap and the resolved spec's documents for single-spec commands, and the roadmap plus each known spec's documents for `ws.specs`.
5. The Kiro plugin shall never create, modify or delete any project file.
6. When a read fails, the Kiro plugin shall record the path and the host's reason in the IDE log at debug level.

### Requirement 7: Presentation of Results
**Objective:** As a developer, I want command results in a notification, optionally inserted into my editor, and consistent with the CLI, so that I can read them in the IDE and compare them with CI.

#### Acceptance Criteria
1. When a command succeeds, the Kiro plugin shall show one notification whose title is the command's result title (at most 256 characters) and whose body is the result text, and shall return the complete result text as the command's return message.
2. If the result text exceeds the notification budget, the Kiro plugin shall cut the notification body at a line boundary below 16 KiB and append a final line stating how many lines were omitted and, when the command has an insert variant, naming that variant (e.g. `… N more lines — run <id>.insert for the full text`), so that the host never truncates it mid-line and the developer can discover the full-output path (amendment 2026-11-04).
3. When the insert flag is set on an insert-capable command, the Kiro plugin shall insert the complete result text at the cursor of the active editor, cut at a line boundary below 1 MiB with an omitted-lines trailer when necessary, and shall still show the notification.
4. If the insert flag is set on a command that is not insert-capable, the Kiro plugin shall return an error stating that the command does not support insertion.
5. When a result includes findings, the Kiro plugin shall present each finding as `path:line: SEVERITY CODE message` in the engine's finding order, identical to the CLI's text output.
6. The Kiro plugin shall set the notification level to error when any presented finding has severity error, to warning when any has severity warning, and to info otherwise.
7. When a command fails, the Kiro plugin shall return a single-line error message of at most 1 KiB, which the host shows as `<title> failed: <message>`, and shall show no additional notification.
8. When `ws.current` runs, the Kiro plugin shall report the resolved spec name, how it was resolved, its phase, which of its four documents were readable, the number of findings per severity, and the findings text.

### Requirement 8: Robustness Within the Sandbox
**Objective:** As a developer, I want the plugin to keep running for the whole session no matter what my spec files contain, so that one malformed file never disables the plugin.

#### Acceptance Criteria
1. The Kiro plugin shall return an error message rather than panicking or trapping for every command input, including empty, malformed and oversized documents, unknown ids and malformed arguments.
2. The Kiro plugin shall use only the host's functions for logging, command registration, notifications, project reads, active-file lookup and editor insertion, and shall not use the file system, network, environment, threads or clocks directly.
3. When a command runs over a project whose roadmap lists up to 64 specs and whose resolved spec's documents total up to 1 MiB, the Kiro plugin shall complete within the host's per-call budget of 5 seconds.
4. When the plugin is deactivated, the Kiro plugin shall return without error and without any host call other than logging.
5. The Kiro plugin shall build its view of the project afresh on every command and retain between calls only the granted permissions and the set of registered command ids, so that each command observes the current project state.

### Requirement 9: Extension Contract for Feature Modules
**Objective:** As the author of a sibling spec (navigation, visualization, vocabulary, checks), I want to add my commands by declaring them once, so that dispatch, resolution and presentation need no changes.

#### Acceptance Criteria
1. The Kiro plugin shall route each command to the feature that owns its `<group>` prefix: `ws` to this spec's commands, `nav` to navigation, `viz` to visualization, `vocab` to vocabulary and `check` to checks.
2. The Kiro plugin shall let each feature declare, per command, its id, title, whether it is spec-addressable, whether it is insert-capable, and whether its first argument token names a spec or all tokens are passed through.
3. When a feature's command runs, the Kiro plugin shall hand the feature the resolved project context (project root, normalised active file, known specs, and spec documents loaded on request) together with the parsed arguments, and shall take back a result title and text for presentation.
4. If a `<group>` has no feature registered in the current build, the Kiro plugin shall return an error stating that the feature is not available.

### Requirement 10: Verification
**Objective:** As a maintainer, I want the plugin's glue verified natively and the component verified in the host, so that regressions are caught without a manual IDE session.

#### Acceptance Criteria
1. The Kiro plugin shall provide native unit tests for argument parsing, command-id grammar, variant generation under the command limit, spec-name validation, active-file normalisation, result clipping at the notification and insertion budgets, and severity-to-level mapping.
2. The Kiro plugin shall provide native end-to-end tests that run activation and the `ws.*` commands against a simulated host serving an in-memory project tree, verifying registered commands, notifications, insertions, return messages and error messages.
3. The Kiro plugin shall provide a test proving that the manifest's static command list equals the set of project-independent commands registered at activation.
4. The Kiro plugin shall build for the host's sandbox target with the release profile the host expects, and shall pass the project's lint and format checks.
5. When the built component is loaded by the host as a dev plugin and `ws.current` is run with a spec document open, the Kiro plugin shall show a notification naming that spec.
