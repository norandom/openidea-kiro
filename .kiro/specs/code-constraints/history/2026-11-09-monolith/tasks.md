# Implementation Plan

- [ ] 1. Foundation: engine model, catalogue, configuration, report shapes, parameter library
- [ ] 1.1 Grow the finding catalogue and declare the engine's code and breadth modules
  - Append the twenty-one `CODE_*` and seven `BREADTH_*` catalogue entries with the design's default severities and descriptions, the matching constants, and the `CODE_CODES` and `BREADTH_CODES` slices; add the rows to the crate documentation's code table so the existing doc test keeps the catalogue and the table in agreement
  - Declare the engine's `code` and `breadth` modules with one stub file per design component of the engine layer so later tasks each own one file and the crate builds before any has content
  - Observable: `cargo test -p kiro-core` passes with the catalogue test asserting twenty-eight new codes, the doc test green, and the sandbox-target build (`wasm32-wasip2`) unaffected
  - _Boundary: CodeCatalogue_
  - _Requirements: 14.4_

- [ ] 1.2 Implement the fact model, the stable entity key and deterministic JSON
  - The facts document with files, entities, token index, bindings, literal comparisons, enum matches, data accesses, clone records, imported breaches, parse failures, unmeasured files, source provenance and an optional configuration digest; the entity key with scope, project-relative path, qualified name, parameter list and ordinal, its family (scope, path, qualified name) and reversible token form; the closed metric enumeration with name, parse, scope, container-count and native flags
  - FNV-1a 64-bit hashing over bytes; serialisation with every collection sorted (entities and shapes by key token, files by path) in two-space JSON with a trailing newline, and a lenient reader that rejects a document of another schema version with one message
  - Observable: unit tests show a write–read–write round trip is byte-identical, two documents built in different orders serialise equal, known FNV-1a test vectors hash as expected, and the family of a key ignores the parameter list and ordinal
  - _Boundary: CodeFacts_
  - _Requirements: 1.3, 9.1, 9.6, 9.8, 13.4_

- [ ] 1.3 (P) Implement the configuration model with defaults, scopes, reasons and digest
  - Parse the committed configuration file (absent → shipped defaults) into limits per routine, type and file metric with severity and origin, report sources with their required flag, path scopes with their own limits, the per-rule settings of the design's configuration example, the ratchet settings and the data topics
  - Validation that collects every problem as a `CODE_CONFIG_INVALID` finding at its line: unknown rule, metric, severity or key; a non-numeric limit; an unknown required source; a scope, user-added ignore entry or relaxation (higher limit, lower severity than the default) without a `reason`
  - Scope resolution by most segments then latest in file; the effective view listing every entry with its origin and reason; a digest over the sorted effective entries that is invariant under key reordering
  - Observable: the design's example file parses to a configuration equal to the defaults; each invalid case in a table test yields exactly one finding naming the key; reordering the file leaves the digest unchanged
  - _Boundary: CodeConfig_
  - _Depends: 1.2_
  - _Requirements: 1.2, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 5.4, 6.4, 7.2, 7.4, 8.5, 9.8, 11.4_

- [ ] 1.4 (P) Implement the report model shared by every rule and renderer
  - The enriched finding wrapping the engine's finding with hint, tag (the seven tags), metric, value, before value, limit, limit origin, pre-existing flag, entity key and file owners; the run report with selection description, before side, per-spec summaries, unowned files, findings, coverage, net line, highest values and counts; the plain value types the design names in the ratchet and attribution interfaces (net line, before side, spec summary, file owners, highest value) defined here as data and re-exported by those modules
  - The coverage finding (`CODE_COVERAGE`, no line, filed against the configuration path) rendering every coverage field in one message; the net line text with the sign always written; the blocking iterator that excludes pre-existing findings; sorting by the engine's order then entity token
  - Observable: unit tests pin the coverage message and the net line for positive, negative and zero deltas, and show that a report with one pre-existing error has no blocking finding
  - _Boundary: CodeReportModel_
  - _Depends: 1.2_
  - _Requirements: 11.1, 11.5, 12.3, 13.4_

- [ ] 1.5 Implement the hint catalogue
  - One tagged hint per finding code, with metric-specific remedies for routine limits (guard clauses or extraction for complexity and nesting, extraction for length, an options record for parameters, fewer lines or comments for the shrink metrics), the component name in `split:` hints when the owner is known, the same-file versus cross-file variants for families, the `collect:` wording for the three scattered rules and the unowned file, the `name:` wording for data access, the `fix:` wording for operational findings, and the pass-through of a source's own hint for imported breaches
  - Observable: a table test asserts every catalogue code has a hint whose first token is exactly one of the seven tags, and the steering block task later quotes these texts unchanged
  - _Boundary: HintCatalog_
  - _Depends: 1.4_
  - _Requirements: 3.3, 4.3, 5.1, 5.3, 6.2, 7.2, 8.1, 9.2, 12.2_

