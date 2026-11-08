# Kiro CLI Command Reference

`kiro-cli` provides native command-line validation, visualization, and ontology import tools for Kiro / cc-sdd workspaces.

---

## Global Options

The following options apply to all subcommands and can be placed before or after the command name:

- `--project <PATH>`: Path to the workspace root containing the `.kiro/` directory (default: `.`).
- `--verbose`: Enable detailed diagnostics on standard error (e.g., individual file read refusals or failures).

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
- `--fail-on <error|warning|info|never>`: Severity threshold that triggers exit code 1. (Default: `error`).
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

### `vocab validate`

Check the syntax, schema compliance, ID validity, and semantic integrity of `.kiro/vocabulary.toml`.

#### Arguments & Options

- `[FILE]`: Path to vocabulary file. Defaults to `<project-root>/.kiro/vocabulary.toml`.
- `--format <text|json>`: Report output format. (Default: `text`).
- `--fail-on <error|warning|info|never>`: Severity threshold triggering exit code 1. (Default: `error`).

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
