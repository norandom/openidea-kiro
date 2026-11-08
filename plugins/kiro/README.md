# OpenIDEA Kiro Plugin (`openidea-plugin-kiro`)

The `kiro` plugin integrates Kiro-style Spec-Driven Development (SDD) into the OpenIDEA editor as a sandboxed WebAssembly component (`wasm32-wasip2`). It bridges the pure `kiro-core` engine to OpenIDEA's extension host.

---

## 1. Building the Component

The WebAssembly component binary is built at `dist/plugin.wasm`. You can build it using any of three routes:

### Route A: PowerShell Script (Recommended on Windows)
From the package root (`plugins/kiro/`):
```powershell
.\build.ps1
```
This script ensures the `wasm32-wasip2` rustup target is installed, compiles the release component with `opt-level = "s"`, strips symbols, and copies the resulting binary to `dist/plugin.wasm`.

### Route B: Host Repository Build Script
From the OpenIDEA host checkout (`../openidea`):
```bash
./plugins/build.sh <absolute-path-to-plugins/kiro>
```

### Route C: Plain Cargo
From the package root (`plugins/kiro/`):
```bash
cargo build --release --target wasm32-wasip2
mkdir -p dist
cp target/wasm32-wasip2/release/openidea_plugin_kiro.wasm dist/plugin.wasm
```

---

## 2. Windows Dev-Plugin Installation

To install and test this plugin as a live developer plugin in OpenIDEA on Windows:

1. Create the plugin folder inside your project's `.openidea` configuration directory:
   ```powershell
   New-Item -ItemType Directory -Force -Path .openidea\plugins
   ```
2. Create an NTFS directory junction pointing to the plugin package directory:
   ```powershell
   cmd /c mklink /J .openidea\plugins\kiro <absolute-path-to-openidea-kiro>\plugins\kiro
   ```
3. Open or switch to the OpenIDEA IDE window.
4. Ensure the project is trusted (Plugins do not run in untrusted mode).
5. Open **Settings › Plugins** or execute the action **Reload Plugins** from Search Everywhere (`Ctrl+Shift+A`).

---

## 3. Which Project to Open

Always open the project whose `.kiro` tree you wish to inspect and work on:
- **This repository (`openidea-kiro`)**: Open this repository in OpenIDEA to navigate and inspect its own specs (`.kiro/specs/kiro-core`, `plugin-core`, etc.).
- **Host repository (`openidea`)**: Open the OpenIDEA host repository when testing host-side integration or when working on the host editor itself.

---

## 4. Installation from Disk & Rebuilding

- **Rebuilding & Live Reload**: When you modify Rust code in `plugins/kiro/src/` or `crates/kiro-core/`:
  1. Rebuild the component using `.\build.ps1`.
  2. In OpenIDEA, run the action **Reload Plugins** (`Ctrl+Shift+A` › "Reload Plugins").
  3. The IDE unloads the old component and reloads `dist/plugin.wasm` without restarting the IDE.
- **Disk Installation**: When packaging for distribution, zip the package containing `plugin.toml` and `dist/plugin.wasm`, then choose **Install Plugin from Disk...** in OpenIDEA's Plugin Settings.

---

## 5. Command Reference

The static commands declared in `plugin.toml` match the registry's static list:

| Command ID | Action Title | Spec-Addressable | Insert-Capable | Description |
| :--- | :--- | :---: | :---: | :--- |
| `ws.specs` | `Kiro: List Specs` | No | Yes | Lists all known specs, discovery sources, `spec.json` readability, phases, and per-spec command variants. |
| `ws.specs.insert` | `Kiro: List Specs — Insert at Cursor` | No | Variant | Full text of `ws.specs` inserted into the active editor buffer. |
| `ws.current` | `Kiro: Current Spec` | Yes | No | Reports the active spec name, resolution source, document presence, and findings. |
| `nav.context` | `Kiro: Spec Context` | Yes | Yes | Reports which specs, components, tasks and criteria own the active file. |
| `nav.context.insert` | `Kiro: Spec Context — Insert at Cursor` | Yes | Variant | Inserts reference line(s) (`kiro-spec: ...`) at the cursor wrapped in the active file's comment syntax. |
| `nav.files` | `Kiro: Files by Component` | Yes | No | Lists design files by component with responsibilities and bound tasks. |
| `nav.task-files` | `Kiro: Files by Task` | Yes | No | Lists tasks with boundary components and component files; optionally filtered by task id. |
| `nav.to-requirements` | `Kiro: Requirements Outline` | Yes | No | Outlines headings from the current spec's requirements document. |
| `nav.to-design` | `Kiro: Design Outline` | Yes | No | Outlines headings from the current spec's design document. |
| `nav.to-tasks` | `Kiro: Tasks Outline` | Yes | No | Outlines headings and task items merged in line order from the current spec's tasks document. |
| `viz.status` | `Kiro: Spec Status` | Yes | Yes | Displays specification status, phase, approvals, task completion, and figures. |
| `viz.status.insert` | `Kiro: Spec Status — Insert at Cursor` | Yes | Variant | Inserts the specification status overview into the active editor buffer. |
| `viz.graph` | `Kiro: Spec Graph` | Yes | Yes | Displays requirements-to-files structure graph (Mermaid flowchart or Markdown table). |
| `viz.graph.insert` | `Kiro: Spec Graph — Insert at Cursor` | Yes | Variant | Inserts the specification structure graph into the active editor buffer. |
| `viz.tasks` | `Kiro: Task Graph` | Yes | Yes | Displays task dependency graph (Mermaid flowchart or Markdown table). |
| `viz.tasks.insert` | `Kiro: Task Graph — Insert at Cursor` | Yes | Variant | Inserts the task dependency graph into the active editor buffer. |
| `vocab.term` | `Kiro: Vocabulary Term` | Yes | No | Looks up a term within the current spec's declared topics. When invoked without arguments, displays usage guidance and the visible terms table. |
| `vocab.list` | `Kiro: List Vocabulary Terms` | Yes | Yes | Lists all visible terms for the current spec's declared topics; optionally filtered by a declared topic argument. |
| `vocab.list.insert` | `Kiro: List Vocabulary Terms — Insert at Cursor` | Yes | Variant | Inserts the visible terms table into the active editor buffer. |
| `vocab.topics` | `Kiro: Vocabulary Topics` | Yes | No | Reports declared, available, not declared, and unknown topics along with vocabulary validation findings. |
| `check.spec` | `Kiro: Check Spec` | Yes | Yes | Runs EARS, declared-term, and contradiction checks for the target spec. |
| `check.spec.insert` | `Kiro: Check Spec — Insert at Cursor` | Yes | Variant | Inserts the complete spec check report into the active editor buffer. |
| `check.ears` | `Kiro: Check EARS Shape` | Yes | No | Checks acceptance criteria for EARS compliance and displays pattern breakdown. |
| `check.terms` | `Kiro: Check Terms` | Yes | No | Resolves declared terms in spec topics and suggests visible undeclared terms. |
| `check.contradictions` | `Kiro: Check Contradictions` | Yes | No | Evaluates incompatible criteria pairs for joint satisfiability with witness states. |

*Note*: For spec-addressable commands like `ws.current`, `vocab.term`, `vocab.list`, `vocab.topics`, `check.spec`, `check.ears`, `check.terms`, and `check.contradictions`, dynamic per-spec variants (`<command>.<spec>`, e.g., `check.spec.demo`, `check.ears.plugin-core`) are registered automatically for each known spec in the active project within the command budget.

---

## 5.1 Vocabulary Grounding & Graceful Degradation

The vocabulary capability grounds requirements and design artifacts in a controlled domain glossary:

- **Programmatic Argument Form**:
  - `vocab.term`: Pass-through arguments joined by single spaces form the query (e.g. `plugin.kiro.vocab.term "acceptance criterion"` or `vocab.term.demo task`).
  - `vocab.list`: The first token optionally specifies a declared topic to filter by (e.g. `plugin.kiro.vocab.list sdd`).
  - `vocab.topics`: Arguments are ignored; the command reports topic status and findings for the target spec.