- [ ] 1.6 (P) Create the engine fixtures
  - Facts documents, configurations and expected-findings lists per rule: a clean tree; routine limits over and at the limit; container limits for a file and a type; a duplicate block with a self-overlapping run and a same-file pair; a family of three with a nested twin and an ignored dunder name; scattered definitions with an idiom name, a two-values name and an unreadable initialiser; scattered state with a single-owner subject and an exhaustive enum; positional and ungrounded column accesses with a vocabulary carrying a `data` topic; a ratchet pair with a signature change, a pre-existing breach and a worsened entity within its limit; an unowned file beside a spec tree with a File Structure Plan; a stale baseline; an imported breach and a disagreeing imported value
  - Ledger fixtures: a valid ledger using both reference spellings and every status; a duplicate id; an unknown status; malformed TOML with a known line; a two-spec tree with requirements and tasks (some complete) for staleness and unmapped-criteria cases
  - Observable: every fixture document loads through the facts reader, the configuration parser and the ledger parser without findings (except the deliberately invalid ones), and each expected-findings file names the codes, severities and tags the rule tasks must produce
  - _Boundary: CodeFixtures_
  - _Depends: 1.2, 1.3_
  - _Requirements: 14.2, 14.4, A1.1_

- [ ] 1.7 (P) Create the shared command-line parameter library
  - A library crate with the global group (project root with its alias and environment fallback, verbosity as `-v`/`--verbose` and `-q`/`--quiet` through the adopted verbosity crate, colour choice honouring the no-colour and force-colour variables), the output group (`--format text|json` with its environment fallback, `--output <path>` with `-` for standard output, a writer that sends the document and nothing else to the chosen destination and asserts `schema_version` is the first key of a JSON document in debug builds), the gate group (`--fail-on error|warning` with its environment fallback), the exit model (clean, findings, failure; the one-line failure form; the meets and verdict helpers) and completion-script generation for a clap command; the engine's lint table; the workspace member line as an explicit touch outside this boundary
  - Observable: `cargo test -p kiro-cli-common` passes with tests for the precedence of flag over environment over default for each group, the writer's stdout-or-file behaviour, the exit code mapping, and a generated completion script for one shell containing the group flags
  - _Boundary: CliCommon_
  - _Requirements: A3.1, A3.3, A3.4, A3.5, A3.6, 14.5_

- [ ] 2. Core: engine rules, ratchet, attribution, steering, breadth, evaluation
- [ ] 2.1 (P) Implement routine, type and file limit judgements and the highest values
  - Judgements for every measured entity in the selection against the limits of its path's scope: the five routine metrics, the two shrink metrics (lines per statement only from the configured minimum statements), the type metrics, the file metrics and any source-supplied metric with a configured limit; strict comparison; the highest value per metric over the whole measured population
  - Observable: fixture tests show a value equal to the limit passes, each metric over its limit yields one judgement carrying value, limit and origin, a scoped file uses the scope's limit, and the highest list names the right entity per metric
  - _Boundary: RuleLimits_
  - _Depends: 1.3, 1.4, 1.6_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 4.5_

- [ ] 2.2 (P) Implement the duplicate-block rule with clone import
  - Window keys over consecutive line hashes built for selected files first, the project walk recording only keys a selected file carries; a run shorter than the minimum or a single repeated line is never reported; self-overlapping runs dropped; both ends reported when both are selected; up to three other locations named; imported clone records overlapping a native finding merged into it with the tool named, the rest reported on their own
  - Observable: fixture tests pin the finding count for the same-file pair, the dropped self-overlapping table, the three-location cap and a jscpd clone that is not reported twice
  - _Boundary: RuleDuplicates_
  - _Depends: 1.4, 1.5, 1.6_
  - _Requirements: 5.1, 5.7_

- [ ] 2.3 (P) Implement the similar-routine family rule
  - The gestalt ratio over token shapes (recursive longest common block, no junk heuristic); the two exact refusals (length band, token multiset) applied before any score; breadth-first family growth from selected routines over comparable pairs; exclusion of a routine from pairing with its container, of ignored names from membership and linking, of cross-language pairs and of routines without a statement count; one finding per family with members, locations, size and weakest similarity, `shrink:` within one file and `delete:` across files
  - A synthetic thousand-routine guard test that holds a family-of-two run under one second in any build
  - Observable: ratio tests reproduce known Python `SequenceMatcher` values; a refused pair, when scored anyway in the test, never reaches the threshold; the fixture family yields exactly one finding naming two other members and excluding the nested twin and the dunder
  - _Boundary: RuleFamilies_
  - _Depends: 1.4, 1.5, 1.6_
  - _Requirements: 5.2, 5.3, 5.4, 5.5, 13.5_

- [ ] 2.4 (P) Implement the scattered definitions, state and schema rules
  - Definitions grouped by name and initialiser text over readable bindings minus ignored names, reported once per selected file when the distinct file count exceeds the maximum, with up to three other files; one `CODE_DEFINITION_AMBIGUOUS` per name with two or more such groups; state grouped by subject over literal comparisons with the minimum distinct values and the file threshold, single-file subjects and ignored subjects skipped, once per file; exhaustive enum matches across too many files at info; column-name literals across too many files as `CODE_SCATTERED_SCHEMA` at info
  - Observable: fixture tests show the idiom name is silent, the two-values name produces two groups plus one ambiguity finding, the single-owner subject is silent, the enum case is info, and each finding's hint opens with `collect:`
  - _Boundary: RuleScattered_
  - _Depends: 1.4, 1.5, 1.6_
  - _Requirements: 6.2, 6.3, 6.5, 7.2, 7.3, 7.4, 8.4_

