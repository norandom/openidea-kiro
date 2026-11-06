# Requirements Document

## Introduction

cli-gate turns the v2 engine into a gate. Today every check lives inside the OpenIDEA plugin, where it is advice: a developer can run it or not, and nothing stops a merge. Version 1 (`spec-kit-ears-tdd`) earned its keep because `ears-sdd validate` ran in CI and blocked merges; v2 has no native frontend yet, so nothing can run the engine over a whole `.kiro` tree, in a pipeline, or do the things the plugin sandbox forbids — walking directories and converting a vocabulary export into the committed vocabulary file.

This spec delivers `kiro-cli`, a native command-line binary with five responsibilities: (1) a `check` command that discovers the specs of a project, runs exactly the spec-constraints checks the plugin runs and the engine's structural findings, and prints them as text (the same lines the IDE shows) or JSON (the engine's finding shape verbatim); (2) an exit-code contract (`0` clean, `1` findings at or above a chosen level, `2` usage or I/O error) that CI can rely on; (3) the three spec-visualization views (`status`, `graph`, `tasks`) printed natively, byte-identical to what the IDE inserts; (4) `vocab validate`, which runs the engine's vocabulary loader over a file; and (5) `vocab import`, the only tool in the project that converts an external vocabulary into `.kiro/vocabulary.toml` — primarily from NVIDIA Auto Ontology's JSON term export (`GET /api/terms`; verified against its OpenAPI description: a page object whose `terms` array holds objects with `id`, `name`, `description`, `synonyms`, `zones`; `related_terms` appears only in the per-term detail shape; there is no hierarchy and no topic) and secondarily from SKOS Turtle through vocabulary-grounding's normative mapping. The CLI holds no parsing, checking or rendering logic of its own; everything that is not I/O or argument handling is the engine's.

## Boundary Context

- **In scope**: the `kiro-cli` binary and its command line; project-root resolution and spec discovery over the filesystem; a root-confined read port for the engine; the `check` command, its options that configure the engine's checks, its text and JSON reports and its exit codes; the `status`, `graph` and `tasks` commands; `vocab validate`; `vocab import` for the Auto Ontology JSON and SKOS Turtle inputs, including the merge behaviour and the validation of the produced file; the importer's own finding codes; the CI documentation, the command reference and the reconciliation of the vocabulary documentation that currently says no import tool exists; integration tests over fixture trees in temporary directories, including the exit-code contract.
- **Out of scope**: any parser, check, view or renderer (kiro-core, spec-constraints, spec-visualization, vocabulary-grounding own them; a gap found there goes back to its spec); the plugin and its packaging; Spec Kit (`.specify`) layouts; running, deploying or calling Auto Ontology or any network service; editing spec, steering or vocabulary files other than writing the file named by `vocab import --out`; a configuration file for the CLI; colour or interactive terminal output; installing the CLI into CI images (the documentation shows how a pipeline builds and runs it).
- **Adjacent expectations**: kiro-core supplies the read port abstraction, the workspace, the finding shape with its JSON serialization and sort order, and the text renderer for findings; spec-constraints supplies the check entry point, its configuration with defaults and severity overrides, the check report with per-check status and summaries, and the exact wording of the IDE's check notification lines that the text report mirrors; vocabulary-grounding supplies the vocabulary loader and its findings, the TOML format, the id rule, the label normalisation and the normative SKOS → TOML mapping; spec-visualization supplies the three views and their text forms, which this CLI prints unchanged; plugin-core defines the `path:line: SEVERITY CODE message` line that both frontends print. The Auto Ontology export is a file the maintainer obtains outside this tool (with their API token) and hands to `vocab import`; topics, hierarchy and kinds are curated by maintainers in the TOML after import, because Auto Ontology supplies none of them.

## Requirements

### Requirement 1: Project Root and Spec Discovery
**Objective:** As a CI maintainer, I want the CLI to find a project's specs on its own, so that the gate covers every spec without a hand-maintained list.

