# Kiro CLI Command Reference

`kiro-cli` provides native command-line validation, visualization, and ontology import tools for Kiro / cc-sdd workspaces.

---

## Global Options

The following options apply to all subcommands and can be placed before or after the command name:

- `--project <PATH>` (alias `--root`): Path to the workspace root containing the `.kiro/` directory (default: `.`).
- `--verbose`, `-v`: Enable detailed diagnostics on standard error (e.g., individual file read refusals or failures). May be repeated.
- `--quiet`, `-q`: Suppress diagnostics on standard error. Cannot be combined with `--verbose`.
- `--color <auto|always|never>`: Colour configuration (default: `auto`).

### Environment fallbacks

A shared parameter is taken from the command line first, then from its environment variable, then from the built-in default.

| Variable | Parameter | Applies to |
|----------|-----------|------------|
| `KIRO_PROJECT` | `--project` | all commands |
| `KIRO_FORMAT` | `--format` (`text` or `json`) | `check`, `code check`, `telemetry export` |
| `KIRO_FAIL_ON` | `--fail-on` (`error` or `warning`) | `check`, `code check` |
| `NO_COLOR` | `--color never` when set and not empty | all commands |
| `CLICOLOR_FORCE` | `--color always` when set and not `0`; `NO_COLOR` wins | all commands |

An invalid value in `KIRO_FORMAT` or `KIRO_FAIL_ON` is a failure (exit code 2) that names the variable. The other commands will follow as they are moved onto the shared parameter library.

---

## Exit Codes

All commands return standard process exit codes:

| Code | Status | Meaning |
|:----:|--------|---------|
| `0` | Clean | Command succeeded; validation passed without findings meeting or exceeding `--fail-on`. |
| `1` | Findings | Validation completed with findings meeting or exceeding the `--fail-on` severity threshold. |
| `2` | Failure | Process or syntax failure; invalid arguments, unresolvable spec selection, unreadable file, or IO error. |

---

## Subcommands

### `check`

Validate specifications against structural constraints, EARS requirements syntax, vocabulary grounding, and logical contradictions.

#### Options

- `--feature <NAME>`: Feature specification to check. May be repeated to check multiple features.
- `--all`: Check all discovered specifications under `.kiro/specs/`. (Mutually exclusive with `--feature`).
- `--format <text|json>`: Report output format: `text` (human-readable) or `json` (machine-readable). (Default: `text`).
- `--output <PATH>`: Write the report to this file instead of standard output; nothing of the report is then printed. `-` means standard output. A file that cannot be written is a failure (exit code 2).
- `--fail-on <error|warning>`: Severity threshold that triggers exit code 1. (Default: `error`).
- `--telemetry-dir <DIR>`: Also append the telemetry of this run to files in this directory, which must lie outside the project. Falls back to `KIRO_TELEMETRY_DIR`. The report and the exit code are unaffected.
- `--checks <ears,terms,contradictions>`: Comma-delimited list of check kinds to run. (Default: all enabled).
- `--patterns`: Report recognized EARS statement patterns (`ubiquitous`, `event-driven`, etc.) as info findings.
- `--no-suggest`: Suppress suggestions for undeclared vocabulary terms.
- `--max-variables <N>`: Maximum number of variables considered during pairwise contradiction analysis.
- `--max-nodes <N>`: Maximum BDD search node cap during contradiction analysis.
- `--severity <CODE=LEVEL>`: Override the default severity of a check finding code (e.g., `--severity EARS_NO_SHALL=info`). May be repeated.

#### Example

```bash
kiro-cli check --all --fail-on warning --checks ears,terms
```

---

### `status`

Print the phase lifecycle, approval statuses, and superseded successor chain for a specification.

#### Arguments

- `<SPEC>`: Name of the specification feature to inspect.

#### Example

```bash
kiro-cli status cli-gate
```

---

### `graph`

Print the requirement dependency and traceability graph for a specification.

#### Arguments & Options