- **Graceful Degradation**:
  - **No Vocabulary**: If `.kiro/vocabulary.toml` is absent, vocabulary commands return an informational notification pointing to `.kiro/vocabulary.toml` and `docs/vocabulary.md` rather than failing.
  - **Invalid Vocabulary**: If `.kiro/vocabulary.toml` contains syntax or schema errors, an error notification reporting the `VOCAB_INVALID` finding is presented.
  - **Undeclared Topics**: If a spec does not declare `"topics"` in its `spec.json`, commands guide the author to declare topics and list all available topics in the vocabulary.
  - **Findings Summary**: When vocabulary validation findings exist on a loaded vocabulary, a reminder line `Vocabulary findings: N (run Kiro: Vocabulary Topics)` is appended to term and list outputs.

---

## 5.2 Spec Navigation & Ownership

The navigation capability provides cross-spec ownership tracing, component and task coverage views, and document outlines:

- **Programmatic Argument Forms**:
  - `nav.context [spec]`: Traces ownership of the active editor file across specs. When `spec` is provided, restricts checking to that spec; otherwise checks all known specs in roadmap order.
  - `nav.files [spec]`: Lists design files by component with declared responsibilities and bound implementation tasks.
  - `nav.task-files [spec] [task-id]`: Lists tasks with their boundary components and files. When `task-id` is provided (e.g., `2` or `2.1`), filters output to that major task and its subtasks or to that subtask alone. When combined with a per-spec variant (`nav.task-files.<spec>`), any argument repeating the spec must agree with `<spec>`, or specify only the `task-id`.
  - `nav.to-requirements [spec]`, `nav.to-design [spec]`, `nav.to-tasks [spec]`: Outlines headings and sections for quick navigation. For `nav.to-tasks`, task items are merged with headings in line order.

- **Comment Syntax for Reference Insertion**:
  When inserting a spec reference (`nav.context.insert`), the inserted line is wrapped in the file's native comment syntax:

  | Style | Comment Markers | Recognized Extensions / Basenames |
  | :--- | :---: | :--- |
  | Double Slash | `// text` | `.c`, `.h`, `.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.cs`, `.java`, `.kt`, `.kts`, `.swift`, `.go`, `.rs`, `.js`, `.mjs`, `.cjs`, `.jsx`, `.ts`, `.tsx`, `.dart`, `.scala`, `.groovy`, `.php`, `.zig`, `.wit`, `.proto` |
  | Hash | `# text` | `.py`, `.rb`, `.sh`, `.bash`, `.zsh`, `.ps1`, `.psm1`, `.psd1`, `.toml`, `.yaml`, `.yml`, `.pl`, `.pm`, `.r`, `.cmake`, `.ex`, `.exs`, `.nim`, `.tf`, `.hcl`, and extension-less `Makefile`, `Dockerfile`, `Justfile` |
  | Double Dash | `-- text` | `.sql`, `.lua`, `.hs`, `.elm`, `.ada`, `.adb`, `.ads`, `.vhd`, `.vhdl` |
  | HTML | `<!-- text -->` | `.md`, `.markdown`, `.html`, `.htm`, `.xml`, `.svg`, `.vue`, `.svelte`, `.xaml`, `.csproj`, `.fsproj`, `.vbproj`, `.props`, `.targets` (any `--` inside text is replaced by `- -`) |
  | Bare | `text` | Unlisted extensions (e.g. `.json`), dotfiles (e.g. `.gitignore`), and unlisted extension-less files |

- **Manual Host Check (Requirement 8.7)**:
  1. Open a source file in OpenIDEA (e.g. `plugins/kiro/src/lib.rs`).
  2. Open Search Everywhere (`Ctrl+Shift+A`) and run **Kiro: Spec Context**.
  3. Verify that the notification balloon displays the owning spec, components, tasks and criteria.
  4. Place the editor cursor on an empty line and run **Kiro: Spec Context — Insert at Cursor**.
  5. Verify that a `kiro-spec: ...` reference line is inserted at the cursor wrapped in the appropriate comment marker (e.g. `// kiro-spec: ...`) and the notification confirms insertion.