#### Acceptance Criteria
1. The kiro-cli shall take the project root from `--project <dir>` (default: the current directory) on every command and shall resolve every project-relative path it reads — `.kiro/specs/…`, `.kiro/steering/roadmap.md`, `.kiro/vocabulary.toml` — under that root.
2. If the project root does not exist or is not a directory, the kiro-cli shall print one error line naming the path on standard error and exit with code 2.
3. When a command needs the set of specs, the kiro-cli shall discover one spec for every directory directly under `.kiro/specs/` that contains a `spec.json` or a `requirements.md`, ordered by directory name, with the directory name as the spec name.
4. The kiro-cli shall make both the discovered spec names and the names listed in the roadmap's specs section known to the engine, so that successor resolution and roadmap findings behave as in the IDE, while loading and checking only discovered specs.
5. When `check` is given one or more `--feature <name>` options, the kiro-cli shall check exactly those specs in the given order with duplicates removed; when given `--all`, every discovered spec in discovery order; if neither or both are given, the kiro-cli shall print a usage message and exit with code 2.
6. If `--feature` names a spec that was not discovered, the kiro-cli shall print an error naming it and the discovered spec names and exit with code 2 without checking anything.
7. If `.kiro/specs` does not exist under the root, or `--all` discovers no spec, the kiro-cli shall print an error naming the root and exit with code 2.

### Requirement 2: Confined Read Port
**Objective:** As a project maintainer, I want every read the engine makes to stay inside the project, so that a spec cannot make the gate read arbitrary files and CI sees exactly what the IDE sees.

#### Acceptance Criteria
1. When the engine asks for a project-relative path, the kiro-cli shall return the text of the file at that path under the project root, or nothing when the file is absent, is not a regular file, is not valid UTF-8, cannot be read, or is larger than 16 MiB — the same limits the IDE host applies to the plugin.
2. If the engine asks for an absolute path, a path with a `..` segment, or a path that would resolve outside the project root, the kiro-cli shall return nothing for it without touching the filesystem.
3. While running any command, the kiro-cli shall write no file other than the output file explicitly named by `vocab import --out`, and shall never modify a spec, steering or vocabulary file in place.
4. Where `--verbose` is given, the kiro-cli shall print each refused or failed engine read with its reason on standard error; otherwise it shall print nothing about reads.
5. The kiro-cli shall read a file named explicitly as a command argument (`vocab import <file>…`, `vocab validate <file>`) as given, relative to the current directory and without root confinement, because it is the operator's own argument and may live outside the project.

### Requirement 3: Check Command
**Objective:** As a reviewer, I want `kiro-cli check` to produce exactly the findings the IDE's check commands produce plus each spec's structural findings, so that CI blocks on the same facts the author saw while writing.

#### Acceptance Criteria
1. When `check` runs on a spec, the kiro-cli shall run the spec-constraints checks over that spec with the engine's default configuration unless an option below changes it, and shall report every finding of the resulting check report unchanged in code, severity, file, line and message.
2. When `check` runs on a spec, the kiro-cli shall also report the spec's own structural findings (missing documents, parse, link and successor findings) exactly as the engine produces them when loading the spec.
3. The kiro-cli shall report the vocabulary file's own findings and the roadmap's findings at most once per run, however many specs are checked.
4. Where `--checks <list>` is given with a comma-separated subset of `ears`, `terms` and `contradictions`, the kiro-cli shall enable only the listed checks; when absent, all three are enabled.
5. Where `--patterns` is given, the kiro-cli shall include the informational EARS pattern findings; when absent, it shall not.
6. Where `--no-suggest` is given, the kiro-cli shall disable term suggestions.
7. Where `--max-variables <n>` or `--max-nodes <n>` is given, the kiro-cli shall pass that cap to the contradiction search; when absent, the engine's defaults apply.
8. Where `--severity <CODE>=<error|warning|info>` is given (repeatable), the kiro-cli shall apply it as a severity override for that check code; if the code is not a spec-constraints check code or the level is not one of the three, the kiro-cli shall exit with code 2 naming the offending value.
9. The kiro-cli shall order the report deterministically — vocabulary findings, roadmap findings, then specs in the order checked, each with its findings in the engine's sort order — so that two runs over the same tree produce byte-identical output.

