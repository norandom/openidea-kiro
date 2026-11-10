# Code rules

Status: in progress (spec `code-constraints`). `kiro-cli code check` measures routines, types and files, judges them against configurable limits and against a before side (a Git revision or a committed baseline), and attributes every finding to its spec. Duplicate detection and the locality rules are not implemented yet; this page says so wherever it matters.

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

Files: everything under the project root except `.git`, `target`, `node_modules` and `dist`, and except what the project's `.gitignore` and `.ignore` files exclude. Only ignore files inside the root count; the user's global Git settings do not, so the result is the same on every machine. An untracked file that is not ignored is read. A directory below the root that has its own `.git` (a nested repository or a worktree) is not read.

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

A nested routine is its own entity and adds nothing to the routine around it. In TypeScript and JavaScript an arrow function or function expression bound to a name (a declaration, a class field or an object property) is a routine of that name; a class field holding one counts as a method, not as a field. Every other closure, lambda or inline callback belongs to the routine that contains it. A bare `*` or `/` in a Python parameter list is not a parameter. Rust's `loop`, the `?` operator and macro calls are not decisions.

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

The numbers are starting values carried over from the archived scitools-hook project, where they were measured with a different analyzer. Their position in the distributions of five repositories is recorded in `.kiro/specs/code-constraints/research.md` (calibration record, 2026-11-10).

**What blocks.** The five routine limits `cyclomatic`, `nesting`, `code_lines`, `statements` and `parameters` are calibrated for Rust and Python: there a breach is an **error** and `code check` exits 1 at the default `--fail-on error`. Every other limit is a **warning**: the same five in TypeScript and JavaScript (one repository was too little to calibrate them), `lines_per_statement`, and all type and file limits. Use `--fail-on warning` to block on those too.

A severity named in the configuration replaces this default for every language; naming a weaker one for a calibrated metric needs a reason.

An error on code that already existed blocks only when there is nothing to compare with. With a before side (`--changed <rev>` or a baseline, see the ratchet below) it is marked pre-existing and does not block; only a new breach or a regression does.

## Configuration

The limits, their severities, path scopes and ignores live in one committed file, `.kiro/code-rules.toml`. Without the file the built-in limits apply.

```toml
[limits.routine.cyclomatic]
max = 12
reason = "state machines in the parser"     # required: looser than the built-in 10

[limits.routine.nesting]
severity = "error"                           # stricter than the built-in: no reason needed

[limits.file.types]
severity = "off"
reason = "type-heavy model modules"

[[scopes]]
paths = ["crates/kiro-core/src/checks/"]     # a directory, or a single file
reason = "finitization is one algorithm"
[scopes.limits.routine.code_lines]
max = 120

[[ignores]]
paths = ["crates/kiro-code/fixtures/"]
reason = "fixtures are measured by tests, not judged"
```

- Severities are `error`, `warning`, `info` and `off`.
- A scope overrides limits for the files under its paths. The most specific matching path wins; on a tie the later scope wins. A directory path covers whole path segments only.
- An ignored file is still measured and counted in the coverage line, but not judged.
- **A reason is required** on every scope, on every ignore, and on every limit that is looser than the built-in one (a higher maximum, or a more permissive severity).
- An unknown key, metric or severity, a limit that is not a number, a missing reason or an invalid path stops the command with `CODE_CONFIG_INVALID` and exit code 2 before anything is measured. All problems are listed at once, each with its key.
- `kiro-cli code config` prints every effective limit, scope and ignore with its origin (`default` or `file`) and reason, and a digest of the effective configuration. The digest also appears in the coverage line of every check.

Changing this file is a human decision. An agent that meets a finding repairs the code; it does not relax a limit, add an ignore or widen a scope to get a pass.

## The ratchet: only regression blocks

A grown code base has breaches everywhere. The gate would be useless if all of them blocked, so it compares each entity with itself on a *before side* and blocks only what got worse or is new.

The before side is, in this order:

1. the revision named by `--changed <rev>`: the gate then selects the tracked files that differ from that revision and compares them with their text there;
2. otherwise the committed baseline `.kiro/code-baseline.json`, written by `kiro-cli code baseline`;
3. otherwise none: every breach is judged on its limit alone, and the coverage line says so.

| Situation | Result |
| --- | --- |
| A breach that existed before and did not worsen | The limit finding is marked pre-existing: reported, never blocking |
| A routine metric that rose and is above its limit | `CODE_RATCHET`, error |
| A routine metric that rose and is still within its limit | `CODE_RATCHET`, warning |
| A new routine, or one that cannot be paired, above a limit | The plain limit finding |
| Lines or statements grew while complexity or nesting fell and neither rose | No ratchet finding: guard clauses and extraction are not punished |
| A file or type count grew within its limit | Nothing: container counts are never ratcheted |

How an entity is paired with its earlier self:

- **exact**: same file (through the renames Git reports), qualified name, parameter text and position;
- **moved**: a routine that left one file and appeared in another with the same name, parameters and an identical body, with one candidate on each side. A move keeps the debt status, so splitting a large file is not reported as new breaches;
- **signature**: within one file and name, exactly one routine went and one came (the parameters changed). This pair can show regression but never excuses a breach.

A file that did not parse on either side takes part in no comparison. `--changed` covers staged and unstaged changes of tracked files; a file Git does not track is selected with `--files` or `--all`. A missing revision is a failure (exit code 2); nothing is fetched.

The baseline is the measurement of the whole tree: sorted, without timestamps, byte-identical for the same tree. `code baseline` refuses when a file does not parse. Capturing a new baseline hides every regression since the last one, so it is a human decision, like changing the configuration.

The coverage line ends with the net growth over the changed files, deleted files included, for example `net: -4 statements, +2 lines`. When a side is incomplete it says `unavailable` and why; a missing side is never counted as zero.

### In a hook

`--summary` prints at most 25 lines of plain ASCII in a fixed order: verdict, before side, finding counts, what worsened and what improved with before and after values, and the net growth. The exit code is the same as without it.

```bash
# pre-push: compare with the upstream branch
kiro-cli code check --changed origin/main --summary
```

The gate installs no hook. Call it from the hook mechanism you already use.

## Duplicate blocks

Status: first rule of spec `clone-constraints`.

`CODE_DUPLICATE_BLOCK` (warning) reports a run of at least 12 consecutive code lines in a selected file that the project also holds at another location. It names the original with its line range and up to three further locations, and its hint is `delete:` keep one copy and call it.

- A code line is the line's tokens with whitespace and comments removed. Indentation, spacing inside a line, comment lines, trailing comments and blank lines do not hide a copy. Lines without a token are skipped and do not interrupt a run.
- The search covers every measurable file of the project, also for a task-scoped check: a copy counts wherever the original stands. Findings are reported for the selected files only; when both ends are selected, each is reported from its own side.
- Two copies in one file count. A run that only overlaps itself does not.
- A renamed identifier, a reordered statement or a line broken differently changes the line and ends the run. Copies that were renamed or reshaped are the subject of the similar-routine rule, which is not built yet.
- A block that occurs at more than 64 locations is not expanded; the coverage line counts such windows.
- The coverage line ends with what was searched, for example `duplicate blocks: 158 files searched, 96 blocks of at least 12 lines, 0 frequent windows skipped`.
- The length of 12 lines and the severity cannot be configured yet, and duplicates are not compared with a before side: an old copy and a new one look the same.

## Scattered definitions and scattered state

Status: first rules of spec `code-locality-data`. Both count across the whole project and report for the selected files.

**`CODE_SCATTERED_DEFINITION`** (warning). The same name is bound to the same initializer text at module level in more than 3 files, for example `TIMEOUT_MS = 30_000` in four modules. The hint is `collect:` define it once in the module that owns it and import it.