---

## 5.3 Spec Visualization & Structural Views

The visualization capability produces pure cross-document structural views of a specification:

- **Programmatic Argument Forms**:
  - `viz.status [spec]`: Displays specification phase, approvals table, task completion statistics with per-major progress, requirements/design figures, and successor chain. Accepts no additional options.
  - `viz.graph [spec] [text]`: Renders the requirements → criteria → tasks → components → files dependency and containment graph. By default outputs a fenced Mermaid flowchart (`graph LR`); when the `text` option is provided, outputs the complete Markdown table.
  - `viz.tasks [spec] [text]`: Renders the task breakdown and dependency DAG (`_Depends:_`, ordering, `(P)` parallel markers). By default outputs a fenced Mermaid flowchart (`graph TD`); when the `text` option is provided, outputs the complete Markdown table.
- **Option Syntax & Rules**:
  - Options follow the spec name: e.g. `viz.graph <spec> text` or `viz.tasks demo text`.
  - Options cannot be passed from the IDE's Search Everywhere UI, which triggers commands without arguments.

---

## 5.4 Spec Constraints & Quality Checks (`check.*`)

The constraints checking capability validates specifications against formal consistency, vocabulary grounding, and EARS requirements:

- **Programmatic Argument Forms**:
  - `check.spec [spec]`: Runs all three checks (EARS with pattern reporting off, term scope resolution with suggestions on, and contradiction search). Shows title with finding counts, scope line, three check status lines, and sorted findings.
  - `check.spec.insert` or `check.spec [spec] insert`: Inserts the complete check report into the active editor buffer at the cursor.
  - `check.ears [spec]`: Runs the EARS check with pattern reporting enabled. Displays the criteria count, finding count, the `Patterns:` breakdown line, and individual criteria findings.
  - `check.terms [spec]`: Runs scoped term resolution and term suggestions. Displays declared term breakdown (resolved, deprecated, mismatched, undeclared, ambiguous, suggested) followed by findings.
  - `check.contradictions [spec]`: Runs the bounded BDD contradiction search over guarded criteria and declared conflicts. Displays guarded criteria count, candidate pairs considered, contradictions found, refused pairs, and active variable/node caps, followed by witness states.
- **Graceful Degradation**:
  - **No Annotations**: Documents without annotations produce no warning or error findings; `CHECK_COVERAGE` info finding reports coverage statistics.
  - **Undeclared Topics / Missing Vocabulary**: When `spec.json` declares no topics or `.kiro/vocabulary.toml` is missing or invalid, commands complete successfully with explanatory status lines, pass-through vocabulary findings, `CHECK_UNSCOPED` (if annotations exist), and a guidance pointer (`See docs/vocabulary.md or run Kiro: Vocabulary Topics.`).
  - **No Findings**: Commands explicitly display `No findings.` rather than an empty notification body.

---

## 6. Three-Tier Output Contract

The plugin provides three complementary presentation tiers:

1. **Notification Balloon (IDE Toast / Summary)**:
   - Displays immediate status and summary metrics.
   - Capped at **15 KiB** (below the host's 16 KiB limit).
   - When text exceeds 15 KiB, it is cleanly cut at a line boundary with an omitted-lines trailer:
     - For insert-capable commands: `… N more lines — run <id>.insert for the full text`
     - For other commands: `… N more lines`
2. **Editor Insertion (`.insert` variant or `insert` argument)**:
   - Inserts the complete, unclipped text payload at the cursor in the active editor buffer.
   - Capped at **1 MiB - 1 KiB** (respecting host memory buffers).
3. **CLI Durable Record (`kiro-cli` JSON)**:
   - Command-line invocations produce structured JSON or CLI tables for scripting, Git pre-commit hooks, and CI/CD pipelines.

---

## 7. Argument Grammar for Programmatic Callers

When triggering commands via Search Everywhere or programmatically:

- Arguments are tokenized by whitespace (`split_whitespace()`).
- The keyword `insert` in any token position sets the insertion flag and is stripped from arguments.
- For commands with `positional = Positional::Spec` (such as `ws.current`):
  - The first remaining token specifies the target spec name (e.g. `ws.current plugin-core`).
  - Spec names must match `[a-z0-9][a-z0-9._-]*` and contain no path separators (`/`, `\`) or `..`.
- If an explicit spec is supplied both in a variant ID (`ws.current.demo`) and in the argument (`old`), conflicting values are rejected.
- All other tokens pass through to the owning feature module.

---

## 8. Host Sandbox Limits & Budgets

The WebAssembly plugin runs in a sandboxed host environment under strict resource budgets:

- **Notification Budget**: 16 KiB hard ceiling; plugin clips at 15 KiB on line boundaries.
- **Title Budget**: 256 bytes hard ceiling; cut on UTF-8 character boundaries.
- **Editor Insertion Budget**: 1 MiB hard ceiling; plugin clips at 1,047,552 bytes on line boundaries.
- **Command Registration Limit**: 256 total registered commands per plugin instance (static + dynamic per-spec variants).
- **Execution Timeout**: Host cancels commands that take longer than 5 seconds (standard Kiro operations complete in < 100 ms).

---

## 9. Verification & Binary Metrics

Validation of the WebAssembly component binary and dev-plugin integration:

- **Component Binary Size**:
  - **Baseline (Task 1.1)**: 13,196 bytes
  - **After Vocabulary Grounding**: 292,878 bytes (includes `toml` parser and `vocab` module)
  - **After Spec Navigation**: 708,150 bytes (includes `nav` module, join ownership and coverage, comment wrapping)
  - **After Spec Visualization**: 754,260 bytes (includes `viz` module, Mermaid flowcharts, and table renderers)
  - **After Spec Constraints**: 950,313 bytes (includes `check` module, `biodivine-lib-bdd` engine, and guard parsers)
- **Host Integration Verification**:
  - Live GUI check deferred: No interactive OpenIDEA GUI host instance was attached during automated verification. Dev-plugin junction (`.openidea/plugins/kiro` → `plugins/kiro`), manifest registration, and command behaviors are verified end-to-end via `plugins/kiro/tests/check.rs`, `plugins/kiro/tests/viz.rs`, `plugins/kiro/tests/nav.rs`, `plugins/kiro/tests/vocab.rs`, `plugins/kiro/tests/dispatch.rs`, and `plugins/kiro/tests/manifest.rs`.

---

## 10. Check commands: verification

- **Component Size**:
  - **Before `spec-constraints` (baseline from 1.1)**: 754,260 bytes
  - **After `spec-constraints`**: 950,313 bytes (+196,053 bytes)
- **Host Check Outcome**:
  - Manual host check as a dev plugin deferred: No interactive OpenIDEA GUI host instance was running during automated execution.
  - Verification is covered end-to-end by simulated host test harness in `plugins/kiro/tests/check.rs` across all 10 scenarios:
    1. `check.spec` on clean spec (`0 errors, 0 warnings, 1 info` with `CHECK_COVERAGE`, info level, three status lines).
    2. `check.spec` on broken spec (`Error` level, counts in title, `CONTRADICTION` finding lines).
    3. `check.ears` on broken spec (`Patterns:` breakdown line and individual `EARS_PATTERN` info reports).
    4. `check.terms` on broken spec (declared breakdown, suggestions count, `TERM_TOPIC_MISMATCH`, undeclared, deprecated, ambiguous).
    5. `check.contradictions` on broken spec (guarded criteria count, pairs considered, active caps, and both witness states).
    6. `check.spec` on unscoped `none` spec (skipped lines for terms and contradictions, `TOPICS_UNDECLARED`, `CHECK_UNSCOPED`, and documentation pointer).
    7. Missing and invalid `.kiro/vocabulary.toml` degradation completing successfully with warning/error notifications, skipped lines, and documentation pointers.
    8. `check.spec.insert` producing exactly one text insertion event with unclipped report.
    9. Unresolved spec error handling matching plugin-core contracts.
    10. Single vocabulary file read cache invariant per command execution.