### Requirement 4: Text Report
**Objective:** As a developer reading CI logs, I want the text report to say what the IDE said, so that I recognise a finding without translating between tools.

#### Acceptance Criteria
1. Where the report format is `text` (the default), the kiro-cli shall print each finding as the engine's text renderer prints it (`<file>:<line>: <SEVERITY> <CODE> <message>`, `:<line>` omitted when there is no line), the same line the IDE notification shows.
2. For each checked spec, the kiro-cli shall print a scope line and per-check status lines whose text equals the scope line and status lines of the IDE's `check.spec` notification for that spec (spec name, declared topics, vocabulary status; for each check its summary figures or its skip reason), followed by that spec's findings or the line `No findings.`
3. The kiro-cli shall end the text report with one summary line stating the number of specs checked, the counts of errors, warnings and info findings, the effective `--fail-on` level and the verdict `pass` or `fail`.
4. The kiro-cli shall print reports on standard output and diagnostics and usage errors on standard error, with no colour or terminal control sequences and no timestamps.
5. The kiro-cli shall print every report with `\n` line endings and project-relative paths with `/` separators on every platform, so that the same tree yields the same bytes on Windows and Linux.

### Requirement 5: JSON Report
**Objective:** As a CI author, I want a machine-readable report, so that a pipeline can annotate pull requests or trend counts without parsing prose.

#### Acceptance Criteria
1. Where `--format json` is given to `check`, the kiro-cli shall print one JSON document on standard output containing a report schema version, the list of spec names checked, each spec's declared topics and per-check status, the summary counts of errors, warnings and info, the fail-on level, the verdict, and the list of findings.
2. The kiro-cli shall serialize each finding in the JSON report exactly as the engine's finding type serializes (`severity`, `code`, `feature`, `file`, `line`, `message`, in that order), adding, removing or renaming nothing.
3. The JSON report shall contain no timestamp, absolute path, host name or other environment-dependent value, so that two runs over the same tree are byte-identical.
4. If the report cannot be written to standard output, the kiro-cli shall exit with code 2.

### Requirement 6: Exit Code Contract
**Objective:** As a CI maintainer, I want exit codes that each mean one thing, so that the gate blocks merges on findings and surfaces misconfiguration separately.

#### Acceptance Criteria
1. When `check` completes and no reported finding has a severity at or above the `--fail-on` level, the kiro-cli shall exit with code 0.
2. When `check` completes and at least one reported finding has a severity at or above the `--fail-on` level, the kiro-cli shall print the full report and exit with code 1.
3. The kiro-cli shall accept `--fail-on error` (the default), under which only errors fail the gate, and `--fail-on warning`, under which warnings and errors fail the gate; any other value exits with code 2.
4. If the command line is invalid, an option value is out of range, the project root or the spec selection is unusable, an explicit input file cannot be read or parsed, or an output file cannot be written or would be overwritten without `--merge`, the kiro-cli shall print one explanatory line on standard error, print no report, and exit with code 2.
5. When `status`, `graph` or `tasks` prints a view, the kiro-cli shall exit with code 0; when the view cannot be built, it shall exit with code 2.
6. The kiro-cli shall never terminate by panic; internal failures surface as code-2 error lines, and the crate forbids the panicking idioms the engine forbids.

### Requirement 7: Visualization Views
**Objective:** As a developer without the IDE open, I want the status, spec graph and task graph of a spec as text identical to what the IDE inserts, so that reviews and documentation use one rendering.