- Module level means: Rust `const` and `static` items at the top of a file or inside `mod`; Python assignments at the top of a module; TypeScript and JavaScript `const`, `let` and `var` at the top of a file, exported or not. A definition inside a routine does not count.
- Name and value are keyed together: the same name with different values is not a repetition. When one name carries two or more values that are each scattered, `CODE_DEFINITION_AMBIGUOUS` (information, `name:`) says so; the groups are never merged.
- The comparison is textual, with whitespace collapsed. A value that refers to another name is compared as written; nothing is evaluated. An initializer longer than 12 lines and a destructuring assignment are not definitions.
- The names `log`, `logger`, `pytestmark` and `__all__` are ignored: every module legitimately has its own.

**`CODE_SCATTERED_STATE`** (information). One subject is compared with at least 2 distinct literal values, and the comparisons stand in more than 3 files, for example `order.status == "paid"` in one file and `order.status == "open"` in three others. The hint is `collect:` declare the values once, as an enumeration or named constants, and put the decisions on them in one place.

- A comparison is an equality or inequality with a string literal on either side, membership in a literal tuple, list or set (Python), and the arms of a `match` or the cases of a `switch`. In Rust an enum path such as `Mode::Fast` is a literal too.
- A subject is its spelling. Two unrelated variables named `status` are one subject here, and one concept spelled `order.status` and `o.status` is two. This is why the rule is information and never blocks: read it as a pointer, not as a verdict.
- The receiver (`self`, `this`) is never a subject: `match self` in two types is not one state.
- A subject whose comparisons all stand in one file is not reported: that state already has an owner.
- The rule cannot see a state carried in booleans, in a number compared with `<`, in a literal that reaches the comparison through a variable, or in dispatch through a table or polymorphism.

The thresholds (3 files, 2 values) and the ignored names cannot be configured yet. Neither rule is compared with a before side. Positional data access and vocabulary grounding of column names are not built.

## Findings

| Code | Severity | Meaning |
| --- | --- | --- |
| `CODE_ROUTINE_LIMIT` | warning | A routine metric is above its limit |
| `CODE_TYPE_LIMIT` | warning | A type holds more methods or fields than its limit |
| `CODE_FILE_LIMIT` | warning | A file metric is above its limit |
| `CODE_RATCHET` | error or warning | A routine metric rose against the before side: error above its limit, warning within it |
| `CODE_DUPLICATE_BLOCK` | warning | A run of at least 12 code lines also stands at another location |
| `CODE_SCATTERED_DEFINITION` | warning | The same module-level definition stands in more than 3 files |
| `CODE_DEFINITION_AMBIGUOUS` | info | One name carries several scattered values; they were not merged |
| `CODE_SCATTERED_STATE` | info | One subject is compared with literal values across more than 3 files |
| `CODE_PARSE_FAILED` | error | A selected file does not parse; no rule was judged on it |
| `CODE_FILE_UNOWNED` | info | A selected file with findings is mapped by no spec |
| `CODE_COVERAGE` | info | What the run measured and what it left out; always present |

Every finding carries a hint that opens with one tag: `simplify:`, `split:`, `delete:`, `shrink:`, `collect:`, `name:` or `fix:`. The JSON report carries tag and hint as separate fields, together with metric, value, limit, the entity and the owning component and tasks. It has no timestamps and no absolute paths, so two runs over the same tree are byte-identical.

## Ownership

A file belongs to the spec whose design lists it in its File Structure Plan, to the component named there, and to the tasks whose `_Boundary:_` names that component. When several specs map a file, the first in roadmap order is the owner and the others are listed. `--spec` and `--task` select files the same way.

## What a clean report does not tell you

- Whether the standard library, the platform or an installed dependency would have replaced a piece of code is never checked.
- A file that was not measured was not judged. Read the `CODE_COVERAGE` line: it counts selected, measured, failed and unmeasured files.
- Without a before side nothing is compared: existing breaches and new ones look the same. The coverage line names the before side.
- Copies that were renamed or reshaped and positional data access are not detected yet.
- A routine within every limit can still be wrong or unclear.