- [ ] 2.5 (P) Implement the data-access rule with vocabulary grounding
  - Positional accesses in selected files reported with the pattern name and a `name:` hint; with data topics configured and a vocabulary present, every named access resolved within those topics, `Unknown` and `NotInTopic` reported with up to three suggestions (visible terms sharing a three-character normalised prefix or within edit distance two), `Ambiguous` reported with the candidate ids, `Found` silent; without data topics the grounding step is skipped and the topics-used list is empty
  - Observable: fixture tests cover each lookup outcome, the suggestion ordering, and the no-topics case returning positional findings only
  - _Boundary: RuleData_
  - _Depends: 1.4, 1.5, 1.6_
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [ ] 2.6 Implement the ratchet, pre-existing classification, net line and staleness
  - Pairing by equal key, then by family when exactly one key disappeared and one appeared, else new; the classification of the design's flowchart turning judgements into limit findings (pre-existing when the before value also exceeded and did not worsen) and `CODE_RATCHET` findings (error over the limit, the configured below-limit severity within it); no ratchet on container counts or imported breaches; the net line over routine entities of the selected files with deleted routines negative; `CODE_NET_GROWTH` only when a maximum is configured; `CODE_BASELINE_STALE` when the before document's digest differs from the effective configuration's
  - Observable: a table test covers all eleven flowchart outcomes; the signature-change fixture ratchets across the renamed key; a container count that grew produces a limit finding but no ratchet finding; the net line for the fixture reads with its sign
  - _Boundary: Ratchet_
  - _Depends: 2.1_
  - _Requirements: 1.3, 4.4, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.9_

- [ ] 2.7 (P) Implement attribution and selection by spec and task
  - Owners of a file through the engine's join over spec views in roadmap order then name: the first covering spec as feature, its components and bound tasks, the other covering specs listed; a design entry ending in a separator or without an extension that prefixes a path selects every file beneath it, otherwise the join's path matching; selection of the files of one spec and of one task's boundary components with errors that carry the alternatives (unknown spec, unknown task, task without boundary, boundary naming no component); per-spec summaries with counts and net line and the list of unowned files; `CODE_FILE_UNOWNED` once per file with findings
  - Observable: fixture tests over a two-spec tree show roadmap order decides the feature, a file mapped by both specs lists the second, a task selection returns exactly its components' files, and each selection error names the available alternatives
  - _Boundary: Attribution_
  - _Depends: 1.4, 1.6_
  - _Requirements: 4.3, 10.1, 10.2, 10.3, 10.4, 10.5_

- [ ] 2.8 (P) Implement the steering block and its marker splice
  - The block text from the effective configuration: limits per scope, rules in force with their hint tags quoted from the hint catalogue, the seven-rung ladder with rungs three to five marked as the agent's alone and never checked, the ratchet statement (only regression blocks, growth within a limit warns, container counts are not ratcheted), and the per-task self-check command; the splice that replaces the text between the begin and end markers, appends the block when the markers are absent, and creates the document when there is none
  - Observable: unit tests pin the block for the defaults, show a changed limit changes exactly its line, and cover the three splice cases without altering text outside the markers
  - _Boundary: SteeringBlock_
  - _Depends: 1.3, 1.5_
  - _Requirements: 11.6, 12.1_

- [ ] 2.9 Implement the evaluation entry point
  - One function from facts, optional before facts and before side, configuration, selection, spec views, optional vocabulary and the show-highest flag to the run report: resolve the selection; exclude files with a parse failure from judgement, reporting `CODE_PARSE_FAILED` as an error inside the selection and a warning outside; run limits, ratchet, duplicates, families, scattered, data, imported breaches (`CODE_IMPORTED`) and native-versus-imported disagreements (`CODE_SOURCE_DISAGREES`, once per run); attribute every finding; skip rules that are off or lack facts and record them in coverage; build the coverage record and append its finding; sort
  - Observable: over the clean fixture the report holds exactly the coverage finding; over each rule fixture the expected findings file matches the report's codes, severities and tags; a disabled rule appears under skipped rules and produces nothing
  - _Boundary: CodeEvaluate_
  - _Depends: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8_
  - _Requirements: 1.5, 1.6, 8.3, 11.1, 11.2, 11.3, 11.4_