- `<SPEC>`: Name of the specification feature.
- `--table`: Render a Markdown table instead of a Mermaid diagram. (Note: The `--table` flag corresponds directly to the Kiro plugin's `text` format option).

#### Example

```bash
kiro-cli graph cli-gate --table
```

---

### `tasks`

Print the implementation task graph, dependencies, and execution readiness for a specification.

#### Arguments & Options

- `<SPEC>`: Name of the specification feature.
- `--table`: Render a Markdown table instead of a Mermaid diagram. (Note: The `--table` flag corresponds directly to the Kiro plugin's `text` format option).

#### Example

```bash
kiro-cli tasks cli-gate
```

---

### `code check`

Measure source files and judge them against the code limits: complexity, nesting, size and parameters of routines, and the size of files and types. Every finding names the spec, component and tasks that own the file and an edit that resolves it. See [Code rules](code-rules.md).

Exactly one selection is required.

#### Options

- `--task <SPEC:ID>`: Judge the files of the components the task names in its boundary, for example `--task code-constraints:3.1`.
- `--spec <NAME>`: Judge the files the spec maps in its File Structure Plan.
- `--files <PATH>...`: Judge these project-relative files.
- `--all`: Judge every file a measurement profile covers.
- `--changed <REV>`: Judge the tracked files that differ from this revision and compare them with it (the ratchet).
- `--summary`: Print the hook summary instead of the text report: at most 25 lines of plain ASCII.
- `--telemetry-dir <DIR>`: Also append the telemetry of this run (code metric points, finding records with hint and owners, the run trace) to files in this directory outside the project. Falls back to `KIRO_TELEMETRY_DIR`. The report and the exit code are unaffected.
- `--show-highest`: Also print the highest value of every metric, for a first look at a grown repository.
- `--format <text|json>`: Report output format. (Default: `text`).
- `--output <PATH>`: Write the report to this file instead of standard output.
- `--fail-on <error|warning>`: Severity threshold that triggers exit code 1. (Default: `error`). The routine limits are errors in Rust and Python; every other limit is a warning, so use `--fail-on warning` to block on those too.

#### Example

```bash
kiro-cli code check --task my-feature:2.1 --fail-on warning
```

---

### `code baseline`

Measure the whole tree and write `.kiro/code-baseline.json`, the side `code check` compares with when no revision is named. The same tree gives the same bytes. Refuses when a file does not parse. See [Code rules](code-rules.md#the-ratchet-only-regression-blocks).

---

### `code config`

Print every effective code limit, scope and ignore with its origin and reason, and the digest of the effective configuration. An invalid `.kiro/code-rules.toml` is a failure (exit code 2) that lists every problem. See [Code rules](code-rules.md#configuration).

---

### `telemetry export`

Build the metric snapshot of the selected specs: phase, approvals, task progress, findings by code, term and contradiction counts, and traceability gaps. The metrics are listed in the [telemetry catalogue](telemetry.md).

With `--telemetry-dir` it appends the run as OpenTelemetry JSON lines to three files in that directory for a Collector to ship; see [Telemetry](telemetry.md). With `--dry-run` it prints the snapshot and writes nothing. One of the two is required.

#### Options

- `--feature <NAME>`: Spec to include. May be repeated.
- `--all`: Include all discovered specs. (Mutually exclusive with `--feature`).
- `--dry-run`: Print the snapshot and write nothing.
- `--format <text|json>`: Output format. (Default: `text`).
- `--output <PATH>`: With `--dry-run`, write the snapshot to this file instead of standard output.
- `--telemetry-dir <DIR>`: Directory outside the project to append `metrics.jsonl`, `logs.jsonl` and `traces.jsonl` to. Falls back to `KIRO_TELEMETRY_DIR`.

#### Example

```bash
kiro-cli telemetry export --all --dry-run --format json
```

---

### `vocab validate`

Check the syntax, schema compliance, ID validity, and semantic integrity of `.kiro/vocabulary.toml`.

#### Arguments & Options

- `[FILE]`: Path to vocabulary file. Defaults to `<project-root>/.kiro/vocabulary.toml`.
- `--format <text|json>`: Report output format. (Default: `text`).
- `--fail-on <error|warning>`: Severity threshold triggering exit code 1. (Default: `error`).

#### Example

```bash
kiro-cli vocab validate .kiro/vocabulary.toml --fail-on error
```

---

### `vocab import`

Import domain terms from external ontology exports (Auto Ontology JSON or SKOS RDF Turtle) into TOML format.

#### Arguments & Options

- `<FILES...>`: One or more input file paths to import.
- `--format <auto-ontology-json|skos-turtle>`: Format of the input files:
  - `auto-ontology-json`: Auto Ontology `/api/terms` JSON page object or bare array.
  - `skos-turtle`: W3C SKOS RDF Turtle document.
- `--topic <TOPIC>`: Target topic identifier for imported terms.
- `--kind <entity|bool|enum|int>`: Default term kind. (Default: `entity`).
- `--out <PATH>`: Output file destination. Defaults to `<project-root>/.kiro/vocabulary.toml`. Specify `-` to output to stdout.
- `--merge`: Append non-conflicting terms to an existing vocabulary file instead of writing a fresh file.

#### Standard Error Output Rule
When `--out -` is specified, generated TOML is printed directly to standard output (`stdout`), while all diagnostic findings (such as skipped terms or duplicates) are printed strictly to standard error (`stderr`).

#### Example

```bash
kiro-cli vocab import export.json --format auto-ontology-json --topic sdd --merge
```