#### Acceptance Criteria
1. When `status <spec>` runs, the kiro-cli shall print the spec-visualization status text of that spec, byte-identical to the text the IDE's `viz.status` command returns for the same tree, followed by a newline.
2. When `graph <spec>` runs, the kiro-cli shall print the spec graph (summary lines and fenced Mermaid diagram) byte-identical to the text `viz.graph` returns; where `--table` is given, the table form byte-identical to the text `viz.graph <spec> text` returns.
3. When `tasks <spec>` runs, the kiro-cli shall print the task graph byte-identical to the text `viz.tasks` returns, with `--table` selecting the table form as for `graph`.
4. The kiro-cli shall label components in the views exactly as the IDE does (raw design component names), so that the two frontends never disagree on a label.
5. If the named spec was not discovered, the kiro-cli shall exit with code 2 naming the discovered specs; if the spec has none of the documents the view needs, the kiro-cli shall print the same error message the IDE shows for that case and exit with code 2.

### Requirement 8: Vocabulary Validation
**Objective:** As a vocabulary maintainer, I want to validate a vocabulary file from the command line with the engine's own loader, so that a pull request touching the vocabulary is checked before the IDE ever loads it.

#### Acceptance Criteria
1. When `vocab validate` runs without a file argument, the kiro-cli shall load `.kiro/vocabulary.toml` under the project root through the engine's vocabulary loader and print its status (`loaded`, `missing` or `invalid`), the topic and term counts when loaded, and its findings as a text or JSON report.
2. When `vocab validate <file>` runs, the kiro-cli shall validate that file's text with the same loader and report its findings with the file path as given, `/`-separated.
3. When the vocabulary loads, the kiro-cli shall exit with code 0 if no finding is at or above `--fail-on` (default `error`) and with code 1 otherwise.
4. If the vocabulary is invalid, the kiro-cli shall print the `VOCAB_INVALID` finding with its line and exit with code 1; if the default file is missing, it shall print the `VOCAB_MISSING` finding and exit with code 1, because the operator asked to validate a file that is not there; if an explicit file cannot be read, it shall exit with code 2.

### Requirement 9: Vocabulary Import — Common Behaviour
**Objective:** As a vocabulary maintainer, I want one importer that turns an external vocabulary into the committed TOML file and tells me what it could not map, so that seeding `.kiro/vocabulary.toml` is a repeatable step rather than a hand edit.

#### Acceptance Criteria
1. The kiro-cli shall accept `vocab import <file>… --format auto-ontology-json|skos-turtle --topic <id> [--kind entity|bool|enum|int] [--out <path>] [--merge]`, requiring `--format` and `--topic`, requiring `--topic` to satisfy the vocabulary id rule, defaulting `--kind` to `entity`, and reading one or more input files of the same format in the given order.
2. The kiro-cli shall produce one vocabulary file in vocabulary-grounding's TOML format: a header comment naming the input format and input file names, `schema_version = "2"`, one `[topics.<id>]` table per topic that occurs, then `[terms.<id>]` tables sorted by id, each carrying `topic`, `label`, `kind` and `definition` always and `alt_labels`, `broader`, `deprecated`, `replaced_by`, `values`, `min` and `max` only when they have a value — identical output for identical input.
3. The kiro-cli shall default `--out` to `.kiro/vocabulary.toml` under the project root, shall use another path as given, and shall treat `--out -` as "print the file to standard output and write nothing".
4. If the `--out` file already exists and `--merge` is not given, the kiro-cli shall refuse, name the file, write nothing and exit with code 2.
5. Where `--merge` is given and the `--out` file exists, the kiro-cli shall keep the existing file's text unchanged, including its comments, existing terms and definitions, shall append only the imported terms whose id does not already exist, shall append a `[topics.<id>]` table for each imported topic the file lacks, and shall report as a merge conflict — keeping the existing term and not writing the imported one — every imported term whose id already exists or whose label or alternative label equals, after label normalisation, a label or alternative label of an existing term of the same topic.
6. Where `--merge` is given and the `--out` file does not exist, the kiro-cli shall behave as a fresh import.
7. After producing the output, the kiro-cli shall validate the complete output text (the merged file when merging) with the engine's vocabulary loader and print the loader's findings after the import findings, so that the maintainer sees before committing what the IDE will report.
8. The kiro-cli shall print import findings and output-validation findings in the finding line form on standard error, so that `--out -` leaves standard output to the TOML text alone.
9. When the output was produced and no import or validation finding is an error, the kiro-cli shall exit with code 0; when it was produced but an error-level finding exists, with code 1; if an input file cannot be read or parsed in the chosen format, the kiro-cli shall print one finding naming the file and, when known, the line of the first problem, write nothing, and exit with code 2.
10. If an imported term's derived id is empty or invalid, the kiro-cli shall skip the term and report a finding naming the source term; if two imported terms derive the same id, the first in input order wins and the kiro-cli shall report a finding naming both.
11. If an imported term has no definition, the kiro-cli shall still emit the term with an empty definition, so that the output validation reports it as the engine does (`TERM_NO_DEFINITION`) rather than the importer inventing text.
12. The kiro-cli shall give every imported term the `--topic` topic and the `--kind` kind unless the input itself carries a topic or kind annotation the format mapping recognises.