- [ ] 2.10 (P) Implement the breadth ledger model and checks
  - Parsing of the ledger (schema version, capabilities with id, requirement references in both spellings with single criteria, whole requirements and in-requirement ranges, planned checks, test and end-to-end paths with the optional `::name` suffix, status, evidence) into plain data, with `BREADTH_INVALID` at the first problem's line for malformed TOML, a missing capabilities list, a duplicate id or an unknown status
  - The checks over spec views and the read port: reference resolution (`BREADTH_REQ_UNKNOWN`, including a range spanning two requirements), path existence through the read port (`BREADTH_TEST_MISSING`, warning while planned), the status rules (`BREADTH_STATUS_UNSUPPORTED`: passed needs a test path, an end-to-end path and evidence; implemented needs a test path; partial, unverified and deferred need evidence), staleness when every task whose requirements intersect the capability's criteria is complete while the status is planned (`BREADTH_STALE`), per-spec unmapped criteria for specs at or past task generation (`BREADTH_CRITERION_UNMAPPED` with count and first five ids) and the coverage summary (`BREADTH_COVERAGE`)
  - Observable: ledger fixture tests cover both reference spellings, every status rule, the cross-requirement range, each invalid document with its line, the staleness case over the two-spec tree, and the unmapped and coverage outputs
  - _Boundary: BreadthLedger_
  - _Depends: 1.1, 1.6_
  - _Requirements: A1.1, A1.2, A1.3, A1.4, A1.5, A1.6, A1.7, A2.4_

- [ ] 3. Core: the native measurer crate and the report adapters
- [ ] 3.1 Scaffold the measurer crate with its languages and confined source walk
  - The crate with the pinned parser runtime and the four grammar crates, the engine as a dependency, the engine's lint table, and an explicit touch outside this boundary: the workspace member line in the root manifest; a C compiler is a build prerequisite and the task records how the development machine and the CI image satisfy it
  - The language model: the five languages with their extension map, the token classes, the adapter trait of the design, and a registry that already names the three adapter modules as stubs so later adapter tasks fill only their own file; the token indexer and pattern matcher as stubs for the same reason
  - The confined walk: depth-first in sorted order under the root, include and exclude globs from the configuration with the shipped exclude defaults, symlinks leaving the root refused, UTF-8 reads up to sixteen mebibytes, every path returned with forward slashes
  - Observable: `cargo build -p kiro-code` and `cargo clippy -p kiro-code -- -D warnings` pass; a smoke test parses one minimal file per grammar without an error node (the ABI check); walk tests cover excludes, the size limit and a symlink escape; the engine's sandbox-target build is unaffected
  - _Boundary: MeasurerCrate, LanguageModel, SourceWalk_
  - _Depends: 1.3_
  - _Requirements: 1.1, 1.6, 13.6, 14.5_

- [ ] 3.2 (P) Create the measurer fixture trees
  - Source trees per language (Rust, Python, TypeScript and JavaScript) exercising routines with hand-counted metrics, types with methods and fields (Rust inherent implementation blocks across one file), module-level bindings including a twelve-line initialiser and an unreadable one, literal comparisons in equality tests, match and switch arms and membership tests, an exhaustive and a non-exhaustive Rust enum match, the shipped data-access patterns in positive and negative forms, nested routines and an ignored dunder; one file with a syntax error; a scitools-hook report fixture with threshold records, highest entries, structural and codecheck breaches and parse errors; jscpd fixtures in both key spellings; an invalid report file
  - Observable: the trees exist with a short README per tree stating the expected counts and findings, and every source file parses with the grammar smoke test except the deliberate failure
  - _Boundary: MeasurerFixtures_
  - _Depends: 3.1_
  - _Requirements: 14.2, 14.4_

- [ ] 3.3 Implement the measurer core together with the Rust adapter
  - Explicit two-boundary task: the measurer needs one real adapter to be tested, and the Rust adapter is the first. The core: one parser per language per run; a tree with an error node yields a parse failure at the first error and no facts for the file; entity keys from the adapter's name path, collapsed parameter text and document-order ordinals; the routine metrics (strict cyclomatic, nesting, code lines, statements, parameters excluding the receiver, comment lines, lines per statement), the type metrics and the file metrics computed over the adapter's nodes; assembly into the facts document with provenance; a text-only entry point for the before side and tests
  - The Rust adapter: functions and methods as routines, structs, enums, traits and unions as types with inherent implementation methods counted, const and static items as bindings with initialiser text capped at twelve lines, equality tests and literal match arms as comparisons, exhaustive variant matches, token classes and statement and decision kinds
  - Observable: the Rust fixture tree's facts snapshot matches the hand counts in its README; the syntax-error file produces one parse failure and nothing else; measuring the same text twice yields byte-identical facts
  - _Boundary: Measurer, RustAdapter_
  - _Depends: 3.1, 3.2_
  - _Requirements: 1.1, 3.1, 3.2, 4.1, 4.2, 6.1, 7.1, 7.3, 9.1, 11.2_

- [ ] 3.4 (P) Implement the Python adapter
  - Functions and methods as routines (lambdas excluded), classes as types with methods and instance attributes, module-scope assignments as bindings, equality tests, match-case literal patterns and membership over literal collections as comparisons, docstrings counted as comment lines, token classes and statement and decision kinds
  - Observable: the Python fixture tree's facts snapshot matches its README counts, including the docstring comment lines and the membership-test comparisons
  - _Boundary: PythonAdapter_
  - _Depends: 3.3_
  - _Requirements: 1.1, 3.1, 4.2, 6.1, 7.1_

