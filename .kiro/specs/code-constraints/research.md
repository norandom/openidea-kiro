# Research & Design Decisions: code-constraints

## Summary
- **Feature**: `code-constraints`
- **Discovery Scope**: Complex Integration — a new measurement layer (native tree-sitter plus report adapters) joined to the existing engine (ownership join, finding shape, CLI gate), with the rule catalogue and defaults transferred from `../scitools-hook` where its measurements apply and two new rules where it has none. Requirements-phase discovery 2026-11-09; design-phase discovery (full) 2026-11-09.
- **Key Findings**:
  - scitools-hook's rule catalogue splits cleanly into (a) metrics any parser can take (cyclomatic strict, nesting, lines, statements, parameters, per-file and per-type counts), (b) metrics only Understand computes (`Essential`, `CountPath`, coupling, cohesion, inheritance), (c) lexical rules that need only a token stream (duplicate blocks, similar-routine families, scattered definitions), and (d) reference rules that need resolved calls (dead code, pass-through, cycles, layers). This spec takes (a) natively, (b) through the scitools-hook JSON adapter, (c) natively, and defers (d).
  - scitools-hook has **no** rule for a state machine spread across files and **no** rule for positional versus role-named data access. "Scattered state" generalises its scattered-definitions mechanism (group project-wide, key on identity and value, report against the selection) from bindings to literal comparisons; "role-named data access" is the roadmap's deferred "idiomatic naming in data code", returning as a vocabulary-grounded rule per the 2026-11-04 decision.
  - The two lexical duplication rules are fully specified in scitools-hook's own source with measurements: line hashes over lexemes (12-line window), token shapes with identifiers → `ID` and literals → `LIT`, ratio `2·matched/(la+lb)` at 0.9 over ≥ 6 statements, families by BFS, nested-twin exclusion, two **exact** pruning bounds (length band, token multiset) that make the pass lossless and cut a 29.9 s quadratic pass to 24.4 s whole-project on 2 405 routines. All of it ports to a tree-sitter token stream.
  - Linters (ruff, clippy, pylint) report **breaches**, not **values**; a ratchet needs values. They are complementary, importable later as breach-only findings, and never a before side.
  - Design phase: every engine contract the gate needs already exists and is stable (`Finding`, `CODES` growth path with the doc-table test, `join::owners_of`/`path_matches`, `Workspace`, `Vocabulary::resolve` with five `Lookup` outcomes, `FsSource`, `Exit`/`FailOn`, the Dagger gate). Measurement is the only new capability, and it must stay out of the wasm engine because tree-sitter carries a C runtime.
  - Design phase: the scitools-hook JSON report (`RunResult`, schema 2) carries values only for the entities it reported plus `highest[]`; it is a partial measurement and the design says so. jscpd's JSON has two key spellings in the wild (`firstFile`/`secondFile` upstream, `duplicationA`/`duplicationB` in the Rust port); the adapter accepts both.
  - Amendments A1–A3 (2026-11-09): QOpenBB's `.kiro/breadth.toml` (200 capabilities; keys `id`, `requirements`, `planned_checks`, `test_paths`, `status`, `evidence`; statuses `planned`, `unverified`, `partial`, `implemented`, `passed`, `deferred`) is kept aligned by steering rules, not by a checker — this spec adds the checker as a pure engine module plus an `e2e_paths` field. QOpenBB's verification policy makes real-journey acceptance primary ("assertions verify displayed data and user effects, not just a toast"). Its seven Python CLIs share `--output`/`--report` conventions and the 0/1/2 exit contract by copy, not by a library — here the standard becomes a crate every command composes.

## Feature Map: scitools-hook → code-constraints

Legend for **Decision**: `native` = measured by this spec's tree-sitter measurer; `adapter` = imported from a report (scitools-hook JSON, jscpd JSON); `custom` = new rule with no pendant; `deferred` = out of this spec, named for a later one; `n/a` = Understand-specific, no counterpart needed.

### Routine thresholds (the complexity gate)

| scitools-hook rule (default, severity, ratchet) | What it counts | Open-source pendant(s) | Decision | Here |
|---|---|---|---|---|
| `routine.CyclomaticStrict` (10, error, yes) | decision points incl. `&&`/`\|\|` | lizard CCN (multi-language), rust-code-analysis `cyclomatic`, ruff `C901`/`PLR0912` (breach only), clippy `cognitive_complexity` (breach only, nursery) | native + adapter | Req 3.1, `CODE_ROUTINE_LIMIT` |
| `routine.CyclomaticModified` (8, error, yes) | `switch` counts once | none common | adapter only (scitools-hook) | Req 3.5 |
| `routine.Essential` (4, **warning**, yes) | unstructured control flow | none | adapter only | Req 3.5 |
| `routine.MaxNesting` (3, error, yes) | deepest nesting | ruff `PLR1702` (preview, breach only) | native | Req 3.1 |
| `routine.CountLineCode` (60, error, yes) | lines with code | lizard NLOC, rust-code-analysis `loc`, clippy `too_many_lines` (breach, default 100) | native + adapter | Req 3.1 |
| `routine.CountStmt` (40, error, yes) | statements | ruff `PLR0915` (breach, default 50) | native | Req 3.1 |
| `routine.CountParams` (5, error, yes; synthetic in scitools-hook) | declared parameters | lizard, rust-code-analysis `nargs`, ruff `PLR0913` (default 5), clippy `too_many_arguments` (default 7) | native | Req 3.1 |
| `routine.CountPath` (100, error, yes) | acyclic paths | none | adapter only | Req 3.5 |
| `routine.CountLineComment` (20, **warning**, yes) | comment lines (docstrings count) | none | native | Req 3.2 |
| `routine.LinesPerStatement` (4, **warning**, judged from 5 statements) | `CountLineCode / CountStmt` | none | native | Req 3.2 |
| `CognitiveComplexity` (8.0 metric, off) | cognitive complexity | rust-code-analysis `cognitive`, complexipy (Python), clippy | adapter; native deferred | Req 3.5 |

### Type and file thresholds (anti god-type, anti god-file)

