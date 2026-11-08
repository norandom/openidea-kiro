# Implementation Plan

- [x] 1. Foundation: crate scaffold, exit model, engine catalogue, fixtures
- [x] 1.1 Scaffold the CLI crate and the exit model
  - Prerequisite: `crates/kiro-core` is implemented with its `checks`, `vocab` and `viz` modules (kiro-core, vocabulary-grounding, spec-constraints and spec-visualization specs) and builds natively
  - Add the CLI crate to the root workspace member list (the growth point the kiro-core design reserves), create the package with the binary target, the argument-parsing, JSON and RDF dependencies at the design's version lines, the test-only temporary-directory dependency, and the engine's lint table (unsafe code forbidden; unwrap, expect, panic, indexing, todo and unimplemented denied)
  - Implement the exit model: the three-valued exit mapped to process exit codes 0/1/2, the single-line failure type (newlines flattened), the fail-on level with its explicit "meets" rule (error always, warning only under fail-on warning, info never) and the verdict over a finding iterator, plus severity-name parsing
  - A minimal entry point that only maps the exit model to the process exit code and prints a failure as one line on standard error; the command-line parser is wired in by task 2.2
  - Observable: `cargo build -p kiro-cli` succeeds, the engine's sandbox-target build is unaffected, and unit tests cover the meets table, the verdict and the failure line flattening
  - _Boundary: CliScaffold, ExitModel_
  - _Requirements: 6.4, 6.6, 13.1_

- [x] 1.2 (P) Append the importer finding codes to the engine catalogue
  - Add the four import codes (input invalid as error; term skipped, duplicate id and merge conflict as warnings) with the design's descriptions to the engine's finding catalogue and constants module, and the matching rows to the crate documentation's code table so the existing doc test keeps both in agreement; the "emitted by" column names the CLI importer
  - Observable: the four constants are exported from the engine's finding module and listed in its documented code table, the engine's catalogue test and doc test pass, and the sandbox-target build of the engine still succeeds
  - _Boundary: CoreIntegration_
  - _Requirements: 13.2_

- [x] 1.3 (P) Create the CLI fixtures
  - Import inputs: the Auto Ontology page object shaped after the OpenAPI response (`terms` array with a term carrying synonyms and zones, a second term whose name derives the same id, a term with a null name, a term with an empty description; the ignored count arrays present), the same terms as a bare array, two detail-shaped objects with related terms and tags, a JSON file with a syntax error at a known line, a Turtle file with one concept scheme carrying a title and concepts exercising every mapped predicate (the three CamelCase id examples of the mapping, three label language forms, definitions, alternative labels, broader and inverse narrower, deprecation with replacement, topic, enum and int annotations, a collection and a blank node to ignore, a broader target that is not a concept), a Turtle file with a syntax error at a known line, and a merge target vocabulary with comments, a declared topic, a deprecated term, an existing id and an alternative label that both collide with the page fixture
  - The warnings-only tree: a copy of the known-good check vocabulary, a roadmap listing one spec, and a spec in the tasks-generated phase with clean requirements and tasks but no design document, so that it yields exactly one missing-document warning and no check finding
  - The vocabulary-findings tree: the known-good check vocabulary plus one term with an empty definition, a roadmap, and two copies of the clean spec under different names, so that a run over both specs must print the vocabulary's single warning exactly once
  - Observable: the files exist at the planned paths; the merge target loads through the engine loader with no warning or error; each tree is parseable by the engine with exactly its one expected warning
  - _Boundary: CliFixtures_
  - _Requirements: 13.4, 13.5_

- [x] 2. Core: filesystem adapter and command line
- [x] 2.1 Implement the filesystem adapter
  - Project root resolution (canonicalised, must be a directory, display form with forward slashes), the engine read port over the filesystem with the lexical guard (absolute paths, drive prefixes, leading separators and parent segments refused without touching the disk) and the host-parity limits (regular file, at most 16 MiB, valid UTF-8; any failure is "absent"), optional verbose diagnostics of refused or failed reads on standard error, spec discovery over the specs directory (direct subdirectories containing a spec or requirements document, sorted, no recursion, files and dot-directories ignored, failure when the directory is missing), operator-argument file reading as given with byte-order-mark stripping, output existence and writing, and path display normalisation
  - This is the only module that names the standard filesystem API; keep every other module free of it
  - Observable: unit tests cover the guard table, oversized, non-UTF-8 and directory reads returning nothing, discovery ordering and filtering, and the read port never panics on any path string
  - _Boundary: FsIo_
  - _Depends: 1.1_
  - _Requirements: 1.1, 1.2, 1.3, 1.7, 2.1, 2.2, 2.3, 2.4, 2.5_

