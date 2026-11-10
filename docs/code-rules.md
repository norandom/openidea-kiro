# Code rules

Status: first slice (spec `code-constraints`). `kiro-cli code check` measures routines, types and files and judges them against absolute limits, with spec ownership. The ratchet against a previous state, the configuration file, duplicate detection and the locality rules are not implemented yet; this page says so wherever it matters.

The rules exist because a code base written task by task degrades in ways no single task review catches: routines grow, files collect unrelated responsibilities. The gate reports each breach with the spec, component and tasks that own the file and with one edit that resolves it.

## Running it

```bash
kiro-cli code check --task my-feature:2.1 --fail-on warning   # the files of one task
kiro-cli code check --spec my-feature                          # the files of one spec
kiro-cli code check --files src/a.rs src/b.rs                  # named files
kiro-cli code check --all --show-highest                       # whole tree, with the highest values
```

Exit codes follow the spec gate: 0 for no finding at the threshold, 1 for a finding, 2 for a usage or read failure.

## What is measured

Languages: Rust (`.rs`), Python (`.py`, `.pyi`), TypeScript (`.ts`, `.mts`, `.cts`, `.tsx`) and JavaScript (`.js`, `.mjs`, `.cjs`, `.jsx`). Parsing is done by `ast-grep-core` with the tree-sitter grammars; this project writes no parser. A file in another language is listed as not measured and nothing is said about its content.

| Scope | Metric | Definition |
| --- | --- | --- |
| Routine | `parameters` | Declared parameters without the receiver (`self`, `cls`, `this`) |
| Routine | `cyclomatic` | 1, plus one per branching construct (`if`, `elif`, loops with a condition, `case`, `catch`, conditional expressions), per match arm beyond the first, and per short-circuit operator (`&&`, `\|\|`, `and`, `or`, `??`) |
| Routine | `nesting` | Deepest chain of control structures inside the body; the body is level 0, an else-if chain stays on one level |
| Routine | `code_lines` | Lines holding at least one token that is not a comment (Python: nor a docstring) |
| Routine | `statements` | Rust and Python: the items of every block; TypeScript and JavaScript: statement nodes and declarations directly in a block |
| Routine | `lines_per_statement` | `code_lines / statements`, judged only for routines of at least 5 statements |
| Type | `methods`, `fields` | Declared members. Rust: methods of the type's inherent `impl` blocks in the same file count; a trait implementation does not. Python has no field count |
| File | `code_lines`, `functions`, `types`, `max_cyclomatic` | Counted over the whole file; a type alias is not a type |

A nested routine is its own entity and adds nothing to the routine around it. Closures, lambdas and arrow functions belong to the routine that contains them. Rust's `loop`, the `?` operator and macro calls are not decisions.

These definitions carry the version `1`. A change of meaning changes the version.

## Limits

A value equal to its limit passes; only a greater value is reported.

| Scope | Metric | Limit |
| --- | --- | --- |
| Routine | `cyclomatic` | 10 |
| Routine | `nesting` | 3 |
| Routine | `code_lines` | 60 |
| Routine | `statements` | 40 |
| Routine | `parameters` | 5 |
| Routine | `lines_per_statement` | 4 |
| Type | `methods` | 20 |
| Type | `fields` | 10 |
| File | `code_lines` | 500 |
| File | `functions` | 25 |
| File | `types` | 8 |
| File | `max_cyclomatic` | 10 |

The numbers are starting values carried over from the archived scitools-hook project, where they were measured with a different analyzer. They have not been calibrated on this measurement yet. Until they are, **every limit reports as a warning**: `code check` exits 0 at the default `--fail-on error` and 1 at `--fail-on warning`. The limits cannot be changed yet; the configuration file `.kiro/code-rules.toml` is not implemented.

## Findings

| Code | Severity | Meaning |
| --- | --- | --- |
| `CODE_ROUTINE_LIMIT` | warning | A routine metric is above its limit |
| `CODE_TYPE_LIMIT` | warning | A type holds more methods or fields than its limit |
| `CODE_FILE_LIMIT` | warning | A file metric is above its limit |
| `CODE_PARSE_FAILED` | error | A selected file does not parse; no rule was judged on it |
| `CODE_FILE_UNOWNED` | info | A selected file with findings is mapped by no spec |
| `CODE_COVERAGE` | info | What the run measured and what it left out; always present |

Every finding carries a hint that opens with one tag: `simplify:`, `split:`, `shrink:` or `fix:` today (`delete:`, `collect:` and `name:` belong to rules that are not built yet). The JSON report carries tag and hint as separate fields, together with metric, value, limit, the entity and the owning component and tasks. It has no timestamps and no absolute paths, so two runs over the same tree are byte-identical.

## Ownership

A file belongs to the spec whose design lists it in its File Structure Plan, to the component named there, and to the tasks whose `_Boundary:_` names that component. When several specs map a file, the first in roadmap order is the owner and the others are listed. `--spec` and `--task` select files the same way.

## What a clean report does not tell you

- Whether the standard library, the platform or an installed dependency would have replaced a piece of code is never checked.
- A file that was not measured was not judged. Read the `CODE_COVERAGE` line: it counts selected, measured, failed and unmeasured files.
- Nothing is compared with a previous state yet. Existing breaches and new ones look the same.
- Duplicated code, scattered definitions and state, and positional data access are not detected yet.
- A routine within every limit can still be wrong or unclear.