- [ ] 3.5 (P) Implement the TypeScript and JavaScript adapter
  - One adapter over the TypeScript, TSX and JavaScript grammars: function declarations, methods and named arrow functions as routines, classes, interfaces and enums as types, top-level declarations as bindings, strict and loose equality tests and switch cases with literals as comparisons, token classes and statement and decision kinds
  - Observable: the TypeScript and JavaScript fixture trees' facts snapshots match their README counts, and a TSX file measures the same as its TS twin
  - _Boundary: TypeScriptAdapter_
  - _Depends: 3.3_
  - _Requirements: 1.1, 3.1, 4.2, 6.1, 7.1_

- [ ] 3.6 (P) Implement the token indexer
  - From leaf tokens in document order with comments and whitespace dropped: one hash per code line per file; one shape per routine with identifiers and literals collapsed to their classes and every other leaf kept as text, encoded through one run-wide vocabulary; files whose leaves cannot be enumerated listed as unreadable
  - Observable: tests show two routines differing only in identifier and literal names have identical shapes, a comment or re-indentation leaves line hashes unchanged, and the shape of a six-statement routine has the expected statement count
  - _Boundary: TokenIndexer_
  - _Depends: 3.3_
  - _Requirements: 5.1, 5.2_

- [ ] 3.7 (P) Implement the pattern matcher with the shipped data-access patterns
  - Compilation of the shipped and configured queries once per run; the Python patterns (dataframe positional column selection, two-dimensional array column axis, row record index, column literals in selection positions) and the Rust patterns (positional column and array index, column literals); a configured query that fails to compile reported as a configuration finding, a shipped one as a test failure
  - Observable: every shipped pattern fires on its positive fixture and stays silent on its negative one; a deliberately broken configured query produces `CODE_CONFIG_INVALID`
  - _Boundary: PatternMatcher_
  - _Depends: 3.3_
  - _Requirements: 8.1, 8.2, 8.5_

- [ ] 3.8 (P) Implement the scitools-hook report adapter
  - Reading the run-result document (schema 2): threshold records with a value and an entity key and the highest entries become entity facts with the mapped metric; structural, codecheck and ratchet records and value-less thresholds become imported breaches carrying the source's hint; parse errors become parse failures with the source marked; unknown metric names are skipped and counted; documents of another schema or without findings are an adapter error with the first problem and its line when it is a syntax error; files touched are recorded for coverage
  - Observable: the fixture report yields the expected entity facts, breaches and parse failures, and the invalid fixture yields an adapter error naming the problem
  - _Boundary: ScitoolsAdapter_
  - _Depends: 1.2, 3.2_
  - _Requirements: 1.2, 1.3, 1.4, 3.5, 4.5_

- [ ] 3.9 (P) Implement the jscpd report adapter
  - Reading the duplicates array in both key spellings (first and second file, or duplication A and B) with name, start and end lines falling back to the location objects, line and token counts, paths made project-relative by stripping the root prefix; a document without a duplicates array is an adapter error
  - Observable: both fixture spellings yield the same clone records, and the invalid fixture yields an adapter error
  - _Boundary: JscpdAdapter_
  - _Depends: 1.2, 3.2_
  - _Requirements: 1.2, 1.4, 5.7_

- [ ] 3.10 (P) Implement the git before side
  - Verification of the revision, the changed-file list with statuses from one diff process, and the before texts of the selected changed and deleted paths from one batch object-reading process; a missing path at the revision reads as new; failures (no repository, unknown revision, process error) are one error with git's first line
  - Observable: a test repository with two commits yields the expected statuses and before texts; an unknown revision yields the error; the measured before facts for a changed file come from the earlier commit
  - _Boundary: GitBefore_
  - _Depends: 3.3_
  - _Requirements: 9.4_

- [ ] 4. Integration: the command line on the shared parameter standard
- [ ] 4.1 Migrate the existing commands onto the shared parameter library
  - Explicit integration task across the existing command modules: the root command flattens the shared global group (the project root keeping `--root` as a hidden alias, verbosity replacing the boolean flag with `--verbose` still accepted, colour); `check` and `vocab validate` flatten the output and gate groups; `graph` and `tasks` take `--format table` with `--table` as a hidden alias; `vocab import` takes `--output` with `--out` as a hidden alias; the exit module becomes a re-export of the shared exit model; the `completions <shell>` command and `--version` are added; every environment fallback of the standard applies
  - Observable: every existing CLI test passes unchanged (byte-identical output); `kiro-cli graph demo --table` and `--format table` print the same bytes; `KIRO_PROJECT` selects the root when `--project` is absent; `kiro-cli completions bash` prints a script naming `--fail-on`
  - _Boundary: CodeArgs_
  - _Depends: 1.7_
  - _Requirements: A3.2, A3.3, A3.5, A3.6, 13.2_