| scitools-hook rule (default, severity, ratchet) | Pendant(s) | Decision | Here |
|---|---|---|---|
| `class.CountDeclMethod` (20, error, **no**) | pylint `R0904` / ruff `PLR0904` (breach) | native | Req 4.2, `CODE_TYPE_LIMIT` |
| `class.CountDeclMethodNonStub` (15, error, no; synthetic) | none | n/a (C#-specific stub notion) | documented, not shipped |
| `class.CountDeclInstanceVariable` (10, error, **no**) | pylint `R0902` (breach) | native | Req 4.2 |
| `class.MaxInheritanceTree` (4, warning, yes) | none without resolution | adapter only | Req 4.5 |
| `class.CountClassDerived` (8, error, no) | none without resolution | adapter only | Req 4.5 |
| `class.CountClassCoupled` (12, error, no) | none without resolution | adapter only | Req 4.5 |
| `class.PercentLackOfCohesion` (70, warning, yes) | none | adapter only | Req 4.5 |
| `file.CountLineCode` (500, error, **no**) | tokei / scc, lizard per-file NLOC, rust-code-analysis per-file | native + adapter | Req 4.1, `CODE_FILE_LIMIT` |
| `file.CountDeclFunction` (25, error, **no**) | lizard (function list per file) | native + adapter | Req 4.1 |
| `file.CountDeclClass` (3, error, **no**) | none | native, **8 as warning** until measured on Rust/TS (scitools' 3 was measured on Python) | Req 4.1 |
| `file.MaxCyclomaticStrict` (10, error, yes) | derived from routine values | native | Req 4.1 |
| `file.RatioCommentToCode` (min 0.1, warning, yes) | none | deferred (shrink family, low value for a gate) | — |
| "the eight rules with the ratchet off" | — | **adopted**: container counts are never ratcheted (measured by scitools-hook: extracting helpers moved `CountDeclFunction` 1→3 while complexity improved) | Req 4.4, 9.5 |
| `project.AVG:*`, `MAX:*` stats | — | deferred (needs a population model; `--show-highest` covers the first-run need) | Req 3.6 |

### Duplication and similarity (the lexical "semantic similarity")

| scitools-hook rule | Mechanism (as implemented there) | Pendant(s) | Decision | Here |
|---|---|---|---|---|
| `structure.duplicate_block` (off; 12 lines) | lexemes minus whitespace/comments, 64-bit truncated SHA-256 per line, window ≥ 12 aligned equal hashes, both ends reported, self-overlapping runs dropped, up to 3 other locations named | **jscpd** v5 (Rust engine; `--min-tokens` 50, `--min-lines` 5; JSON, SARIF, pmd-cpd XML; 200+ formats), **PMD CPD** (`--minimum-tokens`, `--ignore-identifiers`/`--ignore-literals`), pylint `R0801` (Python, 4 lines), lizard `-Eduplicate` | native (port) + jscpd adapter | Req 5.1, 5.7, `CODE_DUPLICATE_BLOCK` |
| `structure.similar_routine` (off; 0.9 / 6 statements / family ≥ 2) | token shapes (`Identifier` → `ID`, `String`/`Literal` → `LIT`), `SequenceMatcher.ratio()` = `2·matched/(la+lb)`, exact pruning by length band and token multiset, BFS families from the selection, nested twin excluded, `similar_name_ignore` (dunders, setUp/tearDown), same-file → `shrink:`, cross-file → `delete:` | none produce **families**; CPD with ignore-identifiers approximates parameterised clones at block level | **custom native port** | Req 5.2–5.6, `CODE_SIMILAR_FAMILY` |
| semantic re-implementation (not a scitools-hook rule) | measured gap: 1.00 rename, 0.64 guard→ladder, 0.34 table-driven | **slopo** (AGPL-3.0, Python, embeddings via external provider, periodic Markdown report) | out of the gate by design; documented as a periodic report | Req 5.6 |
| `structure.net_growth` / the net line | `CountStmt` added − removed over routines of the change | none | native | Req 9.7, `CODE_NET_GROWTH` |

### Scattered state (the user's focus)

| Rule | Mechanism | Pendant(s) | Decision | Here |
|---|---|---|---|---|
| `structure.duplicate_definition` (off; `duplicate_definitions = 3`) — scattered **definitions** | module-level bindings (`Definein` owner is a file), initialiser text ≤ 12 lines, group by (name, value), count distinct files > limit, report against selected files, `ignore` by **name** (`log`, `logger`, `pytestmark`); one-name-two-values reported as two groups | none (semgrep/ast-grep can find assignments, not the cross-file same-value grouping) | **custom native** | Req 6, `CODE_SCATTERED_DEFINITION`, `CODE_DEFINITION_AMBIGUOUS` |
| scattered **state machine** (new) | literal comparison sets per subject name project-wide; `min_states` 2, `max_files` 3; enum exhaustively matched in many files at info | none | **custom native** | Req 7, `CODE_SCATTERED_STATE` |
| scattered **schema** (new, data companion) | column-name literal used in > `max_files` files | none | custom native | Req 8.4, `CODE_SCATTERED_SCHEMA` |

### Idiomatic data code (roadmap deferred item, returning)

| Rule | Mechanism | Pendant(s) | Decision | Here |
|---|---|---|---|---|
| positional column/axis/field access | per-language receiver + position patterns (Python: pandas `iloc[:, N]`, polars `df[:, N]`, NumPy `a[:, N]`, `row[N]`; Rust: polars `select_at_idx`/`df[N]`-style, ndarray `a.column(N)`/`a[[i, N]]`) | ast-grep / semgrep patterns (both pattern engines; ast-grep is tree-sitter based, MIT, has `ast-grep-core` 0.51.1) | **custom native** as tree-sitter queries (ast-grep-core evaluated and not adopted, see decisions) | Req 8.1, `CODE_POSITIONAL_ACCESS` |
| column names grounded in the vocabulary | names in column-selection positions resolved via `vocab::resolve` within configured data topics | none — the kiro-specific twist | custom, on vocabulary-grounding | Req 8.2–8.3, `CODE_COLUMN_UNGROUNDED` |

### Mechanism, contract and operations

| scitools-hook feature | Decision | Here |
|---|---|---|
| Ratchet: per-entity before/after, `EntityKey(scope, path, longname, parameters, ordinal)`, `family` pairing for signature changes, pre-existing never blocks, `below_limit_severity = warning` | **adopted** as the ratchet model | Req 9 |
| Before side from git (`HEAD` shadow or `-gitcommit`) | adopted as `--changed <rev>`; **plus** a committed baseline file so a tree without git history can be gated | Req 9.4 |
| Adaptive baseline that only narrows | deferred (the committed baseline is a snapshot; narrowing is a later feature) | — |
| `analysis.parse_error` blocks in selection, reports outside | adopted (`CODE_PARSE_FAILED`) | Req 11.2, 11.3 |
| `CHECK_COVERAGE`-style trust signal (spec-constraints precedent); scitools' per-rule "was not evaluated" lines | adopted as `CODE_COVERAGE` | Req 11.1 |
| Hints with tags (`delete:`, `yagni:`, `shrink:`; never `stdlib:`/`native:`) | adopted and extended: `simplify:`, `split:`, `delete:`, `shrink:`, `collect:`, `name:`, `fix:`; rungs 3–5 stated as never checked | Req 12.2, 11.6 |
| `agent-rules --write` block between markers | adopted as `code rules --write` into `.kiro/steering/code-rules.md` (steering is auto-loaded by cc-sdd) | Req 12.1 |
| Five skills; gate/improve refuse to edit configuration | adopted as amendments to `kiro-impl`, `kiro-review`, `kiro-validate-impl`; adaptation is human-owned with written reasons (`reason` keys) | Req 12.4–12.6, 2.4 |
| `[scope.<name>]` per-directory limits; `init --detect` proposing a tests scope | scopes adopted; detection deferred | Req 2.3 |
| `--format json` schema-versioned; SARIF 2.1.0 | adopted | Req 12.3, 13.3 |
| Exit codes 0–7, 70 | collapsed to this project's `0/1/2` contract | Req 13.2 |
| `doctor`, licence probes, Understand database lifecycle, snapshot cache, Understand's own SARIF, generated architectures, accuracy floors, `resolution_floor` | n/a (Understand-specific) | — |
| `recommend` (measure and propose limits) | deferred | — |
| Structural rules (`file_cycle`, `arch_cycle`, `layer`, `new_dependencies`, `coupling`, `fan_in/out`) | **deferred → `structure-constraints`**; pendants: `cargo-modules` (Rust), import-linter / pydeps (Python), dependency-cruiser / madge (JS/TS) | — |
| Lean dead-code rules (`unused_parameter/class/variable`, `pass_through`, `single_implementation`, `over_export`, `unused_routine`) | **deferred → `lean-code`**; pendants: rustc/clippy dead-code lints, vulture, ruff `F841`/`ARG00x`, knip / ts-prune | — |

## Research Log

### What "semantic code similarity" means in scitools-hook
- **Context**: The user asked what scitools-hook does for semantic code similarity.
- **Sources Consulted**: `docs/compare/tools.md` §"Semantic clone detection: slopo and the embedding approach"; `docs/reference/rules.md` §`structure.similar_routine`, §`structure.duplicate_block`; `src/scitools_hook/analysis/lean/similar.py` and `duplicates.py` module docstrings; `src/scitools_hook/understand/worker_lean.py` (`token_index`, `_shape_text`, `IDENTIFIER_SHAPE = "ID"`, `LITERAL_SHAPE = "LIT"`, `_line_hash` = SHA-256 truncated).
- **Findings**: Both rules are **lexical by design** and take no trust floor because they make presence claims. The family rule is parameterised clone detection (Type-2 clones): it finds a routine copied and renamed and misses a routine re-implemented in another shape; measured ratios 1.00 / 0.64 / 0.34 for rename / ladder / table-driven rewrites of one function. Lowering the threshold to 0.85 tripled findings and quadrupled runtime on facdrone with no restructured twins gained. Families, not pairs: 69 pairs at 0.9 were 44 families on a 101 800-line code base. The pass is made lossless and fast by two exact ceilings on `matched`: `2·min(la,lb)/(la+lb) < t` and the token multiset overlap. A 4-gram shingle index was deleted after measurement (filtered 0.78 % of pairs).
- **Implications**: Port the algorithm over a tree-sitter token stream (identifier and literal node kinds per grammar); keep the exact bounds; keep families; keep the nested-twin exclusion and the name ignore list; document the blind spot and point to embedding tools as periodic reports outside the gate (Req 5.6).

### Scattered definitions, and what "scattered state" adds
- **Context**: The user asked to focus on scattered state machine and scattered variable findings.
- **Sources Consulted**: `docs/reference/rules.md` §"Scattered definitions: one value, many files"; `docs/guide/developer-guide.md` §2 "Scattered state"; `analysis/structure/definitions.py`; `understand/worker.py` (`MODULE_VARIABLE_KIND`, `DEFINITION_SPAN = 12`, `_definitions`); grep of the whole repository for "state machine" (no rule exists).
- **Findings**: scitools-hook's only rule in this family is `structure.duplicate_definition`: module-level **name + value** copied into > N files, project-wide count, reported against affected files, name-based ignore list for per-module idioms, unreadable initialisers never grouped, textual comparison (under-reports semantic equality, over-reports values that reference another name). Its triage advice ("are these one decision?") and the one-name-two-values case (`MIN_ACTIVE` = 3 in four modules and 5 in five) are the parts worth keeping as behaviour (Req 6.5) rather than prose. There is no rule for a state machine whose states are string/int literals compared all over the code base.
- **Implications**: Req 6 ports the rule. Req 7 generalises the mechanism from bindings to literal comparison sets keyed by subject name, with `min_states` and `max_files`, an info-level variant for enums exhaustively matched in many files, and honest limits (booleans, helper predicates, name collisions). Both rules use the same project-wide collect → group → report-against-selection shape, so one component (`RuleScattered`) owns them.

### Idiomatic data code: positional versus role-named access
- **Context**: The user asked for "array columns named according to role, same for dataframes"; the roadmap (2026-11-04) deferred "idiomatic naming in data code" to return as vocabulary topics.
- **Sources Consulted**: `.kiro/steering/roadmap.md` amendments; `vocabulary-grounding` design (`Vocabulary::resolve(label, topics)`, `spec_topics`); scitools-hook grep (no rule; `FloatArray = NDArray[np.float64]` appears only as a scattered-definition example).
- **Findings**: No pendant judges positional access as a rule; ast-grep and semgrep can match the patterns. Grounding names in a vocabulary is unique to this project and is exactly the deferred decision.
- **Implications**: Req 8: a pattern rule (`CODE_POSITIONAL_ACCESS`), a grounding rule over configured data topics (`CODE_COLUMN_UNGROUNDED`), a coverage statement when no data topic exists, and the scattered-schema companion. Shipped patterns for Python (pandas, polars, NumPy) and Rust (polars, ndarray); TypeScript data libraries are not shipped (no dominant idiom), extensible by configuration.

### The ratchet, the baseline and why linters cannot feed it
- **Context**: Only regression may block; the gate must work on a grown repository.
- **Sources Consulted**: `docs/argument/ratchet.md`, `docs/argument/economics.md` (230 blocking findings on scitools-hook's own `--all`, none of which blocks a staged commit), `models/snapshot.py` (`EntityKey`, `family`), `docs/reference/rules.md` §"What blocks", §"The eight rules with the ratchet off".
- **Findings**: The ratchet needs a value per entity on two sides and a stable key; `(scope, path, longname, parameters, ordinal)` with `family = (scope, path, longname)` pairing exactly one removed with exactly one added key handles signature changes. Growth inside the limit is a warning (added after a field report where the gate refused the refactorings its own hints asked for). Container counts are not ratcheted. Linters emit breaches without values.
- **Implications**: Req 9 adopts the model; a committed baseline (`.kiro/code-baseline.json`) complements `--changed <rev>` so CI on a shallow clone and a tree without history can still be gated; breach-only imports are never ratcheted (Req 1.3, 9.5) and surface as `CODE_IMPORTED`.

### Design phase: engine and CLI contracts consumed
- **Context**: The design must build on what exists without re-deriving it.
- **Sources Consulted**: `crates/kiro-core/src/{finding,join,source,design,tasks,roadmap}.rs`, `vocab/{mod,scope,model}.rs`, `render/text.rs`, `lib.rs` (accepted grammars, code table doc test); `crates/kiro-cli/src/{main,cli,exit,fs,check,report}.rs`; `.dagger/src/kiro/main.py`; `.cargo/config.toml`; `deny.toml`.
- **Findings**: `Finding` has six fields and a builder; `CODES` grows by appending entries and doc-table rows (the doc test asserts equality); `CHECK_CODES` is the precedent for a per-spec slice. `join::owners_of` returns components, tasks and criteria for a path through `path_matches` (suffix at `/` boundaries); `DesignDoc::files_of`, `TasksDoc::task` and `Task.boundary` are public. `Workspace::new(source, known)` plus `adopt_roadmap_features` and `RoadmapDoc::names()` give roadmap order. `Vocabulary::resolve` returns `Found | NotInTopic | Ambiguous | Unknown | Unscoped`; `visible(topics)` lists candidates for suggestions. `FsSource` is the only `std::fs` module in the CLI; `Exit`, `Failure`, `FailOn`, `meets`, `verdict`, `resolve_root`, `discover_features`, `read_argument_file`, `write_output` exist. The Dagger gate ends with `kiro-cli check --all --fail-on error`; `deny.toml` evaluates the wasm32 graph only, with `exclude-dev = true`.
- **Implications**: The engine gains a pure `code` module and 21 codes, nothing else; the CLI gains a `code` command family reusing `fs`/`exit`; the measurer is a new native crate so the wasm graph stays untouched; the Dagger gate gains one step.

### Design phase: tree-sitter and grammar versions
- **Context**: The native measurer needs pinned, compatible crates.
- **Sources Consulted**: `cargo search` on 2026-11-09: `tree-sitter = "0.27.2"`, `tree-sitter-rust = "0.24.2"`, `tree-sitter-python = "0.25.0"`, `tree-sitter-typescript = "0.23.2"`, `tree-sitter-javascript = "0.25.0"`, `ast-grep-core = "0.51.1"`; docs.rs tree-sitter 0.26.x binding README (`LANGUAGE.into()` pattern); the Arch rebuild tracker for grammar package versions.
- **Findings**: Grammar crates expose a `LANGUAGE: LanguageFn` constant through `tree-sitter-language`, which decouples them from the core crate's minor version; ABI compatibility across 0.27 core and grammars built for 0.23–0.25 must still be verified by building and parsing a fixture in task 1 before any adapter work. Grammar node-kind names (not the ABI) are what the adapters depend on, and they change between grammar minors.
- **Implications**: Exact pins in `crates/kiro-code/Cargo.toml`; one fixture per language whose expected facts snapshot would catch a node-kind rename; the adapter trait isolates kind names in one file per language.

### Design phase: report shapes of the two adapters
- **Context**: Requirement 1.2 names scitools-hook JSON and jscpd JSON as sources.
- **Sources Consulted**: `D:\Source\scitools-hook\src\scitools_hook\report\json_out.py` (the document *is* `RunResult.model_dump_json(indent=2)`), `models/findings.py` (`RunResult` schema 2: `findings[]`, `highest[]`, `parse_errors[]`, `net_delta`, `effective_thresholds`, `unavailable_metrics`, `analyzed_files`, counts; `Finding` with `kind`, `rule`, `metric`, `scope`, `entity.key{scope,path,longname,parameters,ordinal}`, `value`, `before`, `limit`, `preexisting`, `hint`), `models/snapshot.py` (`EntityRef`, `EntityKey`); jscpd 5.3.0 README and the `jscpd-rs` `report/json.rs` and `detector/model.rs` sources (`duplicates[]` with `format`, `lines`, `tokens`, `fragment`, `firstFile`/`secondFile` or `duplicationA`/`duplicationB`, each with `name`, `start`, `end`, `startLoc`, `endLoc`).
- **Findings**: scitools-hook's report holds values only for entities that produced a finding plus the `highest[]` list, so it is a **partial** measurement — enough for limits and the ratchet on those entities, not a census. Its `hint` field can travel into `CODE_IMPORTED`. jscpd reports clone pairs with line ranges; token counts are informational.
- **Implications**: `ScitoolsAdapter` maps `threshold` records and `highest[]` to `EntityFacts`, everything else to `ImportedFinding`; `FileFacts.measured_by` records partial coverage; `JscpdAdapter` accepts both key spellings and produces `CloneRecord`s deduplicated against native windows.

### Design phase: the before side from git
- **Context**: Requirement 9.4 needs file contents at a revision without a shadow checkout.
- **Sources Consulted**: git 2.56 on the development machine; `git diff --name-status -z <rev>`, `git cat-file --batch` semantics (one process, `<rev>:<path>` per line, blob headers with sizes).
- **Findings**: Two processes per run suffice: one to list changed and deleted paths, one to stream every needed blob. No checkout, no temporary tree, no per-file process. A pure-Rust git library (`gix`) would avoid the subprocess but adds a large dependency to a native crate that may already assume git is present for `--changed`.
- **Implications**: `GitBefore` shells out to `git`; `--changed` on a tree without git or with an unknown revision exits 2 with the git message on one line.

### Design phase: algorithm and hashing choices
- **Context**: The family rule's threshold was calibrated on Python's `SequenceMatcher`; the duplicate rule's margin argument rests on a 64-bit hash.
- **Findings**: `SequenceMatcher.ratio()` is the Ratcliff/Obershelp gestalt measure (recursive longest common block) with an autojunk heuristic that only triggers above 200 elements; implementing the gestalt measure without autojunk keeps the measured 0.9 meaningful and is a few dozen lines over `&[u32]`. For hashing, FNV-1a 64-bit is deterministic, dependency-free and adequate for a non-adversarial equality test at this scale (the birthday bound at a million distinct lines is ~2.7e-8, the same order as scitools-hook's truncated SHA-256); it also serves as the configuration digest.
- **Implications**: `families::ratio` implements gestalt matching; `facts::hash64` implements FNV-1a; both documented in `docs/code-rules.md`.

### Amendment A1: QOpenBB's breadth ledger
- **Context**: The owner asked for "a ../QOpenBB style breath.toml workflow" before approving the task plan.
- **Sources Consulted**: `D:\Source\QOpenBB\.kiro\breadth.toml` (1 609 lines, 200 `[[capabilities]]`; one stray header with `schema_version`, `owning_spec`, `requirements_file`, `draft`), `AGENTS.md` ("Keep `.kiro/breadth.toml` aligned with capabilities, owning spec criteria, actual test paths and evidence. Planned checks are not completed tests."), `.kiro/steering/testing.md` §"Breadth and evidence" ("links capability -> spec criteria -> planned/implemented test -> evidence status. Planned test IDs are not executable tests. Never label a capability verified based on an empty or skipped suite."), `.kiro/steering/delivery-workflow.md` ("the breadth map retains actual criterion/test/evidence links"; checkbox reconciliation in the completion commit), `.kiro/spec-ownership.toml` (scope handover per spec) and `.kiro/complexity-budgets.json` (per-file exceptions with baseline, revision and rationale — the same reason-carrying shape as this spec's configuration).
- **Findings**: Requirement references are free-form strings (`market-data-integrations/requirements.md:4.1-4.7`, `...:P4`, `...:Project Description`); test paths are plain paths (e2e specs among them); status counts on 2026-11-09: 173 passed, 14 unverified, 10 planned, 1 partial, 1 implemented, 1 deferred; evidence records are dated prose naming commands, revisions, digests and review documents. Integrity is a human rule — no test or tool in QOpenBB parses the ledger.
- **Implications**: This project can check what QOpenBB only asks for: references resolve against parsed criteria, paths exist through the read port, statuses need the tests and evidence they claim, completed tasks with a still-planned capability are stale, and unmapped criteria are visible. The roadmap's deferred "requirement → test traceability" returns as this ledger. The schema stays QOpenBB-compatible (`<spec>/requirements.md:<ref>` accepted beside `<spec>:<ref>`) and adds `e2e_paths` for A2.

### Amendment A2: end-to-end first
- **Context**: The owner asked for "an emphasis on end to end testing".
- **Sources Consulted**: QOpenBB `.kiro/steering/testing.md` (isolated headless browsers, Playwright regressions against the real application, success and failure/recovery cases, "browser checks do not prove financial calculations"), `delivery-workflow.md` ("candidate-image E2E acceptance per slice", "synthetic regressions ... cannot alone establish feature completion"), `docs/evidence.md` (evidence validated as records with exit codes 0/1/2), this spec's existing CLI integration tests (fixture trees, temp dirs, exit codes).
- **Findings**: The transferable principle is that acceptance is a journey a user would take, observed from the outside (output, exit code, files), recorded with command, revision and result; unit tests pin algorithms. For a CLI the "real application" is the built binary over a real project and a real git repository.
- **Implications**: One e2e test binary with a harness and seven journeys mirroring the requirements' user stories; an evidence line per step; CI runs it; `passed` in the ledger needs an e2e path (status rule) — so A1 and A2 enforce each other.

### Amendment A3: one parameter standard, as a library
- **Context**: The owner asked that "the CLIs follow a common parameter standard (using a lib)".
- **Sources Consulted**: QOpenBB `pipeline/qopenbb_ci/{cli,quality_cli,schemas}.py` (argparse; `--output` on four of seven CLIs, `--input`/`--expected`/`--evidence`/`--report`; exit 0 valid, 1 gate failed, 2 invocation or I/O failure; JSON report to a path and stdout; `--version`); this repo's `crates/kiro-cli/src/cli.rs` (`--project`/`--root`, `--verbose`, `--color` global; `--format`/`--fail-on` on `check` and `vocab validate`; `--table` on `graph`/`tasks`; `--out` on `vocab import`); `cargo search` 2026-11-09: `clap-verbosity-flag = "3.0.4"`, `clap_complete = "4.6.11"`, `clio = "0.3.5"`, `clap-cargo = "0.19.0"`.
- **Findings**: QOpenBB's consistency is by convention and copy; the drift this repo already shows (`--table` versus `--format`, `--out` versus an output flag) is the failure mode a library prevents. `clap`'s `#[command(flatten)]` makes shared `Args` structs composable across commands and binaries; `clap-verbosity-flag` is the de-facto `-v`/`-q` group; `clap_complete` generates completions from the definitions; `clio` (file arguments with `-`) was considered and not adopted because only `--output` needs the convention and a ten-line writer covers it.
- **Implications**: `crates/kiro-cli-common` with `GlobalArgs`, `OutputArgs`, `GateArgs`, the exit model and completions; every existing command migrates with hidden aliases so no invocation breaks; a conformance test over `--help` keeps every binary on the standard; `docs/cli-standard.md` is the human form.

### Consulted skills and guidance
- Requirements phase: `kiro-spec-init`, `kiro-spec-requirements` (EARS rules, review gate), `kiro-discovery` brief structure. Design phase: `kiro-spec-design` with `design-principles.md`, `design-discovery-full.md`, `design-synthesis.md`, `design-review-gate.md`; the design template. Steering: `workflow.md` (foreground mode; the user has overridden it for implementation — Opus xhigh subagents, recorded in `spec.json.notes`), `agent-guidance.md`, `roadmap.md`. No UI, accessibility or design skill applies (terminal text only).

## Architecture Pattern Evaluation

| Option | Description | Strengths | Risks / Limitations | Notes |
|--------|-------------|-----------|---------------------|-------|
| **Hybrid: native measurer + report adapters + pure rules in the engine (chosen)** | A native crate measures (tree-sitter) and imports reports; `kiro-core` judges, ratchets, attributes and renders over plain data | Works with no external tool for the bundled languages; full Understand fidelity when the user supplies scitools-hook JSON; the pure half is wasm-safe for a later plugin readout; mirrors scitools-hook's own worker/analysis split | Two code paths for facts (native, imported) need one fact model; tree-sitter grammars add native build weight | Matches steering "engine first, frontends thin" |
| Adapters only | Import scitools-hook / jscpd / lizard reports, no native measurement | Smallest code; twelve languages via Understand | Licence-bound for the complexity gate; no tool exists for scattered state, positional access or families; nothing works offline on a fresh clone | Rejected as the only path; kept as the adapter half |
| Fully native measurer | tree-sitter for every rule and every language | No external dependency | Re-implements what Understand already measures for twelve languages; `Essential`, `CountPath`, coupling, cohesion need resolution the measurer lacks | Rejected; native stays bounded to what a parser can take |
| ast-grep rules as the whole rule engine | YAML rules executed by ast-grep, results imported | Rules are data; fast to add patterns | No metrics, no ratchet, no families; a second configuration language for the operator | Rejected as the engine; evaluated as a library for Req 7/8 and not adopted (see decisions) |
| Measurement inside `kiro-cli` | tree-sitter directly in the CLI crate | One crate fewer | Mixes a parser runtime into the adapter crate; untestable without the CLI; blocks a later second frontend | Rejected; a separate native crate |
| Plugin-first (IDE readout) | Show code health per spec in OpenIDEA | Human readout where the specs are read | The sandbox cannot read source trees or run anything; nothing to show until a report exists | Deferred; the pure half prepares the shapes |

## Design Decisions

### Decision: Measurement never judges; judgement never measures
- **Context**: scitools-hook keeps thresholds out of the worker so the policy is unit-testable without a licence; here the equivalent is keeping policy out of the native crate so it is wasm-safe and fixture-testable.
- **Alternatives Considered**: 1. Rules inside the measurer crate; 2. One fact model, rules in `kiro-core` (chosen).
- **Selected Approach**: A `CodeFacts` document (entities with metrics and keys, token index, bindings, comparisons, data accesses, clones, imported breaches, parse failures, provenance) produced natively or by an adapter; `kiro_core::code::evaluate` turns facts, configuration and spec views into a `CodeReport`.
- **Rationale**: One place for every verdict; both fact producers tested against the same fixtures; the plugin can later consume a committed facts or report document.
- **Trade-offs**: A facts document is a second schema to version. Accepted; it is also the baseline's content.
- **Follow-up**: Task 1 fixes the facts schema, its sorted JSON form and the digest.

### Decision: The baseline is a facts document, not a findings list
- **Context**: Requirement 9.4 needs a before side without git; 9.8 needs determinism.
- **Alternatives Considered**: 1. Store before findings; 2. Store before facts (chosen).
- **Selected Approach**: `code baseline` writes `to_json_sorted(CodeFacts)` with `config_digest`; the ratchet compares entity metrics, so limits can change without recapturing, and `CODE_BASELINE_STALE` says when the configuration moved.
- **Rationale**: Findings depend on limits; facts do not. A facts baseline survives a configuration change and lets a later rule run over old facts.

### Decision: Gestalt ratio and FNV-1a hashing, both dependency-free
- **Context**: The family threshold was calibrated against Python's `SequenceMatcher`; `kiro-core` may not grow dependencies.
- **Selected Approach**: `families::ratio` is Ratcliff/Obershelp without autojunk over `&[u32]`; `facts::hash64` is FNV-1a 64-bit used for line hashes and the configuration digest.
- **Trade-offs**: FNV-1a is not collision-resistant against an adversary; the gate compares code a team controls, and a false duplicate needs twelve aligned collisions. Documented.

### Decision: tree-sitter queries, not `ast-grep-core`, for the pattern rules
- **Context**: `ast-grep-core` 0.51.1 would give YAML-style patterns with metavariables.
- **Selected Approach**: Shipped patterns are tree-sitter S-expression queries compiled once per run; configured patterns use the same language.
- **Rationale**: tree-sitter is already the measurer's dependency; `ast-grep-core` adds its own grammar bundle and a second pattern language the operator would have to learn; the shipped patterns are few and fixed.
- **Trade-offs**: S-expression queries are less friendly to write than ast-grep patterns; the configuration documents each shipped query as an example.

### Decision: `git` as a subprocess for `--changed`
- **Context**: Requirement 9.4; see the research entry.
- **Selected Approach**: `git diff --name-status -z <rev>` and `git cat-file --batch`, two processes per run, in `kiro-code::git`.
- **Rationale**: No checkout, no temporary tree; `gix` rejected as a heavy dependency for a feature that presumes git anyway.

### Decision: `CODE_IMPORTED` and the `fix:` tag
- **Context**: Requirement 1.3 asks that a breach imported without a value be reported as an imported, unratcheted finding; requirement 12.2 asks that every finding's hint open with a tag, including findings about the run itself.
- **Selected Approach**: One code `CODE_IMPORTED` (warning) for breach-only imports carrying the source's hint; a seventh tag `fix:` for `CODE_PARSE_FAILED`, `CODE_SOURCE_INVALID`, `CODE_CONFIG_INVALID`, `CODE_BASELINE_STALE`, `CODE_SOURCE_DISAGREES` and `CODE_COVERAGE`; `CODE_FILE_UNOWNED` takes `collect:`. Requirement 12.2 was amended on 2026-11-09 to name `fix:`; the amendment is flagged for the approver.
- **Trade-offs**: One more tag than scitools-hook's three-plus-extensions; the alternative (operational findings without tags) would have contradicted 12.2.

### Decision: Defaults transfer only with their measurement
- **Context**: scitools-hook's rule is "every default has a limit; not every default has a ratchet; no blocking default without a measurement".
- **Selected Approach**: Routine and container defaults copy scitools-hook's measured numbers where the metric definition is identical; `file.CountDeclClass` ships at 8 as a warning until measured on Rust and TypeScript; duplication rules ship as warnings; the three new rules ship as warnings; `CODE_SCATTERED_SCHEMA` and `CODE_DEFINITION_AMBIGUOUS` ship at info.
- **Follow-up**: The implementation measures this repository and `../openidea` and records the inventory (Req 14.1).

### Decision: Scopes and ignores carry reasons; agents never edit them
- **Context**: scitools-hook's skills refuse to touch configuration; its acknowledgement mechanism requires a written reason.
- **Selected Approach**: `reason` is mandatory on every relaxation (Req 2.4); the skills are amended so the implementer loop runs `code check --task` and fixes code (Req 12.4–12.5); adaptation is a human-reviewed configuration change (Req 12.6).
- **Trade-offs**: More friction for a legitimate exception. Accepted: the friction is the feature.

### Decision: Steering, not AGENTS.md, carries the rules block
- **Context**: scitools-hook writes into `AGENTS.md`; cc-sdd loads `.kiro/steering/` as project memory on every run.
- **Selected Approach**: `code rules --write` targets `.kiro/steering/code-rules.md` with begin/end markers; the block is regenerated from the effective configuration.
- **Rationale**: The agent reads steering before every spec or implementation step by construction.

### Decision: The breadth ledger is checked by the engine, not kept by convention
- **Context**: QOpenBB keeps its ledger honest by steering rules; this project has a parser for every criterion and task already.
- **Alternatives Considered**: 1. Adopt the file and the rules as documentation only; 2. A pure `kiro_core::breadth` module with seven codes and a `breadth check` command (chosen).
- **Selected Approach**: Parse, resolve references against `RequirementsDoc`s, test path existence through `Source::read`, status rules, staleness against `TasksDoc`s, per-spec unmapped criteria, a coverage summary; the CLI only supplies the read port and the spec views.
- **Rationale**: Every integrity rule QOpenBB states in prose becomes a finding with a line; the plugin can later read the same ledger; the schema stays compatible with QOpenBB's so a ledger can be read across the two projects.
- **Trade-offs**: One more committed file to keep current; the skills carry that duty (A1.8).

### Decision: End-to-end journeys are the acceptance; the ledger requires them
- **Context**: A2 and A1 together.
- **Selected Approach**: One e2e test binary with a harness over the real `kiro-cli` and real git repositories, seven journeys mapped to the requirements' user stories, an evidence line per step, run in the Dagger gate; `passed` requires an `e2e_paths` entry (status rule) so neither amendment can be satisfied without the other.
- **Trade-offs**: Longer CI; mitigated by running e2e last and keeping the journeys to seven.

### Decision: One parameter library crate, migration by aliases
- **Context**: A3; the existing commands already drift.
- **Alternatives Considered**: 1. A module inside `kiro-cli`; 2. A separate crate `kiro-cli-common` (chosen); 3. Adopting `clio` for file arguments.
- **Selected Approach**: A library crate with `GlobalArgs`, `OutputArgs`, `GateArgs`, the exit model and completions, built on `clap` 4.5, `clap-verbosity-flag` 3.0.4 and `clap_complete` 4.6.11; every command flattens the groups; `--root`, `--table` and `--out` remain as hidden aliases; a conformance test over `--help` and `docs/cli-standard.md` keep binaries on the standard.
- **Rationale**: A crate is what a second binary can depend on; aliases make the migration invisible to users and to the existing CLI tests; `clio` was not adopted because one writer covers the single `--output` convention.
- **Trade-offs**: `Exit`/`Failure` move crates (re-exported for compatibility); a small amount of churn in the existing command modules, guarded by the existing byte-equality tests.

### Synthesis outcomes
- **Generalisation**: one collect → group → report-against-selection component (`RuleScattered`) for definitions, state and schema; one parameter library every command composes; one ledger that both the spec gate's specs and the code gate's tests feed; one `Judgement` → `Ratchet` path for every limit so blocking is decided in one place; one `LanguageAdapter` trait so a fourth language is one file; one selection model (`--all`, `--changed`, `--spec`, `--task`, `--files`) shared by every rule; one `Imported` shape for both adapters.
- **Build vs adopt**: adopt tree-sitter and the four grammar crates, `serde_json`, the existing `toml` and `clap`, `FsSource`/`Exit`/`findings_text`, `git`; port the two duplication algorithms; build the fact model, configuration, ratchet, the three new rules, attribution, renderers, SARIF writer and the steering block. Rejected: `ast-grep-core` (second pattern language), `gix` (dependency weight), `rust-code-analysis` (maintenance unconfirmed), `sha2` (unnecessary in a pure crate).
- **Simplification**: no adaptive baseline, no `recommend`, no `doctor`, no stats prefixes, no SARIF companions, no colour, no plugin command yet; exit codes stay `0/1/2`; the measurer walks the whole tree on every run rather than maintaining an incremental cache (the measured budget allows it; a cache is a later optimisation if it does not).

## Risks & Mitigations
- Tree-sitter ABI or node-kind drift between the core crate and the grammars — exact pins; a parse-and-snapshot fixture per language in task 1 before any adapter work; kind names isolated in one adapter file per language.
- Noise from the three new rules on real code — ship as warnings; measure on this repository and `../openidea` before any default blocks; name-based ignore lists with reasons.
- scitools-hook JSON is a partial measurement — `FileFacts.measured_by` and `CODE_COVERAGE` say which files the source touched; the design never claims a census from it.
- Spec size (14 requirements, ~40 components) exceeds one implementation wave — tasks will be grouped into waves (engine model and catalogue → measurer and adapters → rules → ratchet and attribution → CLI and renderers → self-gate, docs, skills); each wave's boundary is one `_Boundary:_` set.
- Performance of the family pass on large trees — the exact bounds; release-only timing tests; `--task` and `--changed` selections bound the judged set while the index stays project-wide.
- Attribution depends on File Structure Plans being current — `CODE_FILE_UNOWNED` makes drift visible; no finding is dropped for lack of an owner.
- Operators confuse "no finding" with "checked" — `CODE_COVERAGE` on every run; parse failures block in the selection.
- Windows path handling in the walker and in git output — all paths normalised to `/` at the boundary; tests run on Windows (development) and Linux (CI).
- End-to-end suite runtime in CI — seven journeys over small fixture projects; run last in the Dagger gate; the harness reuses one build of the binary.
- Alias migration regressions in existing commands — the existing CLI tests assert byte-identical output; the conformance test asserts the aliases are still accepted.
- Ledger drift between QOpenBB's shape and this project's — the schema is a superset (`e2e_paths` only); references accept both spellings; a ledger written for QOpenBB parses here.

## References
- `D:\Source\scitools-hook\docs\reference\rules.md` — rule catalogue, defaults, "What blocks", "Scattered definitions", lean rules and their blind spots.
- `D:\Source\scitools-hook\docs\reference\features.md` — every feature with its on/off default.
- `D:\Source\scitools-hook\docs\compare\tools.md`, `linters.md` — CodeQL/Semgrep, aggregate tools, slopo, ruff/mypy boundaries.
- `D:\Source\scitools-hook\docs\guide\lean-code.md`, `agents.md`, `developer-guide.md` — the ladder, tags, agent-rules block, skills, before/after examples.
- `D:\Source\scitools-hook\docs\argument\{ratchet,working-set,economics}.md` — why per-entity, why commit-time, why only regression blocks.
- `D:\Source\scitools-hook\src\scitools_hook\analysis\lean\{similar,duplicates}.py`, `analysis\structure\definitions.py`, `understand\worker_lean.py`, `models\{snapshot,findings}.py`, `report\json_out.py` — algorithms, bounds, `EntityKey`, the JSON report.
- `D:\Source\scitools-hook\.kiro\specs\{maintainability-gate,lean-code-rules,understand-8-features}\requirements.md` — the upstream requirement texts the catalogue derives from.
- [jscpd](https://npmjs.com/package/jscpd) · [jscpd 5.3.0 README](https://unpkg.com/jscpd@5.3.0/README.md) · [jscpd-rs report/json.rs](https://docs.rs/crate/jscpd-rs/0.1.5/source/src/report/json.rs) · [jscpd-rs detector/model.rs](https://docs.rs/crate/jscpd-rs/0.1.7/source/src/detector/model.rs) — report shapes.
- [lizard on PyPI](https://pypi.org/project/lizard) — languages, `-Eduplicate`.
- [rust-code-analysis](https://mozilla.github.io/rust-code-analysis) · [docs.rs](https://docs.rs/crate/rust-code-analysis/0.0.18/source/README.md) · [big-code-analysis fork](https://docs.rs/crate/big-code-analysis/latest) — metrics, maintenance status.
- [tree-sitter 0.26.8 binding README](https://docs.rs/crate/tree-sitter/0.26.8/source/binding_rust/README.md) · [tree-sitter on lib.rs](https://lib.rs/crates/tree-sitter) — binding API; versions confirmed with `cargo search` on 2026-11-09.
- [slopo](https://github.com/rafal-qa/slopo) — embedding-based clone finder, AGPL-3.0.
- [ast-grep rule reference](https://cdn.jsdelivr.net/npm/@d-trattner/pidex@0.3.0/skills/ast-grep/references/rule_reference.md) · [ast-grep man page](https://linuxcommandlibrary.com/man/ast-grep) — YAML rules, `--json`.
- [clippy `too_many_lines_threshold` default](https://doc.rust-lang.org/nightly/nightly-rustc/clippy_config/conf/defaults/fn.too_many_lines_threshold.html) — configuration keys.
- [complexipy vs ruff](https://rohaquinlop.github.io/complexipy/comparison-with-ruff) — `C901`/`PLR0912` semantics.
- [pylint `similar.py`](https://github-redirect.dependabot.com/pylint-dev/pylint/blob/1e05190ef09f6bdb3d4ce8573e2548853df30cdf/pylint/checkers/similar.py) · [pylint issue 4970](https://hub.harborframework.com/tasks/swe-bench/pylint-dev__pylint-4970) — R0801 mechanics.
- [PMD CPD user docs (mirror)](https://demo.gitea.com/phoedos/pmd/raw/commit/1044ce1860ff71dda2ef6ddc8125f431ca0eb6d2/pmd_userdocs_cpd.html) — `--minimum-tokens`, formats.
- `.kiro/steering/roadmap.md` (amendments 2026-11-04 and 2026-11-09), `.kiro/steering/agent-guidance.md`, `.kiro/specs/{cli-gate,spec-constraints,vocabulary-grounding,kiro-core}/design.md` — consumed contracts.
- `D:\Source\QOpenBB\.kiro\breadth.toml`, `AGENTS.md`, `.kiro\steering\{testing,delivery-workflow}.md`, `.kiro\spec-ownership.toml`, `.kiro\complexity-budgets.json`, `docs\evidence.md`, `pipeline\qopenbb_ci\{cli,quality_cli,schemas}.py` — the ledger shape, the verification policy and the CLI conventions behind amendments A1–A3.
- `cargo search` 2026-11-09 — `clap-verbosity-flag 3.0.4`, `clap_complete 4.6.11`, `clio 0.3.5` (not adopted), `clap-cargo 0.19.0` (not adopted).

---

## Review: reuse, fidelity, decomposition and acceptance evidence (2026-11-09)

### Assessment and evidence scope

**NO-GO for implementing the entire current task plan unchanged; proceed with bounded prototypes and a design revision.** The pure policy core, native measurement boundary, existing ownership join, explicit coverage signals and reason-bearing configuration are sound foundations. The main risks are measurement semantics, late integration and unsupported acceptance claims, rather than Rust feasibility.

This is a review, not approval or implementation. Requirements, design, tasks and approval flags were not changed. Source inspection used these repositories and their current working trees:

- openidea-kiro HEAD `cebeebd338782e2bfa07ab64b5ac1608fefd7484`; the reviewed spec is untracked and the roadmap already modified.
- scitools-hook HEAD `73a736e707912ae2c5e29503be42a632ced8dd17`; clean at inspection.
- QOpenBB HEAD `b37ec1963e1bc138115b431e3c168e5e48a8bd47`; unrelated working-tree changes were present and left untouched.

Fresh checks: `cargo run --locked -p kiro-cli -- check --project . --feature code-constraints --fail-on warning` exited 0: 108 criteria, 0 errors, 0 warnings, 42 informational findings. It checked **zero annotated criteria and zero contradiction pairs**; this is syntax/shape evidence, not semantic validation. `kiro-cli status --project . code-constraints` reports 0/53 task nodes, 42 components and 80 mapped files. A separate dependency audit found 47 leaf tasks, 84 edges, no unknown dependencies and no cycle. `cli-gate` reports completed, 30/30, although its roadmap checkbox remains open. No code-constraint implementation, analyzer benchmark, sibling test suite or new E2E suite was executed for this review. Cargo artifacts used the configured `D:\BuildArtifacts` locations.

### Three critical concerns

1. **Measurement and ratchet fidelity are insufficiently specified (high risk).** `design.md:977` imports a sparse findings report, while `design.md:1248` converts even numeric upstream ratchet findings to unratcheted warnings. The ratchet flow at `design.md:390` lacks scitools-hook's decomposition, invalid-before and floor safeguards. These gaps can both miss regression and reject beneficial changes. Preserve explicit metric semantics and completeness, separate finding import from measurement import, and pin differential fixtures before accepting a baseline. Traceability: 1.2–1.5, 3.1–3.5, 9.1–9.9, 11.1–11.4.
2. **The delivery unit is too broad and tests integration too late (high risk).** Acyclic tasks are not independent deliverables. Task 4.5 waits for all rule families and source adapters; both harnesses wait on 4.8, and E2E work begins at 5.6. A3 also changes every existing CLI command. Move the first real-binary journey to the foundation and split at the contracts below. Resolve the concrete `vocab import --format` collision before migrating commands. Traceability: 13.1–13.5, 14.1–14.5, A2, A3.
3. **The ledger can accept a claim that was never tested (high risk).** `design.md:882–888` checks file existence and nonempty evidence strings, not test collection, execution or source identity. A2.5 allows visible skips, while A1.4 can still accept `passed`; A1.9 requires seeding at task generation, but task 6.1 postpones it. Add structured execution evidence and a separate acceptance gate. Traceability: A1.3–A1.9, A2.1–A2.5.

### A. Reuse before building

Prefer a hybrid: reuse measurement/detection machinery where its semantics fit, and retain Kiro-specific ownership, vocabulary, policy and evidence logic. The earlier rejection of adapters as necessarily licence-bound is too broad: open-source analyzers can supply measurements. Offline operation after installation is possible; shipping a self-contained Rust binary is a separate packaging decision.

| Area | Candidate and verified capability | Recommendation and prototype question |
|---|---|---|
| Routine metrics | [rust-code-analysis](https://github.com/mozilla/rust-code-analysis) exposes a Rust library and metric-export CLI based on tree-sitter. | Prototype as a native dependency or offline report producer. Compare metric definitions, grammar coverage, dependency size, licence policy and supported toolchain. The existing note “maintenance unconfirmed” is an unresolved question, not evidence that rewriting is cheaper. Keep it outside the WASM core. |
| Independent metric oracle / alternate source | [Lizard](https://github.com/terryyin/lizard) covers Rust, Python and TS/JS, exposes function NLOC, CCN, token/parameter counts and nesting extensions. Its documentation acknowledges partial-parser limitations. | Use a pinned version for a comparison corpus or an optional adapter. Its CCN is not automatically Understand `CyclomaticStrict`; do not silently share thresholds or before/after values across tools. It does not replace every required fact. |
| Clone detection | [jscpd Rust crates](https://jscpd.dev/reference/rust-crates) expose `cpd-core`, `cpd-tokenizer` and `cpd-finder`; core provides detection algorithms and models without I/O. | Evaluate library reuse before porting a clone engine; retain the JSON adapter for external runs. Prefer a narrow engine interface over the whole finder when Kiro must own confined reads. Verify pinned releases, dependency policy and exact clone boundaries. |
| Similar functions | [jscpd similarity](https://jscpd.dev/guides/similarity) documents AST-based function similarity, including renamed/edited copies. | A serious candidate, but not scitools parity: AST fingerprints, retained called names, exclusions and scores differ from token-shape `SequenceMatcher`. Test pair detection, then add Kiro family grouping only if it preserves the intended behavior. The similarity page describes 5.4.1 while the crate page illustrates 5.4.0; pin and test an actual release before relying on either API. |
| Clone debt | [jscpd baseline](https://jscpd.dev/guides/baseline) supports committed fingerprints and new-clone gating. | Reuse the established semantics or import its result if new-clone gating is desired. Do not build a second clone baseline implicitly inside the metric ratchet. |
| Data-access patterns | [ast-grep](https://ast-grep.github.io/guide/introduction.html) supplies structural matching and a rule-testing workflow. | Compare a few rules against direct tree-sitter queries. Either can extract facts; cross-file aggregation and vocabulary resolution remain Kiro logic. A pattern authoring tool does not require exposing a second configuration language to users. Keep direct queries if the prototype shows they are simpler. |
| CLI and testing | Existing clap configuration and `crates/kiro-cli/tests/common/mod.rs:80–101` already provide fixture setup and real-binary execution. | Extract shared argument groups using clap and extend the current test helper. Avoid separate, substantially identical harnesses in tasks 5.1 and 5.6. Keep existing insta/nextest infrastructure; one canonical runner should partition component and journey tests. |

Keep custom: capability-to-criterion validation, spec/component/task attribution, vocabulary-backed data names, policy decisions over typed facts, source-completeness reporting and evidence validation. Do not replace every selected dependency or reopen established parser choices without a measured benefit.

### B. What scitools-hook actually demonstrates

| Technique / assumption | Inspection result | Required disposition |
|---|---|---|
| Measure separately from judging | Matches the worker/snapshot versus analysis separation. | Preserve this architectural seam. |
| Lexical similarity, not semantic equivalence | `analysis/lean/similar.py:811–820` uses `SequenceMatcher(..., autojunk=False)`. | Preserve the honest limit. Embeddings may support exploration, but neither detector proves behavioral equivalence. |
| Exact pruning and affected-family traversal | Present, including canonical pair ordering because scoring is asymmetric (`similar.py:675–700`). | Port oracle fixtures for ties, argument order, repetition and threshold edges. |
| Nested-routine exclusion | More than rejecting a parent/child edge: `similar.py:703–726` removes nested members from the family and recomputes reachability. | Add the third-member bridge case; edge-only exclusion is insufficient. |
| Signature pairing | `analysis/ratchet.py:74–90` allows pairing to add ratchet findings, but deliberately does not use a guessed pairing to excuse a threshold breach as pre-existing. | Make exact and heuristic matches separate result types and test both classifications. Current `pair()` plus one flow leaves this ambiguous. |
| Refactoring-safe ratchet | `ratchet.py:33–61,108–134` exempts certain count growth when control-flow complexity improves, rejects before-side parse failures, and respects floors on both sides. | Add these explicit decisions or document and approve a deliberate divergence. Guard-clause transformation must not be blocked solely for the count growth it causes. An absolute cap can still apply. |
| Full-fidelity imported measurements | **Not supplied by RunResult.** `models/findings.py:279–335` holds findings, highest values, metadata and counts, not all entities/metrics. Missing records do not mean zero, deleted, new or unchanged. | Treat it as findings plus partial observations. Require a complete, versioned measurement export for general imported ratcheting and net totals, or mark those operations unavailable. Preserve source severity/blocking context and state any local policy override. |
| Stable comparisons | Source/version/metric semantics can change independently of configuration. The current baseline only checks its configuration digest. | Bind facts to producer, producer version, grammar/metric definition version, input scope and completeness. Compare like with like; report incompatibility rather than computing a numeric regression across producers. |
| Change selection and deletions | `design.md:1003` reads deleted blobs, but `1004` excludes deleted paths from selection; net totals use selected files (`787`). | Separate output selection from accounting scope. Keep deleted/old rename paths for before-side subtraction and map renames explicitly. Test untracked files and define whether `--changed` includes them. |
| Complete feature map | scitools lists `structure.call_cycle` and `structure.reachable_complexity` in `docs/reference/rules.md:111–112`; the current map's deferred list omits both. | Add both explicitly to the future structure spec. Absence from the current implementation is reasonable; absence from the vision map is not. |
| Analysis trust | Understand lifecycle and licensing are tool-specific; the principle that missing/poor analysis cannot justify a clean result is general. | Carry measured/unavailable/failed/not-applicable distinctions into the fact model and later dead-code/reference rules. Do not discard this principle with Understand-specific accuracy fields. |

The earlier map is directionally useful but does not establish vision completeness. Maintain a versioned inventory of all upstream rule IDs and techniques, with one of implemented, adapted, deliberately different, deferred-with-owner, or not-applicable-with-reason. Also retain the original Kiro roadmap's deferred intentions/precedence, envelope comparison, production-code separation and plugin readout. A CLI journey cannot accept a future IDE capability.

### C. Divide-and-conquer delivery strategy

Risk is **high for the current all-at-once plan**, and plausibly **medium after the contract and prototype gates below**. These are qualitative engineering judgments, not measured failure probabilities. The full scope is XL under the gap-analysis rubric; a reliable calendar estimate requires the prototype results.

Retain the current spec as an umbrella/vision record and partition acceptance criteria without losing their original IDs:

| Proposed spec | Owns | Dependency / first accepted slice |
|---|---|---|
| `cli-contracts` | A3: common parameter library and compatibility migration | Existing cli-gate; one old command migrated with unchanged invocation and output behavior. |
| `verification-evidence` | A1 and reusable A2 harness/evidence contract | Existing core parsers/CLI; a ledger plus one real-binary journey whose invalid claims fail. No dependency on implementing code analysis. |
| `code-constraints` (narrowed) | Facts/provenance, configuration, basic metrics, ratchet, attribution, coverage, baseline/git, scitools import and core CLI | Stable contracts; Rust task-scoped fail/fix/pass first, then Python and TS/JS as explicit acceptance milestones. |
| `clone-constraints` | Requirement 5, jscpd import, duplicate/family behavior and bounded performance | Facts contract; independently runnable clone journey. Reuse prototype decides detector. |
| `code-locality-data` | Requirements 6–8: scattered definitions/state/schema and role-named access | Facts, vocabulary and attribution; curated positive/negative examples before warnings become blockers. |

Keep the already deferred `structure-constraints` and `lean-code` separate. Defer SARIF/plugin presentation if needed through an explicit scope amendment, not an unchecked task quietly omitted. These are proposed boundaries; moving approved criteria requires normal requirements/design/tasks review.

Shared contracts need one owner: `CodeFacts` and provenance, result/exit envelope, ownership join and execution-evidence schema. Consumer specs use versioned fixture contracts. Record each criterion's owning spec and each shared file's owner; avoid duplicating rule catalogues, defaults or metric definitions across specs. Parallel implementation becomes safe after these seams stabilize, not merely because tasks carry `(P)`.

**A3 has a concrete compatibility conflict:** the current binary uses `vocab import --format auto-ontology-json|skos-turtle` for input (`crates/kiro-cli/src/cli.rs:279–286`, confirmed with `--help`). The proposed shared `--format text|json` cannot occupy that same command slot unchanged. Specify command capabilities and a compatibility exception: introduce an unambiguous input-format name while preserving old invocations, and separate report format where needed. Likewise define `--table` relative to the current graph/task rendering; an enum limited to Text/Json cannot express “alias for table.” Conformance must verify applicable groups and actual behavior, not demand meaningless gate flags on every command.

| Gate | Timing | Required evidence / stop condition |
|---|---|---|
| G0: contract and scope | Before broad implementation | One owner per criterion and shared contract; immutable sample fact/report/evidence documents; sparse input cannot masquerade as a census; CLI flag conflicts resolved; planned ledger seeded. Stop on unresolved contradictory contracts. |
| G1: reuse and metric prototype | Before custom measurer/clone commitments | Pinned tool versions against a hand-counted corpus for each language: boolean operators, match/exception branches, nested functions, lambdas, parameters, comments and malformed syntax. Every discrepancy is explained and mapped or marked unavailable. Verify clean builds on Windows and Linux, plus no native dependency leaking into core's WASM build. Choose reuse or custom work from these results. |
| G2: first vertical slice | Before implementing every rule | Actual binary on a temporary Git/spec project: task-owned routine fails, fix passes, baseline debt passes, regression fails; assert finding, ownership, exit and coverage. A deliberately disabled detector must make the journey fail. |
| G3: ratchet and source integrity | Before baseline adoption | Regression fixtures from scitools: signature change, decomposition, invalid before parse, metric floor, missing metric, source-version change, sparse/empty source, rename and delete. Only justified comparable measurements influence the verdict; net totals are exact or explicitly unavailable. |
| G4: detector quality and scale | Before enabling clone/new-rule gates | Known positives and negatives, nested-family bridge case, unrelated same-name variables, legitimate positional indexing and computed access blind spots. Record precision/recall on the labeled corpus and an owner-approved noise budget. Exercise small selections in large trees and adversarial repeated shapes. Meet the stated release budgets (task <10s, whole tree <60s) on a recorded machine/corpus; bounded work must report incomplete analysis rather than silently pass. |
| G5: evidence integrity | Before any capability becomes passed | Exact test IDs collected and executed; zero required skips, failures or missing tests; source/build/contract identity matches independently computed expectations. Stale record, nonexistent test selector, empty suite and exit-zero skip must each fail the acceptance gate. |
| G6: integration and compatibility | At each spec join | Canonical runner exercises old CLI commands plus new journeys, output/alias behavior, Windows/Linux paths and line endings, required-tool failure, source containment, deterministic reports, core WASM and standalone plugin checks. Run expensive common checks once per joined state. |
| G7: rollout | Before making warnings blocking | Advisory run on this repo and representative sibling corpora; inspect findings and record false positives/unsupported cases. Human-reviewed baseline/configuration, then blocking only for accepted rules. Keep prior executable/configuration and a documented human rollback path. Never let the implementing agent refresh the baseline to hide failures. |

Move the reusable harness out of tasks 5.1/5.6 into G0/G2. Each feature adds its journey with its implementation. Keep final integration testing, but do not defer observable behavior until the last phase. Reconcile the stale cli-gate roadmap checkbox and the nextest requirement versus `.dagger/src/kiro/main.py:42` currently invoking `cargo test --workspace` when revising the canonical runner.

### D. Breadth ledger and end-to-end validity

The filename is **`.kiro/breadth.toml`**. QOpenBB has it (200 capability entries); this repository currently does not. A1/A2 clearly specify the workflow, but the implementation and ledger are still pending. Seed planned capabilities during the next task-generation revision, as A1.9 already requires; do not mark them implemented or passed merely because this review exists.

QOpenBB's useful precedent goes beyond TOML. `.kiro/steering/testing.md:37–45` requires behavioral assertions, real test entry points and no acceptance from empty/skipped suites. `tests/contracts/test_evidence.py:32–72,111–115` tests rejection of exit-zero nonpassing results, irrelevant requirements, stale revision/lock/contract identity, and an expected context independent of the submitted evidence. Those are inspected test contracts, not claims that the sibling suite passed during this review.

Recommended minimal design:

1. **Separate the plan from executions.** Keep human-owned capability/criterion/test mappings in TOML. Store structured run records separately: capability and criterion IDs, exact collected test IDs, command and working directory, result/counts, revision plus dirty-input or source-manifest digest, binary/toolchain identity, lock/config/spec/test digests, platform, fixture/input identity and artifact digests. Scope identity to relevant inputs so unrelated changes do not invalidate all evidence. Historical passes remain historical; the current acceptance result is computed.
2. **Two checks with different claims.** `breadth check` validates references and mapping. An acceptance verifier consumes runner output and independently computed expected identity. Existence of `file.rs::test_name` must not count as proof that the function exists or ran. Release acceptance requires all in-scope criteria mapped and their required journeys current/passing; leave unmapped criteria informational only during planning. A legitimate excluded/deferred criterion needs an owner and reason.
3. **Reject vacuous success.** Required tests must be discovered, executed and passed, with zero missing/failed/skipped tests. Missing Git or another required tool is an incomplete acceptance run, even if the local runner explains the skip. Optional matrix cases may remain unverified under an explicit policy; they must not raise the whole capability to passed. A multi-task capability stays partial until all its obligations are met.
4. **Keep the evidence reproducible.** A stdout hash and command cannot reconstruct missing fixture state. Preserve fixture identity, Git setup, relevant environment and replay instructions; retain stdout/stderr and written-file assertions for failures. Capture structured output for successful runs too, because test harnesses commonly hide passing-test stdout. Use one portable runner locally and in Dagger, with test selection ensuring the journey suite runs once.
5. **Test the checker against false claims.** Mutate criterion IDs, test selectors, source/config hashes, expected counts and result status; simulate a skipped Git journey and a disabled detector. All must prevent acceptance. Verify an actual fresh run repairs the state. Keep the evidence-producing job and expected source identity outside the ledger's self-declared claims.
6. **Avoid self-invalidating records.** Bind a run to the tested source tree or relevant content manifest; attach the result through CI or a later evidence commit without pretending that later commit was tested. Ignore the evidence artifact's own bytes when computing the tested-input manifest. Human baseline/configuration review remains separate from automatic status calculation.

The plan is feasible with these changes. The next design revision should settle the metric/fact and evidence contracts, adopt the five delivery boundaries (or equivalent independent slices), and place G0–G3 ahead of broad implementation. Current approval flags should remain pending until that revision is reviewed.


---

## Design revision: bounded deliverables (2026-11-09)

### Summary

Owns comparable measurements, routine and container policy, the refactoring-safe ratchet, ownership attribution and the code CLI. Clone detection belongs to clone-constraints; locality and vocabulary-backed data rules belong to code-locality-data; CLI conventions and execution evidence are consumed contracts.

The user instructed applying the preceding review and proceeding to designs. Requirements are approved for this design step on that basis; design approval and implementation remain pending. The original 108 criteria have explicit successors in split-map.md; added safeguards are new numeric criteria.

### Discovery and consulted guidance

Integration-focused discovery reuses the preceding repository/tool review. Consulted kiro-spec-requirements (EARS and requirements-review-gate), kiro-spec-design (design-principles, light discovery, synthesis and design-review-gate), core product/tech/structure steering and foreground workflow. Review performed inline; no subagent or model-specific implementation request was invoked.

- ../scitools-hook/src/scitools_hook/analysis/ratchet.py:33–90,108–134; models/findings.py:279–335; docs/reference/rules.md:111–112.
- [rust-code-analysis](https://mozilla.github.io/rust-code-analysis/) supports native library/CLI use; [Lizard](https://github.com/terryyin/lizard) supplies independent measurements with documented parser limits.
- Source comparison in the retained review establishes the missing safeguards, not cross-provider metric equivalence.

### Synthesis and decisions

- Generalization: plain typed observations consumed by pure policies and the existing report/evidence contracts.
- Build versus adopt: reuse existing parsers, joins, fixture helpers and documented libraries before adding custom machinery; the first prototype pins dependencies and records per-behavior adoption or rejection before broad implementation.
- Simplification: own only this spec's behavior; no duplicated filesystem walks, evidence frameworks or downstream schema fields. Shared registry edits remain explicit and serialized.
- Alternatives: the old monolith delays observable integration; adapters alone cannot supply all Kiro policy. The selected split retains a common contract without coupling feature acceptance.

### Requirements and design review gate

Inline reviewer checked numeric criteria, EARS shape, criterion traceability, component-to-file mapping, boundary commitments, dependencies, first deliverable and negative/error paths. Local repair pass resolved output-destination conflicts, planned-versus-executed evidence, default required metric coverage and enum ambiguity. Mechanical checks and current spec-gate results are recorded in code-constraints/design-validation.md after finalization.

### Risks and validation hooks

Prototype results are not available and no external dependency has been added. G1 must fail visibly if the required semantics or packaging cannot be met. G2 provides a real-binary slice before broad integration; G3/G5 prohibit false regression/acceptance claims; G4/G6 establish corpus/performance/platform evidence. Current design review does not certify runtime readiness.

## G1 record: parse layer (2026-11-10)

Prototypes outside the repository: `D:\BuildArtifacts\openidea-kiro\g1-parse` (ast-grep) and `D:\BuildArtifacts\openidea-kiro\g1-rca` (rust-code-analysis). No lexer or parser was written; the prototype holds node-kind tables per language and counting rules over the library's tree.

| Question | Result |
|---|---|
| Candidate A | `ast-grep-core` `=0.51.1` with `ast-grep-language` `=0.51.1`, default features off, features `tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-typescript`, `tree-sitter-javascript` |
| Resolved parse stack | one `tree-sitter` 0.27.2 with `tree-sitter-rust` 0.24.2, `tree-sitter-python` 0.25.0, `tree-sitter-typescript` 0.23.2, `tree-sitter-javascript` 0.25.0 |
| Hand-counted corpus | 7 routines in Rust, Python and TypeScript; parameters, strict cyclomatic complexity, nesting, code lines and statements all equal the hand counts (35 of 35 values) |
| Constructs covered | receiver exclusion (`self`, `&self`), boolean operators (`&&`, `\|\|`, `and`, `or`, `??`), else-if and elif chains without extra nesting, match arms, switch cases, ternaries, Python docstring and comments excluded from code lines, tail expressions, default and splat parameters |
| Malformed input | reported as a parse error with its line; nothing is measured |
| Platforms | Windows 11 (rustc 1.98.1) and Linux x86_64 in a `rust:latest` container (rustc 1.99.1, fresh dependency resolution): identical results. macOS not run |
| Cost | 48 crates in the normal dependency tree, release binary 5.1 MB |
| Speed | all 72 Rust files of this repository (1.0 MB, 871 routines) in 0.70 s; `../lesscode/native/src` (49 files, 625 routines) in 0.58 s, unoptimized counting code |
| Candidate B | `rust-code-analysis` `=0.0.25` does not compile from crates.io: its dependency ranges resolve to `tree-sitter` 0.20.9 and 0.27.2 at once and the types no longer match. Rejected. The fork `rust-code-analysis-code-split` pins tree-sitter 0.26 and would add a second parse stack; not adopted |

Decision: adopt candidate A for `crates/kiro-code`. It is also what `../lesscode` pins (0.51.0), so the two tools share one parse stack. Statement counting needs a per-language rule (block children for Rust and Python, statement kinds for TypeScript); that is a profile table, not a parser.

First inventory of this repository with the candidate limits (not a quality score): 37 routines above cyclomatic 10, 30 above nesting 3, 61 above 60 code lines, 19 above 40 statements, 8 above 5 parameters. Highest: `read` in `kiro-cli/src/import/skos.rs` (cyclomatic 62, nesting 7, 242 lines) and `main` in `kiro-cli/src/main.rs` (cyclomatic 52, 357 lines).

Not covered by this prototype and still owed before the profiles are called calibrated: file and type metrics, the larger construct list of task 1.2 (closures, nested routines, macros, generics, decorators, arrow functions, classes), macOS, and the clone and locality libraries (`cpd-core`, `difflib-fast`, pattern queries), which are checked when their specs start.

## Implementation record: M0 and the first M1 slice (2026-11-10)

- `crates/kiro-code` (native): `walk.rs` reads a confined, sorted source bundle (excluded directories, 16 MiB bound, links leaving the root skipped and listed); `measure.rs` holds per-language node-kind tables for Rust, Python, TypeScript, TSX and JavaScript over `ast-grep-core` 0.51.1 and emits routine, type and file observations with producer, grammar and definition version 1.
- `crates/kiro-core/src/code/` (pure, wasm build unchanged): `facts.rs`, `config.rs` (built-in limits only), `limits.rs`, `attribution.rs` (the existing join; spec and task selection), `report.rs` (text and JSON), `mod.rs` (evaluation). Seven finding codes were added to the catalogue and its documentation.
- `kiro-cli code check --task|--spec|--files|--all [--show-highest]` composes the shared report and gate groups; profile `code check` is declared.
- Oracle: `crates/kiro-code/tests/provider_contract.rs` asserts hand-counted routine, file and type values for four fixture files, plus parse failure, unsupported language, line endings, ordinals and a no-panic run over generated text. The oracle caught one real defect (a Rust doc comment node includes its line break, which removed the following code line from the file count) and one error in my own hand count (the tail expressions in a closure's branch blocks are block items).
- Journey: `crates/kiro-cli/tests/e2e_code.rs` (11 tests): a task-owned routine with nesting 4 fails at `--fail-on warning`, is repaired by extraction, and the same command passes; task, spec and whole-tree selection; parse failure blocks and is not judged; unowned file; unmeasured file; usage failures name the choices; JSON is complete and byte-identical across runs; only the requested output is written.
- On this repository (release build): `code check --all` measures 144 files in 2.7 s and reports 384 warnings; a task-scoped check takes 0.2 s. The debug build needs 12.6 s for the whole tree; evaluation recomputes ownership per file and looks values up linearly, which should be indexed before the tree grows.
- `deny.toml`: the wasm dependency policy now excludes the native crate `kiro-code` from its graph, because its parse stack pulls `walkdir`, which is banned for the sandbox tree. `kiro-core` does not depend on it. Both policy checks pass. This is a policy-file change for the owner to review.
- Tasks marked complete: 1.1, 1.3, 2.1, 2.4, 3.1, 4.2. Partly done and left unchecked: 1.2 (corpus lacks decorators, async, lambdas, arrow functions, generics, macros, interfaces and enums in TypeScript), 1.4 (no calibration), 2.2 (no configuration file; built-in limits only), 2.3 (walk implemented, containment tests missing), 3.2 (no scoped limits or optional metrics), 3.3 (a required unavailable metric does not yet prevent a clean verdict), 4.1 (no extension composition), 4.3 (journey passes on Windows; Linux and macOS not yet).
- Every limit reports as a warning until calibration. Nothing is ratcheted: there is no before side yet.

## Platform record (2026-11-10, end of day)

The whole workspace test suite passes on Windows 11 (rustc 1.98.1) and on Linux x86_64 in a throwaway `rust:latest` container (rustc 1.99.1): 470 tests each, 0 failed. This includes the provider contract, the pure policy tests and the `code check` journey, so task 4.3 holds on Windows and Linux. macOS was not run.

The first Linux run failed one telemetry test and, because `cargo test` stops at the first failing suite, reported only 105 tests. The defect was real: `--telemetry-dir <project>/a/../b`, where `a` does not exist, was rejected with "failed to resolve" instead of "inside the project", since only Windows resolves such a path lexically. The resolver now walks the path component by component. The Linux script uses `--no-fail-fast` since then.

## Corpus extension and routine detection (2026-11-10, night)

- Task 1.2 complete: hand-counted fixtures now cover Python decorators, nested and async routines, splat and keyword-only parameters, lambda, `try`/`except`, `with`; TypeScript interface, enum, class fields, constructor, `for..of`, `try`/`catch`, inline callbacks, single-parameter arrows; and a JavaScript class. Every hand count matched on the first measured run. The optional Lizard cross-check was not done (no Python on this workstation).
- Decision: in TypeScript and JavaScript an arrow function or function expression bound to a name is a routine of that name; a class field holding one is a method, not a field. Without this most TypeScript code would not be measured as routines. Inline callbacks stay with the routine around them.
- A bare `*` or `/` in a Python parameter list is not counted as a parameter.
- Not in the corpus yet: Rust macros with control flow in their arguments, `impl` for a foreign type, Python `match`, TypeScript abstract classes, optional chaining, generators.
- `main.rs` of the CLI was split into one function per command; evaluation resolves ownership once per file.

## Implementation record: configuration and the ratchet, M2 (2026-11-10, night)

- **2.2 Configuration.** `.kiro/code-rules.toml` with limits and severities per metric, path scopes (most specific wins, later on a tie), ignores, and a required reason on every scope, ignore and relaxation; `CODE_CONFIG_INVALID` with every problem and its key, exit 2 before measuring; `code config` with origins, reasons and a digest. Parsed with the core's existing `toml` and serde; unknown keys are rejected by the deserializer.
- **5.1, 5.2 Pairing and decisions.** `ratchet.rs` pairs entities exactly (through Git renames), as moved, or by a changed signature. The decision corpus `crates/kiro-core/tests/code_ratchet.rs` was written by hand before the first run; all 14 decisions held. The first run failed five tests, all through positional lookups in the test, none through a decision.
- **Move rule** (review item C1): `kiro-code` records a fingerprint of every routine's code tokens; a routine that leaves a file and appears in another with the same name, parameters and fingerprint, one candidate each side, keeps its debt status. Journey: a split into a new file passes; an edited copy does not.
- **5.3 Baseline.** `code baseline` writes `.kiro/code-baseline.json` (facts schema 2, config digest, sorted, no timestamp, byte-identical on repeat); refuses when a file does not parse. A baseline of another schema version is not compared and the coverage line says why. A named revision wins over the baseline.
- **6.1 to 6.3 Git before side.** `crates/kiro-code/src/git.rs`: `rev-parse --verify`, `diff --name-status -z --find-renames --relative`, `show <rev>:./<path>`, separate arguments, read-only, an option-like revision refused. `git show` per file was used instead of `cat-file --batch`; simpler, and fast enough for a change set. Net statement and line deltas over changed, renamed and deleted files; the hand-counted values of two journeys (+1/+3 and -1/-6) matched.
- **7.1 Hook summary** (`--summary`): at most 25 lines of printable ASCII in a fixed order. The metric-points half of 7.1 (a `MetricProvider` for the code report with `kiro.code.*` names in the telemetry catalogue) is not done, so 7.1 stays unchecked.
- **7.3 Journeys** in `crates/kiro-cli/tests/e2e_ratchet.rs` over real Git repositories: existing debt does not block; a regression blocks at the default threshold although limits are warnings; a repair passes; without a before side the same change does not block (the journey depends on the ratchet); move, rename and deletion; unknown revision; baseline.
- **Not done in M2**: 7.2 (provider selection across several sources: there is one source), `CODE_NET_GROWTH` (no configured maximum yet), strict mode (pre-existing breaches blocking), `CODE_BASELINE_STALE` as a finding (the reason is in the coverage line instead).
- A hand count of mine was wrong once more: the summary test expected 0 warnings where three metrics had risen within their limits.
- Process slip: one commit (`d6b2557`) was made before the lint result was read and contained three lint errors in a test file; fixed in the next commit.

## Calibration record: distributions on the owner's corpus (2026-11-10)

Corpus named by the owner: this repository, `../mmhmem`, `../facdrone`, `../mvibe`, `../Contour`. Measured with `cargo run --release -p kiro-code --example distribution -- <root>...`; the repositories were read only. The full table is `D:\BuildArtifacts\openidea-kiro\calibration\dist.md`.

Files measured: openidea-kiro 141 (132 Rust), mmhmem 290 (234 Rust, 53 Python), facdrone 1875 (1874 Python), mvibe 181 (91 Python, 88 TypeScript), Contour 40 (29 Python, 11 Rust).

**Contour is not calibrated.** It is a C++ project: 1462 `.cpp` and 1165 `.hpp` files are tracked and no profile measures them. Only its 40 helper files in Python and Rust were read.

**The first run counted mmhmem twice.** The walk read 204 Rust files under `.claude/worktrees`, a Git worktree of the same project. The walk now skips a directory below the root that has its own `.git`; the Rust file count of mmhmem then equals what Git tracks (234). The numbers below are from the second run.

Share of routines above the limit, for the populations of at least 500 routines:

| Population | Routines | cyclomatic > 10 | nesting > 3 | code_lines > 60 | statements > 40 | parameters > 5 |
|---|---|---|---|---|---|---|
| facdrone, Python | 17184 | 0.1% | 0.1% | 0.5% | 0.1% | 0.3% |
| mmhmem, Rust | 3264 | 0.5% | 0.2% | 0.9% | 0.2% | 0.5% |
| mvibe, Python | 2072 | 2.1% | 1.6% | 0.4% | 0.9% | 0.8% |
| openidea-kiro, Rust | 1501 | 3.5% | 2.7% | 6.5% | 1.7% | 0.8% |
| mvibe, TypeScript | 846 | 6.9% | 2.5% | 1.5% | 2.8% | 0.1% |
| mmhmem, Python | 551 | 3.8% | 2.5% | 0.7% | 1.3% | 1.3% |

Percentiles of the same populations (p95 / p99 / maximum):

| Population | cyclomatic | nesting | code_lines | statements | parameters |
|---|---|---|---|---|---|
| facdrone, Python | 6 / 9 / 21 | 2 / 3 / 5 | 32 / 53 / 211 | 16 / 24 / 58 | 4 / 5 / 11 |
| mmhmem, Rust | 5 / 9 / 18 | 2 / 3 / 5 | 38 / 59 / 148 | 18 / 27 / 57 | 3 / 5 / 7 |
| mvibe, Python | 7 / 16 / 89 | 3 / 4 / 6 | 26 / 49 / 160 | 20 / 39 / 152 | 3 / 5 / 12 |
| openidea-kiro, Rust | 9 / 20 / 62 | 3 / 4 / 7 | 67 / 139 / 328 | 26 / 52 / 122 | 3 / 5 / 7 |
| mvibe, TypeScript | 12 / 91 / 185 | 2 / 5 / 6 | 23 / 91 / 189 | 23 / 122 / 226 | 3 / 4 / 7 |
| mmhmem, Python | 9 / 20 / 55 | 3 / 4 / 6 | 28 / 49 / 135 | 23 / 42 / 116 | 3 / 6 / 9 |

What the numbers say:

- **Routine limits, Rust and Python.** In four of the five populations every limit lies at or above the 95th percentile, and in the two largest at or above the 99th. The exception is `code_lines` in this repository (95th percentile 67 against the limit 60). The limits mark outliers, which is what a limit is for. No number argues for a different value.
- **Routine limits, TypeScript.** One repository, 846 routines, and a long tail: the 95th percentile of cyclomatic is 12 and the 99th is 91. The highest value, 185 in `parseWorkspaceProbe`, was read: it is one dense validation routine with long chains of `||`, so a high count is plausible. The exact number was not recounted by hand. One repository is too little to call the TypeScript limits calibrated.
- **File `max_cyclomatic`** repeats the routine finding on the file: between 0.7% and 50% of files are above it, 45% of the TypeScript files of mvibe. It adds a second finding for the same cause.
- **File `functions` at 25** lies near the 90th to 95th percentile (4.9% to 26% of files above in the larger populations).
- **This repository is the worst Rust population** on every routine metric: `code_lines` above 60 for 6.5% of its routines against 0.9% in mmhmem.

Limits of this record:

- Test code is included and not separated.
- The share above a limit says how many findings a limit produces. It does not say that a routine above the limit is harder for an agent to change; no outcome was measured.
- No severity was changed. Task 1.4 stays open: the decision to block is the owner's, and the calibration is not persisted as an executable fixture.

### Addendum: lesscode replaces Contour, and the blocking decision (2026-11-10, late)

The owner replaced `../Contour` with `../lesscode` (no C++ profile will be built). lesscode: 654 files (447 Python, 129 Rust, 73 TypeScript, 5 JavaScript).

| Population | Routines | cyclomatic > 10 | nesting > 3 | code_lines > 60 | statements > 40 | parameters > 5 |
|---|---|---|---|---|---|---|
| lesscode, Python | 2231 | 5.8% | 2.0% | 1.9% | 2.2% | 1.5% |
| lesscode, Rust | 1450 | 3.7% | 2.9% | 4.1% | 0.9% | 1.0% |
| lesscode, TypeScript | 173 | 1.2% | 0.0% | 0.0% | 0.0% | 2.3% |

Percentiles (p95 / p99 / maximum): Python cyclomatic 12 / 21 / 62, code_lines 37 / 76 / 162; Rust cyclomatic 9 / 18 / 53, code_lines 57 / 117 / 296. lesscode Python is the one population where a routine limit (cyclomatic) lies below the 95th percentile.

**Owner decisions on this record:**

1. The five routine limits block in Rust and Python. Implemented as the default severity `error` for `cyclomatic`, `nesting`, `code_lines`, `statements` and `parameters` in `.rs`, `.py` and `.pyi` files (`calibrated` in `crates/kiro-core/src/code/config.rs`). A severity named in the configuration replaces it; a weaker one needs a reason.
2. The file limit `max_cyclomatic` stays, as a warning.
3. TypeScript and JavaScript, `lines_per_statement`, and the type and file limits stay warnings.

**Consequence measured on this repository:** without a before side `code check --all` reports 231 errors. Against `main` (`code check --changed main`) the branch `feat/code-quality-port` has 56 blocking findings: routines written for this port that are above the limits, and one regression.