- [x] 2.2 (P) Define the command line, the check configuration translation and the parser wiring
  - The command tree of the design's command summary with the global project and verbose options; the check options (feature list, all, report format, fail-on, check selection list, pattern reporting, suggestion disabling, the two caps, repeatable severity overrides); view arguments with the table flag; vocabulary validate and import arguments (one or more input files, required input format and topic, kind with default entity, output path, merge); this task owns the input-format value enum that the importer reuses
  - Selection resolution producing the design's usage failure when neither or both of feature and all are given, with duplicate features removed and order kept; translation of the check options into the engine's check configuration starting from its defaults, with override parsing that rejects unknown check codes, unknown levels and zero caps by name; topic validation against the vocabulary id rule
  - Explicit touch of the entry point: wire the non-exiting parser into it (help and version exit 0; any other parse error exits 2 after the parser's own message) and dispatch to a placeholder failure per command until the command tasks add their arms
  - Observable: unit tests cover the selection table, every option's effect on the configuration, and each rejection message; `kiro-cli --help` exits 0, an unknown subcommand exits 2, and `kiro-cli check` with no selection exits 2 with the usage line
  - _Boundary: CliArgs_
  - _Depends: 1.1_
  - _Requirements: 1.5, 3.4, 3.5, 3.6, 3.7, 3.8, 6.3, 9.1_

- [x] 3. Core: the check gate
- [x] 3.1 Implement the report envelope and the text and JSON reporters
  - The gate report envelope (specs checked, vocabulary outcome, roadmap findings, per-spec results with the engine's check report plus merged findings, counts, fail-on, exit) and the vocabulary report; the text form with its four sections (vocabulary line and findings, roadmap findings when any, per-spec scope line and the three status lines reproduced from the IDE's check notification wording followed by the engine's finding lines or "No findings.", the summary line with counts, fail-on and verdict); the JSON form with the schema version, spec list, per-spec topics, vocabulary status and per-check status, counts, fail-on, verdict and the findings serialised by the engine's own derive; vocabulary report text and JSON
  - No timestamps, no absolute paths, newline-only line endings; two renderings of equal reports are byte-equal
  - Observable: unit tests pin the scope and status line wording with literal strings from the spec-constraints design, the summary line, the JSON key order, and byte-equality across two renderings
  - _Boundary: Reporters_
  - _Depends: 1.1_
  - _Requirements: 3.9, 4.1, 4.2, 4.3, 4.4, 4.5, 5.1, 5.2, 5.3, 8.1_

- [x] 3.2 Implement the check command
  - Selection against the discovered specs (unknown feature or empty discovery fail with the discovered names or the root), one workspace over the filesystem adapter with discovered and roadmap names known, roadmap findings once, vocabulary loaded once with its findings kept in their own section, then per selected spec the engine's check run and the spec's structural findings merged (vocabulary-file pass-through copies dropped by file name) and sorted; counts and verdict over every finding; printing of the chosen report form and exit by verdict; a stdout write failure exits 2; under verbose, the discovered spec names are printed on standard error
  - Explicit integration touch: adds only its own dispatch arm to the entry point
  - Observable: running the command over the known-good check tree prints a passing report and exits 0, over the known-bad tree prints the contradiction lines and exits 1, and `--format json` output parses with the documented keys
  - _Boundary: CheckCommand_
  - _Depends: 2.1, 2.2, 3.1_
  - _Requirements: 1.4, 1.5, 1.6, 1.7, 3.1, 3.2, 3.3, 3.9, 5.4, 6.1, 6.2_

- [ ] 4. Core: views and vocabulary validation
- [ ] 4.1 (P) Implement the view commands
  - Status, spec graph and task graph over the engine's view functions with the raw labeler and the engine's caps, mirroring the plugin's command bodies step by step (successor chain only for superseded specs; table form on the flag; the two "nothing to draw" messages verbatim); the spec must be a discovered one, otherwise failure naming the discovered specs; printed text gets exactly one trailing newline; exit 0 on success, 2 on failure
  - Explicit integration touch: adds only its own three dispatch arms to the entry point (single lines; no other entry-point change)
  - Observable: `status demo` over the engine's minimal fixture tree prints the engine's status golden text; `graph demo --table` prints a table and no fence; a spec without a tasks document fails with the IDE's message and exit 2
  - _Boundary: ViewCommands_
  - _Depends: 2.1, 2.2_
  - _Requirements: 6.5, 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 4.2 Implement the vocabulary validation command
  - Without a file: the project vocabulary through a workspace and the engine loader; with a file: the operator's file through the engine's parse with the path as given; the vocabulary report with status, topic and term counts and findings in text or JSON; exit 1 when the status is missing or any finding meets the fail-on level, 2 when an explicit file cannot be read, else 0
  - Explicit integration touch: adds only its own dispatch arm to the entry point (after 4.1's arms; not parallel with 4.1 because both edit the entry point)
  - Observable: validating the project's seed vocabulary exits 0 with "loaded" and counts; validating an engine semantic fixture lists its findings and exits 1; a missing default vocabulary prints the missing finding and exits 1
  - _Boundary: VocabCommands_
  - _Depends: 2.1, 2.2, 3.1, 4.1_
  - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [ ] 5. Importer
- [ ] 5.1 Implement the importer model, id rule, deduplication and TOML writer
  - The imported-term model built on the engine's kind type with provenance comment lines and a source reference; the id derivation rule (CamelCase and separators to hyphens, lower-case, non-id characters to hyphens, runs collapsed, ends trimmed) satisfying the mapping's three examples and Auto Ontology name forms; first-wins deduplication with the duplicate finding naming both sources; the TOML writer producing the design's header, schema version, topic tables with optional labels, and term tables sorted by id with the fixed key order and omission rules, using `toml_edit` / `toml` value formatting (or a basic-string escaper backed by it, since `toml_edit` is already in `Cargo.lock`); the input-format enum is reused from the command-line module
  - Observable: unit tests cover the id table, escaping, key order and omission, deduplication, and that every rendered id satisfies the id rule
  - _Boundary: Importer_
  - _Depends: 1.2, 2.2_
  - _Requirements: 9.2, 9.10, 9.11, 9.12_

- [ ] 5.2 Implement the append-only merge, output validation and reader dispatch
  - The merge that keeps the existing text unchanged, appends a merge header, only the terms whose id and labels do not resolve in the existing vocabulary through its scoped public queries, and topic tables only when the existing vocabulary has no declared table for them (using the same TOML table rendering as `render` or `toml_edit`), reporting each dropped term as a merge conflict; output validation through the engine's vocabulary parse of the exact text to be written; the format dispatch to the two readers, created here as minimal stub modules returning an empty import so that each reader task replaces only its own file
  - Observable: unit tests cover the two merge conflict rules and the topic-table rule on a constructed vocabulary, the unchanged prefix, and that a writer defect producing invalid TOML surfaces as the engine's invalid finding
  - _Boundary: Importer_
  - _Depends: 5.1_
  - _Requirements: 9.5_

- [ ] 5.3 Implement the Auto Ontology JSON reader
  - Lenient reading of each input as the terms page object or a bare array (anything else is an invalid-input finding with the parser's line when it is a syntax error); per item the trimmed name as label and id source (absent, null or blank → skipped with a finding naming the item's id or position), description as definition (empty when absent), synonyms as sorted alternative labels deduplicated after label normalisation and without the label itself, a provenance comment with the Auto Ontology id and zone names (object name, else id, or string), a related-terms comment when present (never a broader relation), every other property ignored, the default topic and kind on every term
  - Runs after 5.2 (replaces its stub module); 5.4 may run concurrently with this task
  - Observable: unit tests cover page versus array versus other top-level shapes, the null and blank name cases, synonym handling, zone and related comment lines for object and string forms, ignored properties, and the line in the invalid-input finding
  - _Boundary: AutoOntologyReader_
  - _Depends: 5.2_
  - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7_

- [ ] 5.4 (P) Implement the SKOS Turtle reader
  - Turtle parsing with the first syntax error reported as an invalid-input finding with its line; concepts in first-occurrence order; ids from the IRI local name through the shared id rule (invalid → skipped with a finding); preferred label and definition by the literal preference rule (untagged, English, first); alternative labels; broader from both directions; deprecation and replacement; the project namespace annotations for topic, kind, enum values and integer bounds, with an unrecognised kind kept as default plus a comment; non-concept subjects, collections and blank nodes ignored; the single concept scheme's title as the default topic's label
  - Observable: the Turtle fixture yields the three mapping-example ids, the expected labels and definitions per language rule, merged broader lists, the deprecated term with its replacement, the annotated topic, enum and int kinds, and the scheme title; the invalid fixture yields the finding with its line
  - _Boundary: SkosReader_
  - _Depends: 5.2_
  - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.5, 11.6, 11.7_

- [ ] 5.5 Implement the import command
  - The pipeline: read the operator's input files as given, run the chosen reader (an invalid-input finding is printed and the command fails with exit 2, nothing written), deduplicate, resolve the output path (default under the project root; dash means standard output), refuse an existing output without merge (exit 2), merge into an existing valid vocabulary (an invalid one fails with its finding line and exit 2), validate the exact output text, write the file or print it, print all import and validation findings in the finding line form on standard error, exit 1 when any is an error and 0 otherwise; under verbose, the written output path is printed on standard error
  - Explicit integration touch: adds only its own dispatch arm to the entry point
  - Observable: a fresh import of the page fixture writes the file and prints the two expected warnings; a second run without merge exits 2 and leaves the file unchanged; a merge into the merge-target fixture appends only new tables and reports two conflicts; `--out -` prints TOML on standard output only
  - _Boundary: VocabCommands_
  - _Depends: 2.1, 4.2, 5.3, 5.4_
  - _Requirements: 2.3, 2.5, 9.1, 9.3, 9.4, 9.6, 9.7, 9.8, 9.9_

- [ ] 6. Validation: binary-level integration tests
- [ ] 6.1 Build the test harness
  - A shared helper that copies a fixture tree into a temporary directory and runs the built binary capturing status, standard output and standard error, with paths to the engine's check and minimal fixture trees and golden files; the source-grep test asserting the standard filesystem API appears only in the adapter module
  - Observable: a smoke run of the binary over the known-good tree through the helper exits 0, and the grep test passes
  - _Boundary: CliTests_
  - _Depends: 3.2, 1.3_
  - _Requirements: 13.3, 13.4_

- [ ] 6.2 Write the check gate tests
  - Scenarios: known-good tree exits 0 with the passing summary; known-bad tree exits 1 with contradiction lines; warnings-only tree exits 0 by default and 1 under fail-on warning; missing root, unknown feature, and no or double selection exit 2 with their messages; a severity override changes the verdict; a check selection shows the skipped status line; JSON output parses with the schema version, the documented keys and finding fields in order and no timestamp; the vocabulary-findings tree with both specs prints the vocabulary warning exactly once; two runs are byte-equal; verbose prints a failed-read line for a missing document and nothing without the flag
  - Observable: all scenarios pass on the development platform and the suite is platform-neutral (forward slashes, newline endings)
  - _Boundary: CliTests_
  - _Depends: 6.1_
  - _Requirements: 3.3, 3.9, 4.4, 4.5, 5.3, 6.1, 6.2, 13.4_

- [ ] 6.3 (P) Write the view parity tests
  - Status, graph (diagram and table) and tasks (diagram and table) over the engine's minimal tree compared byte for byte with the engine's golden files plus the trailing-newline rule; the superseded `old` spec's status compared with the engine's superseded status golden (the chain line); the two error messages and exit 2; an undiscovered spec exits 2 naming the discovered ones
  - Observable: every comparison passes without normalisation
  - _Boundary: CliTests_
  - _Depends: 4.1, 6.1_
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [ ] 6.4 (P) Write the vocabulary and importer tests
  - Validate: the seed vocabulary exits 0; an engine semantic fixture exits 1 listing its findings; missing default exits 1; unreadable explicit file exits 2; JSON form parses
  - Import: page and array fixtures produce byte-equal TOML (asserted in-test, no committed goldens); the detail fixture yields the related comment; the skipped and duplicate findings appear; fresh write, refused overwrite, merge with an unchanged prefix and two conflicts, standard-output mode with findings on standard error; the Turtle fixture produces the mapping-example ids, annotations and scheme title; invalid JSON and Turtle exit 2 with the line; the written files load through `vocab validate` with only the expected findings
  - Observable: all scenarios pass and no file outside the temporary directories is created
  - _Boundary: CliTests_
  - _Depends: 5.5, 6.1_
  - _Requirements: 8.1, 8.2, 8.3, 8.4, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 9.10, 9.11, 10.1, 10.2, 10.4, 10.5, 11.2, 11.6, 11.7, 13.5_

- [ ] 6.5 (P) Write the self-tree determinism test and the performance test
  - The check over this repository's own kiro tree exits 0 or 1 but never 2 and produces byte-identical output on two runs; a generated tree of twenty specs with documents near 100 KiB checks in under five seconds, asserted only in release builds
  - Observable: both tests pass; the timing test is skipped in debug builds with a visible message
  - _Boundary: CliTests_
  - _Depends: 6.1_
  - _Requirements: 13.6, 13.7_

- [ ] 7. Integration: documentation, upstream reconciliation, final gate
- [ ] 7.1 Write the CI and command reference documentation
  - The CI document with a GitHub Actions job and a generic shell snippet that install the `wasm32-wasip2` target and `cargo-nextest` (`cargo install cargo-nextest --locked`), run the native tests in parallel with `cargo nextest run` plus `cargo test --doc`, run `cargo deny check` with `wasm32-wasip2` in `[graph] targets`, build the plugin component for `wasm32-wasip2`, build the CLI and run the gate with fail-on error; the exit-code table and a JSON example with a one-line query; the command reference derived from the command-line definitions with every option, default and exit code, one example per command, the note that the table flag equals the plugin's text option, and the standard-error rule of the importer (amended 2026-11-04: the CI steps above match requirement 12.1 as amended, including the nextest install step)
  - Explicit touches outside the docs boundary: the crate README pointing at both documents (CliScaffold) and one test comparing the help output's option names with the reference (CliTests)
  - Observable: every option named in the reference exists in `kiro-cli <command> --help`, checked by that test
  - _Boundary: Docs_
  - _Depends: 5.5, 6.1_
  - _Requirements: 12.1, 12.2_

- [ ] 7.2 (P) Reconcile the vocabulary documentation and verify upstream alignment
  - Explicit integration touch outside this crate: in `docs/vocabulary.md` ensure the Auto Ontology import procedure documents the verified CLI flags and examples (`kiro-cli vocab import <export.json> --format auto-ontology-json --topic <t>`, the verified page shape and term fields, related terms only in the detail shape, what is mapped, what remains maintainer curation — topics, hierarchy, kinds — and SKOS as the secondary input), leaving the SKOS mapping table unchanged; verify that `.kiro/specs/vocabulary-grounding/requirements.md` (10.1) and `.kiro/specs/vocabulary-grounding/design.md` (Non-Goals, VocabDocs, Supporting References) already name `kiro-cli vocab import --format auto-ontology-json` (applied during cross-spec review)
  - Observable: `docs/vocabulary.md` documents the verified `kiro-cli vocab import` flags and examples, the SKOS mapping table is byte-identical to the design's table, and the vocabulary-grounding requirements and design documents match
  - _Boundary: Docs_
  - _Depends: 5.5_
  - _Requirements: 12.3_

- [ ] 7.3 Run the final quality gate
  - Workspace-wide clippy with warnings denied, formatting check, the full test suite, and the engine's sandbox-target build; confirm by review that the CLI contains no parser, check or renderer and that every printed finding code is in the engine catalogue (the catalogue test and the source grep test from 6.1 are the evidence)
  - Observable: all commands pass on a clean checkout and the gate run over this repository (`check --project . --all`) matches the self-tree test's outcome
  - _Boundary: CliScaffold_
  - _Depends: 6.2, 6.3, 6.4, 6.5, 7.1, 7.2_
  - _Requirements: 13.1, 13.3_

- [ ] 7.4 Append the plugin wasm build and kiro-cli gate steps to the CI config (amendment 2026-11-04, gap-analysis arbitration; amended 2026-11-08)
  - Explicit integration touches outside this crate: add `kiro-check = "run -p kiro-cli -- check --project . --all --fail-on error"` to `[alias]` in the root `.cargo/config.toml` alongside `gate = "nextest run"` (since Cargo `[alias]` entries map to a single Cargo subcommand and cannot chain multiple commands), and append the standalone plugin checks and `wasm32-wasip2` build steps (`cargo clippy --manifest-path plugins/kiro/Cargo.toml --all-targets -- -D warnings`, `cargo test --manifest-path plugins/kiro/Cargo.toml`, `cargo deny --manifest-path plugins/kiro/Cargo.toml check`, `cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2`) plus `cargo kiro-check` (`cargo run -p kiro-cli -- check --project . --all --fail-on error`) to the CI job in `.github/workflows/ci.yml` (owned by kiro-core task 5.5, CrateScaffold — neither `plugins/kiro` nor `kiro-cli` exists in Wave 1); extend the CI document of task 7.1 (Docs) with the final gate line
  - Observable: after this task, `cargo kiro-check` runs the `kiro-cli check` gate, and the CI job lints/tests/audits/builds the plugin component and runs `kiro-cli check`; the CI document's shell snippet matches
  - _Boundary: CliScaffold, Docs_
  - _Depends: 7.1, 7.3_
  - _Requirements: 12.1, 13.1_