- [ ] 4.2 Define the code command tree and wire the entry point
  - The `code` command family with check, baseline, rules and config composed from the shared groups; the check options (the five selections with exactly one required, show-highest, repeatable source assignments in the format-equals-file form, SARIF path) beside the shared output and gate groups; the baseline output option; the rules write option with its default path; translation of source assignments into source specifications merged over the configuration; the measurer crate as a dependency of the CLI crate
  - Explicit touch of the entry point: dispatch arms for the four commands pointing at placeholder failures until the command tasks replace them; help and version still exit 0, usage errors exit 2
  - Observable: `kiro-cli code --help` lists the four commands with the shared flags; `code check` with no or two selections exits 2 with the usage line; a malformed source assignment exits 2 naming it
  - _Boundary: CodeArgs_
  - _Depends: 3.1, 4.1_
  - _Requirements: 1.2, 3.6, 10.4, 10.6, 13.1, 13.2, 14.5, A3.2_

- [ ] 4.3 (P) Implement the text and JSON renderers
  - Text: the header with selection and before side, per-spec sections with the summary line and each finding in the engine's line form followed by its indented hint line and a pre-existing marker, the unowned files section, the highest-values table when asked, the net line, the summary line with the fail-on verdict; JSON: the schema-versioned document with selection, before side, per-spec summaries, unowned files, flattened findings carrying every design field, coverage, net line, highest values, counts, fail-on and verdict, pretty-printed with one trailing newline and written through the shared output group's writer
  - Observable: unit tests pin the text of a report with one pre-existing and one blocking finding, the JSON key order with `schema_version` first and the byte-equality of two renderings
  - _Boundary: CodeRenderers_
  - _Depends: 1.4, 1.7_
  - _Requirements: 3.6, 9.7, 10.5, 11.5, 12.2, 12.3, 13.4, A3.5_

- [ ] 4.4 Implement the SARIF writer
  - A SARIF 2.1.0 document with the tool driver naming the gate and one rule per catalogue code, one result per finding with rule id, level mapped from severity, the message carrying the finding text and the hint as a second sentence, the physical location with project-relative uri and start line, and properties for tag, pre-existing flag, feature, components and tasks
  - Observable: the document written for the fixture report parses back as JSON with the expected rule ids and levels and no absolute path
  - _Boundary: CodeRenderers_
  - _Depends: 4.3_
  - _Requirements: 13.3_

- [ ] 4.5 Implement the check command
  - The run: resolve the root; read and parse the configuration (findings to standard error, exit 2); discover specs, build the workspace with roadmap adoption, load every discovered spec and the vocabulary; measure the whole tree; take the before side from the baseline when present or from the named revision (the revision wins when both exist), failing with exit 2 on an unreadable baseline or a git error; import each source, a missing or invalid required one failing with exit 2 and an optional one becoming a source-invalid finding; evaluate; render the chosen format through the shared writer; write the SARIF document when asked; exit by the verdict over non-pre-existing findings; under verbose print discovered specs, walked and skipped files, parse times and the before-side source on standard error
  - Explicit integration touch: replaces only its own dispatch arm in the entry point
  - Observable: over the clean fixture project the command exits 0 printing the coverage finding; over the routine-limit project it exits 1 with the limit finding and its `simplify:` hint; with a baseline that already carries the breach it exits 0 marking it pre-existing; `--task` over a spec fixture judges only the boundary's files; an unknown task exits 2 naming the available tasks
  - _Boundary: CodeCheckCommand_
  - _Depends: 2.9, 3.6, 3.7, 3.8, 3.9, 3.10, 4.2, 4.3, 4.4_
  - _Requirements: 1.4, 1.7, 2.1, 2.4, 9.3, 9.4, 9.9, 10.4, 10.6, 11.5, 13.2, 13.6_

- [ ] 4.6 Implement the baseline command
  - Measure the whole tree with the effective configuration, set the configuration digest, write the sorted facts document to the default path or the given one, print one line with file and entity counts; configuration and I/O failures exit 2
  - Explicit integration touch: replaces only its own dispatch arm in the entry point
  - Observable: two captures of the same tree are byte-identical; a following check reports the before side as the baseline; capturing after a configuration change makes the earlier baseline report as stale
  - _Boundary: CodeBaselineCommand_
  - _Depends: 4.5_
  - _Requirements: 9.8, 13.6_

- [ ] 4.7 Implement the rules and config commands
  - Rules: print the steering block, or with the write option read the existing steering document, splice the block and write it, printing the path; config: print every effective entry with origin and reason sorted by key, then the sources and data topics
  - Explicit integration touch: replaces only its own two dispatch arms in the entry point
  - Observable: writing twice leaves the steering document unchanged on the second run; a file with surrounding prose keeps that prose; the config listing shows a scoped limit with its scope name and reason
  - _Boundary: CodeRulesCommand, CodeConfigCommand_
  - _Depends: 4.6_
  - _Requirements: 2.6, 12.1, 13.6_

- [ ] 4.8 Implement the breadth check command
  - Resolve the root, read the ledger through the read port (absent → the invalid finding and exit 1), discover specs and load each one's requirements and tasks into spec views, run the engine's breadth check with the filesystem read port for path existence, render the findings with the engine's line form plus the summary line, or the schema-versioned JSON form, through the shared output group; exit by the gate verdict
  - Explicit integration touch: adds only its own dispatch arm to the entry point
  - Observable: over the valid ledger fixture project the command exits 0 with the coverage summary; a ledger claiming `passed` without evidence exits 1 with the status finding; a reference to an unknown criterion exits 1 naming it; `--format json` parses with `schema_version` first
  - _Boundary: BreadthCommand_
  - _Depends: 2.10, 4.1, 4.7_
  - _Requirements: A1.2, A1.3, A1.6, A1.7, A3.2_