### Requirement 10: Auto Ontology JSON Import
**Objective:** As a vocabulary maintainer, I want to import the JSON that Auto Ontology actually exports, so that the project's vocabulary can be seeded from the organisation's source of truth without an intermediate hand conversion.

#### Acceptance Criteria
1. Where the format is `auto-ontology-json`, the kiro-cli shall accept each input file either as the `GET /api/terms` page object (an object whose `terms` property is an array of term objects) or as a bare JSON array of term objects, and shall treat any other JSON shape as unparsable.
2. For each term object, the kiro-cli shall take the trimmed `name` as the label and derive the id from it by the vocabulary id rule (lower-case, CamelCase and spaces to hyphens, any other character to a hyphen, runs collapsed, leading and trailing hyphens removed); if `name` is absent, null or blank, it shall skip the term with a finding naming the object's `id` or its position.
3. The kiro-cli shall map `description` to the definition (absent or null → empty) and `synonyms` to alternative labels (strings trimmed, blanks dropped, duplicates and the label itself removed after label normalisation, sorted).
4. The kiro-cli shall not map `zones` to a topic: it shall record the zone names (each zone object's `name`, else its `id`; or the string itself) in a comment line above the term, together with the Auto Ontology `id`, so that provenance survives without affecting resolution.
5. The kiro-cli shall never map `related_terms` to `broader`: where a term object carries `related_terms` (the per-term detail shape), it shall record their names (object `name`, else `id`; or the string itself) in a `# related:` comment line above the term, so that a maintainer can promote a relation to `broader` by hand.
6. The kiro-cli shall ignore every other property of a term object (`tags`, `certification`, `owner_id`, `schema_names`, certification flags and any unknown property) without a finding.
7. The kiro-cli shall assign the `--topic` topic and the `--kind` kind to every imported term, because this input carries neither.

### Requirement 11: SKOS Turtle Import
**Objective:** As a vocabulary maintainer with a SKOS vocabulary from another tool, I want the same importer to read Turtle, so that the TOML file has one generic standards-based input besides the Auto Ontology export.

#### Acceptance Criteria
1. Where the format is `skos-turtle`, the kiro-cli shall parse each input file as RDF Turtle and shall treat a syntax error as unparsable, reporting the line of the first error.
2. The kiro-cli shall produce one term for every subject typed `skos:Concept`, deriving its id from the IRI's local name (the part after `#`, else after the last `/`) by the same rule as Requirement 10.2 with CamelCase split into hyphenated words, and shall skip with a finding any concept whose derived id is empty or invalid.
3. The kiro-cli shall take `label` from `skos:prefLabel` and `definition` from `skos:definition`, preferring a literal without language tag, else `@en`, else the first in document order; an absent label becomes the id and an absent definition becomes empty.
4. The kiro-cli shall map each `skos:altLabel` to an alternative label (sorted, deduplicated) and the union of `skos:broader` targets and inverse `skos:narrower` sources to `broader` (target ids derived by the same rule, sorted, deduplicated), leaving targets that are not concepts in the file for the output validation to report.
5. The kiro-cli shall map `owl:deprecated "true"` to `deprecated = true` and `dct:isReplacedBy` to `replaced_by` with the target's derived id.
6. The kiro-cli shall honour the `kiro:` annotations (`kiro:topic` overriding `--topic`, `kiro:kind` overriding `--kind`, `kiro:value` literals as enum values in document order, `kiro:min` and `kiro:max` integers) with the namespace vocabulary-grounding defines.
7. The kiro-cli shall ignore every subject that is not a `skos:Concept` (schemes, collections, blank nodes) and shall take a `[topics.<id>]` label from the concept scheme's `skos:prefLabel` or `dct:title` when the file declares exactly one scheme.

### Requirement 12: Documentation
**Objective:** As a team adopting the gate, I want the CI usage, the command reference and the vocabulary procedure documented in one place each, so that a pipeline and a vocabulary seed can be set up from the docs alone.

#### Acceptance Criteria
1. The kiro-cli documentation shall include a CI document with a GitHub Actions job and a generic shell snippet that install the `wasm32-wasip2` target and `cargo-nextest` (`cargo install cargo-nextest --locked`), run the native tests in parallel with `cargo nextest run` plus `cargo test --doc`, run `cargo deny check` with `wasm32-wasip2` in `[graph] targets`, build the plugin component for `wasm32-wasip2`, build the CLI and run `check --project . --all --fail-on error`; the document also includes the exit-code table and a JSON report example.
2. The kiro-cli documentation shall include a command reference listing every command, option, default and exit code, with one example per command.
3. The vocabulary documentation shall be reconciled: the statement that converting Auto Ontology JSON has no tool in this project shall be replaced by the `vocab import --format auto-ontology-json` procedure, the verified export shape, what the importer maps, and what remains a maintainer's curation (topics, hierarchy, kinds); the SKOS → TOML mapping table shall remain unchanged and normative.

### Requirement 13: Quality and Tests
**Objective:** As a maintainer, I want the CLI to be as total and deterministic as the engine and to prove its exit codes, so that a green gate means something.

#### Acceptance Criteria
1. The kiro-cli shall be a native-only crate of the root workspace with the engine's lint table (unsafe code forbidden; unwrap, expect, panic and indexing panics denied), passing clippy with warnings denied and the formatting check.
2. The kiro-cli shall emit no finding whose code is outside the engine's finding catalogue; the importer's own codes are added to that catalogue so that the engine's catalogue test covers them.
3. The kiro-cli shall contain no parsing, checking or rendering logic: every finding of `check`, `status`, `graph`, `tasks` and `vocab validate`, and every view text, originates in the engine, and the importer's output format is produced from the engine's vocabulary model types.
4. The kiro-cli tests shall run the built binary over fixture trees copied into temporary directories and assert standard output, standard error and exit code for: a clean tree (0), a tree with errors (1), a warnings-only tree with `--fail-on warning` (1) and with the default (0), a missing root (2), an unknown feature (2), no selection (2), the JSON report shape, and byte-identical output across two runs.
5. The kiro-cli tests shall cover the read port's confinement (absolute, `..`, outside-root and oversized paths return nothing without a read) and the importer over fixtures for both formats, including the page object and bare array shapes, a term without name, duplicate ids, `related_terms`, `zones`, a fresh import, a refused overwrite, a merge with conflicts, and `--out -`.
6. When run over this repository's own `.kiro` tree, `check --all` shall exit with code 0 or 1, never 2, and produce byte-identical output across two runs.
7. The kiro-cli shall check a tree of twenty specs of typical size (up to 100 KiB per document) with the default configuration in under 5 seconds in a native release build.