- [ ] 5. Validation: binary-level tests, conformance, end-to-end journeys, measured bounds
- [ ] 5.1 Build the CLI test harness and fixture projects
  - A helper that assembles a temporary project from a measurer fixture tree plus a spec tree with a File Structure Plan, a roadmap, a vocabulary and an optional ledger, runs the built binary and captures status, standard output and standard error; a source-grep test asserting the filesystem API appears in the CLI only in its adapter module and in the measurer only in its walk module
  - Observable: a smoke run over the clean project exits 0 through the helper, and the grep test passes
  - _Boundary: CodeCliTests_
  - _Depends: 4.8_
  - _Requirements: 14.2, 14.4_

- [ ] 5.2 Write the check scenarios
  - One scenario per rule fixture asserting codes, severities, hints and tags in text and JSON; exit 0, 1 and 2 cases (clean, blocking, pre-existing only, usage, configuration without reason, unknown task, required source missing, unreadable baseline, unknown revision); the five selections; fail-on warning; a scoped limit; a disabled rule appearing as skipped; the coverage finding's content with and without data topics; the attribution of a finding to its spec, component and tasks and the unowned-file case
  - Observable: every scenario passes on the development platform with forward slashes and newline endings only
  - _Boundary: CodeCliTests_
  - _Depends: 5.1_
  - _Requirements: 1.4, 1.7, 2.4, 3.3, 4.3, 5.3, 6.2, 7.2, 8.1, 8.2, 8.3, 9.3, 10.1, 10.3, 10.4, 10.6, 11.1, 11.2, 11.4, 13.2_

- [ ] 5.3 (P) Write the determinism, document, companion-command and breadth-command tests
  - Two runs byte-equal in text and JSON; two baseline captures byte-equal; the SARIF document parses back with rule ids equal to the codes; the rules command writes an idempotent block and preserves surrounding text; the config command lists origins and reasons; the `--changed` selection over a temporary repository ratchets a worsened routine and prints the net line; the breadth command over each ledger fixture yields the expected findings and exit codes, including the stale and unmapped cases
  - Observable: all scenarios pass and no file outside the temporary directories is created
  - _Boundary: CodeCliTests_
  - _Depends: 5.1_
  - _Requirements: 9.4, 9.7, 9.8, 12.1, 12.3, 13.3, 13.4, A1.4, A1.5, A1.6_

- [ ] 5.4 (P) Write the parameter-standard conformance test
  - A test that walks every command and subcommand of the built binary through `--help`, asserts the shared groups appear with identical names, values and defaults, that no short flag outside `-v -q -h -V` exists, that the hidden aliases (`--root`, `--table`, `--out`) are still accepted with byte-identical output to their standard forms, and that the standard document names every global flag the binary prints
  - Observable: the test passes over the whole command tree and fails on a deliberately added non-standard flag in a scratch build
  - _Boundary: CodeCliTests_
  - _Depends: 5.1_
  - _Requirements: A3.2, A3.4, A3.7_

- [ ] 5.5 (P) Write the timing tests and record the first inventory
  - Release-only tests holding a task-scoped check over any task of this repository under ten seconds and a whole-tree check under sixty, skipped with a visible message in debug builds; a run of the whole-tree check and the highest values over this repository and over the sibling IDE repository, with the counts per rule and the highest values recorded in the research log under a note that the first number is not a quality score
  - Observable: both timing tests pass in a release build, and the research log carries the inventory table with the date and the build
  - _Boundary: CodeCliTests_
  - _Depends: 5.1_
  - _Requirements: 13.5, 14.1_

- [ ] 5.6 Build the end-to-end harness and evidence log
  - One end-to-end test binary with a harness that assembles temporary projects from fixture trees, spec trees, roadmaps, vocabularies and ledgers, initialises git repositories and commits named states when a journey asks, runs the built binary with explicit arguments and captures status, standard output, standard error and written files; an evidence line per step (`e2e: <journey> | <command line> | exit <n> | stdout sha256 <digest>`) printed to the test output; tool probes that skip a journey with a visible `skipped: <tool> not found` message rather than silently
  - Observable: a smoke journey runs the binary's help through the harness and prints one evidence line; the git probe skips visibly when the tool is hidden from the test's path
  - _Boundary: E2eSuite_
  - _Depends: 4.8_
  - _Requirements: A2.1, A2.3, A2.5_

- [ ] 5.7 Write the implementer, CI and change journeys
  - Implementer loop: a project with a spec whose task boundary maps a file holding an over-limit routine; `code check --task` exits 1 with the limit finding and its hint; the routine is split; the same command exits 0. CI loop: `code baseline` captured; a routine regresses; `code check --all` exits 1 with a ratchet finding; a tree whose only breach is pre-existing exits 0. Change loop: a two-commit repository with a worsened and a new routine; `code check --changed <rev>` reports the ratchet on the worsened one, judges the new one by limits alone and prints the net line
  - Observable: all three journeys pass with their evidence lines, and each asserts on output, exit code and written files only
  - _Boundary: E2eSuite_
  - _Depends: 5.6_
  - _Requirements: A2.2_

- [ ] 5.8 (P) Write the onboarding, sources, breadth and attribution journeys
  - Onboarding loop: `code config` lists the defaults, `code rules --write` creates the steering document with the block, and both `check --all` and `code check --all` run on the fresh project. Sources loop: scitools-hook and jscpd fixture reports imported and visible in coverage; a required source missing exits 2. Breadth loop: a ledger moving from `planned` to `implemented` to `passed` exits 0 at each step, while `passed` without evidence, a `planned` capability whose tasks are all complete, and an unknown criterion reference are refused with their findings. Attribution loop: a finding names its spec, component and tasks; a file outside every design is reported unowned
  - Observable: all four journeys pass with their evidence lines; the breadth journey's ledger states are the ones the self-gate later uses
  - _Boundary: E2eSuite_
  - _Depends: 5.6_
  - _Requirements: A2.2, A1.9_

- [ ] 6. Integration: self-gate, documentation, agent contract, final gate
- [ ] 6.1 Gate this repository
  - The repository's configuration with a tests scope carrying reasons and with fixture directories excluded as analysis inputs; the baseline captured after the inventory; the steering document generated through the rules command; the ledger seeded with this spec's capabilities as `planned` and moved to `implemented` and `passed` with dated evidence from the end-to-end journeys; the `kiro-code` and `kiro-breadth` Cargo aliases; the Dagger steps after the spec gate: code gate, breadth check, then the end-to-end suite; the workspace members
  - Observable: `cargo kiro-code` and `cargo kiro-breadth` exit 0 on this tree; the Dagger gate's step list includes the three steps; the generated steering document matches a fresh generation; every capability of this spec is `passed` with an end-to-end path and evidence
  - _Boundary: SelfGate_
  - _Depends: 5.5, 5.7, 5.8_
  - _Requirements: 14.1, A1.1, A1.9, A2.4, A2.5_

- [ ] 6.2 (P) Write the rules reference, the breadth and standard documents, and the CI and CLI sections
  - The rules reference: how to read a report, one entry per rule (reports, does not report, default, measurement or source), the feature map and pendant decisions summarised from the research log, every configuration key with the reason rule, the bundled languages and extensions, the accepted report formats and what each supplies, the agent contract (steering block, the seven tags, the ladder with the never-checked rungs, the human-owned adaptation procedure), the honest limits, and the end-to-end-first policy; the breadth document (schema, status rules, workflow, the check and its codes); the command-line standard document (groups, precedence, naming, aliases, output rules); the CI guide gains the code gate, the breadth check, the end-to-end suite and baseline capture; the command reference gains the code and breadth command families and completions
  - Observable: a test compares the command reference's option names with the binary's help output; every rule and every breadth code in the catalogue has an entry in the documents, checked by a test over them; the conformance test of 5.4 reads the standard document
  - _Boundary: CodeDocs_
  - _Depends: 4.8_
  - _Requirements: 1.1, 1.2, 2.2, 3.5, 4.5, 5.6, 6.4, 6.6, 7.5, 8.5, 8.6, 11.6, 12.6, 14.3, A1.1, A2.6, A3.7_

- [ ] 6.3 (P) Amend the skills and the agent-guidance steering
  - The implementation skill runs the task-scoped code check in JSON before completion verification in both modes, treats blocking findings as review failures to fix in code (never by editing the configuration, the baseline or the ignore lists), and moves the mapped ledger capability to `implemented` with its actual test paths; the review skill lists the code report among its inputs and checks the findings' components in its boundary audit; the validation skill runs the whole-tree code gate, the breadth check and the end-to-end suite as mechanical checks and moves capabilities to `passed` only with an end-to-end path and a dated evidence record; the task-generation skill enters a spec's capabilities into the ledger as `planned`; the agent-guidance steering gains the Code Gate and the ledger as the code half of the feedback contract
  - Observable: each amended document names the exact commands and ledger transitions, and a dry read of the implementation skill's completion steps shows the code check and the ledger update before the verification gate
  - _Boundary: SkillAmendments_
  - _Depends: 4.8_
  - _Requirements: 12.4, 12.5, 12.6, A1.8_

- [ ] 6.4 Run the final quality gate
  - Format check, clippy with warnings denied over the workspace, the native test suite, the doc tests, dependency policy, the engine's sandbox-target build, the standalone plugin checks, the spec gate, the code gate, the breadth check and the end-to-end suite over this repository; the roadmap entry for this spec ticked when every task is complete
  - Observable: every command above exits 0 on the development machine and in the Dagger gate, the ledger shows every capability of this spec `passed`, and the validation skill reports the feature complete
  - _Boundary: SelfGate, CodeCliTests, E2eSuite_
  - _Depends: 6.1, 6.2, 6.3_
  - _Requirements: 14.4, 14.5, A2.4_
