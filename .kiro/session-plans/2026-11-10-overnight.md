# Overnight session plan, 2026-11-10

Goal set by the owner: get as much done as possible, unattended. Priority set earlier the same day: complete the scitools-hook port (code-constraints, then clone-constraints and code-locality-data). Dashboards are in the backlog and are not touched.

This file is the work order for a session that runs without the owner. Work top to bottom. Each item names its done-test. The state at the start is in section 1; the rules are in section 2; the queue is in section 3; the morning report is in section 5.

## 1. State at the start

All work of 2026-11-10 is uncommitted in the working tree of `main`.

| Spec | Tasks done | State |
|---|---|---|
| cli-contracts | 9 of 16 | Shared library and migrated `check` done (C0, C1). Open: 4.1 to 4.3, 5.1, 5.2 |
| quality-telemetry | 14 of 21 | Writer, Collector example, catalogue done and verified with OpenObserve. Open: 5.2 to 5.4, 6.1, 6.2. Dashboards are backlog |
| code-constraints | 6 of 46 | Parse layer adopted; `kiro-code` crate; `code check` with limits and ownership; journey passes on Windows |
| clone-constraints | 0 of 24 | Not started. Its library candidates are untested |
| code-locality-data | 0 of 26 | Not started |
| verification-evidence | 0 of 26 | Not started; follows the port |

Verified before this plan was written: 470 workspace tests pass on Windows and the same 470 pass on Linux (throwaway `rust:latest` container, rustc 1.99.1); clippy with warnings denied; format check; `kiro-core` builds for `wasm32-wasip2`; `cargo deny check` passes for the workspace and the plugin. The first Linux run found one real defect, fixed the same evening: a telemetry directory given as `a/../b` through a directory that does not exist was rejected with the wrong message, because only Windows resolves such a path lexically. macOS has not been run.

Known defects and debts to carry in mind:

- `crates/kiro-cli/src/main.rs`: `main` has cyclomatic complexity above 50 and grew today. The code gate reports it.
- `kiro_core::code::evaluate` recomputes ownership per file and `CodeFacts::value` is a linear lookup. Fine at 144 files (2.7 s release), not at ten times that.
- Finding records in telemetry lack hint and owners; `Finding` has neither. Code findings have both and are not exported yet.
- Partly done code-constraints tasks are listed in `.kiro/specs/code-constraints/research.md` under "Implementation record: M0 and the first M1 slice".

## 2. Rules for the unattended session

1. **Scope.** Only the items in section 3, in order. No dashboards. No change to approved requirements: if a requirement turns out wrong or impossible, write that down under the task and move on.
2. **No parser, no reinvention.** Source code is parsed only through `ast-grep-core`. Before writing an algorithm, check whether a pinned crate does it; record the check.
3. **Every task ends with evidence.** Run the task's tests, then the gate: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo build -p kiro-core --target wasm32-wasip2`, `cargo deny check`. Mark a task `[x]` only when its done-test in `tasks.md` is met in full. Otherwise leave it unchecked and note what is missing in the spec's `research.md`.
4. **Time box.** About 90 minutes per task. Three failed attempts to get the gate green on one task: revert that task's changes, write down why, take the next task that does not depend on it.
5. **Hand-counted oracles first.** For every new metric or detector, write the expected values by hand from the fixture before running the tool. When tool and hand count differ, find out which is wrong and say so in the test.
6. **Honest labels.** Nothing is "calibrated", "accepted" or "passed" without the run that shows it. Limits stay warnings. The breadth ledger stays `planned`.
7. **Outside the repository.** Prototypes and disposable output go under `D:\BuildArtifacts\openidea-kiro\`. Linux runs use a throwaway `rust:latest` container with the repository mounted read-only (`D:\BuildArtifacts\openidea-kiro\linux-test\run2.sh`, started with `docker run --rm -v "D:\Source\openidea-kiro:/src:ro" -v "D:\BuildArtifacts\openidea-kiro\linux-test:/script:ro" rust:latest sh /script/run2.sh`). Pass shell scripts as mounted files; inline quoting through PowerShell breaks. Do not remove containers or data another task created.
8. **Not allowed.** No push, no force operations, no change to branch protection or CI secrets, no archive or change of `../scitools-hook` or `../lesscode`, no installation outside Cargo and the containers, no network service left running at the end (stop containers that were started).
9. **Commits.** Only if the owner allowed them in the start prompt. Then: branch `feat/code-quality-port`, one commit per completed task or coherent step, message `<spec> <task>: <what>`, trailer `Co-Authored-By: Claude <noreply@anthropic.com>`. Without that permission, leave everything uncommitted and say so in the report.
10. **Subagents.** Allowed where tasks are independent (clone prototype, locality prototype, corpus extension). One writer per file at a time: `crates/kiro-core/src/lib.rs`, `finding.rs`, `crates/kiro-cli/src/cli.rs`, `main.rs`, `Cargo.toml` files and `docs/cli.md` are edited by the main session only.
11. **Shell.** The Git Bash tool sometimes fails with a fork error and applies nothing. After any `sed` or `awk` edit, check the result. Prefer the edit and write tools. Reads of files of 300 lines or more need an offset and limit. The current binary is `%CARGO_TARGET_DIR%\debug\kiro-cli.exe`; `./target/debug/kiro-cli.exe` in the repository is stale.
12. **Checkpoint.** Save an mmhmem checkpoint after every completed item and before stopping.

## 3. Work queue

### 0. Start check (10 min)

- Run the gate once on Windows and the Linux script `D:\BuildArtifacts\openidea-kiro\linux-test\run2.sh` (it uses `--no-fail-fast`; the first version stopped at the first failing suite and hid the rest). Both showed 470 passed at the end of the evening.
- Done-test: the same counts, before any change. If not, find out what changed first.
- Repeat the Linux run after every completed section of the queue, not only at the end.

### 1. code-constraints, finish M1 (tasks 1.2, 1.4, 2.2, 2.3, 3.2, 3.3, 4.1, 4.3)

1. **Refactor `main.rs` into one function per command** returning `Result<Exit, Failure>`, no behaviour change. Done-test: the 18 compatibility records and all journeys pass unchanged; `code check --files crates/kiro-cli/src/main.rs` reports no routine above the limits.
2. **Index evaluation**: ownership computed once per file, values looked up by entity. Done-test: reports byte-identical before and after on this repository; debug whole-tree time recorded.
3. **1.2 Corpus**: add hand-counted fixtures for Rust (generics, macros with control flow inside, trait impls, nested modules, `impl` for a foreign type), Python (decorators, async, lambda, nested function, class with properties, `match`, `try`), TypeScript and JavaScript (arrow functions, class fields, interface, enum, async, optional chaining, `for..of`, `try`). Decide and document how arrow functions assigned to a `const` are treated; today they are not routines.
4. **2.2 Configuration** `.kiro/code-rules.toml`: limits and severities per metric, path scopes with most-specific-wins, name and path ignores, a required `reason` on every scope, ignore and relaxation, unknown keys rejected with `CODE_CONFIG_INVALID` and exit 2 before measuring, a stable config digest, and `code config` printing every effective value with its origin. Use the existing `toml` dependency of the core.
5. **2.3** containment tests: link leaving the root, excluded directories, oversize file, non-UTF-8 file.
6. **3.2, 3.3, 4.1**: scoped limits in judgement; a required metric that is unavailable prevents a clean verdict; `RuleOutcome` type and `compose` so extension findings can join without the base importing them.
7. **4.3**: journey on Linux through the container.

### 2. code-constraints M2: the ratchet (tasks 5.1 to 7.3)

This is the heart of the port: only regression blocks.

1. **6.1 Git before side** in `crates/kiro-code/src/git.rs`: `git rev-parse --verify`, `git diff --name-status -z --find-renames <rev>`, `git cat-file --batch`, separate arguments, read-only, no checkout. Deleted and renamed paths kept for accounting. Fixtures create their own repositories.
2. **5.1, 5.2 Pairing and safeguards**: exact identity; unique signature change for regression only; the move rule (identical normalized body and signature, one candidate each side, keeps debt status); parse failure or below-floor population on either side gives no comparison; decomposition guard (count growth excused only when a control-flow metric improves and none worsens). Write the decision corpus `crates/kiro-core/fixtures/code/ratchet.json` by hand first.
3. **5.3 Baseline**: `code baseline` writes facts deterministically; incompatibility reported, never silently compared. `--changed <rev>` wins over the baseline file.
4. **6.2, 6.3** net statement and line delta only from complete comparable scopes, otherwise "unavailable" with the reason.
5. **7.1 Hook summary and metric points**: `code check --summary` (at most 25 lines, printable ASCII, fixed order, improved and worsened metrics with before and after); `impl MetricProvider` for the code report with names under `kiro.code.` registered in the telemetry catalogue and `docs/telemetry.md`; code findings exported with hint and owners.
6. **7.2, 7.3** journeys: existing debt accepted, new regression rejected, repair passes, a moved routine keeps its status, renamed and deleted files accounted, a disabled ratchet breaks the journey.

### 3. clone-constraints (K0, K1, K2)

1. **K0 prototype outside the repository**: `cpd-core` and `cpd-tokenizer` (0.1.23, 0.1.22) for duplicate blocks and `difflib-fast` (0.6.0) or `difflib` (0.4.0) for the similarity ratio, against a hand-labeled corpus: exact copy, renamed copy, reformatted copy, near-threshold pair, unrelated similar syntax. Record API, licence, build on Windows and Linux, whether they share the tree-sitter runtime of `ast-grep`, and the decision. If `cpd-*` brings a second parse stack, derive tokens from the `ast-grep` tree instead and record that.
2. **K1**: duplicate blocks of at least 12 normalized code lines, `CODE_DUPLICATE_BLOCK`, through `code check`, with the introduce-copy, fail, remove, pass journey.
3. **K2**: token-shape families at ratio 0.9 for routines of at least 6 statements, connected components, exclusions with reachability repair, `CODE_SIMILAR_FAMILY` with `shrink:` or `delete:`.
4. jscpd report import stays in the spec but is last; skip it if time is short.

### 4. code-locality-data (L0, L1, L2)

1. **L0**: confirm `ast-grep` pattern matching serves the extraction; fixture comparison recorded.
2. **L1**: scattered definitions (same name and initializer in more than 3 files), ignore list, ambiguity info, consolidation journey.
3. **L2**: scattered state and schema, shipped at info or warning as the design says, with the same-spelling negative cases in the corpus.
4. **L3** (data access with vocabulary grounding) only if everything above is green.

### 5. If time remains

1. code-constraints M3: Python and TypeScript journeys (8.x), SARIF (9.1), `code rules --write` (9.2), companion commands and docs (9.3).
2. code-constraints 11.1: advisory inventory of this repository and candidate configuration and baseline files for the owner's review. Do not enable blocking and do not add the self-gate to CI.
3. cli-contracts 4.1 to 4.3 and 5.1 (migrate `status`, `graph`, `tasks`, `vocab validate`, `vocab import`, add `completions`), with the 18 compatibility records unchanged.
4. verification-evidence E0 and E1.

## 4. Decisions already made, to apply without asking

- Parse layer: `ast-grep-core` 0.51.1; `rust-code-analysis` rejected.
- Limits are priors and stay warnings until calibrated.
- Telemetry is OTLP JSON files outside the repository; no network code in the gate.
- scitools-hook compatibility is not a goal; its report import is removed.
- macOS is required but cannot be run here: say "not run" in every record.
- When a design detail conflicts with an existing test or rule of the repository (as with `std::fs` confined to `fs.rs`), follow the repository and record the deviation.

Questions that need the owner are collected in the report, not guessed, when they change a requirement, a public flag that already exists, or a policy file other than `deny.toml`'s native-crate exclusion.

## 5. Morning report

Write `.kiro/session-plans/2026-11-11-report.md` and give the same content as the last message:

1. What is done, by spec and task, with the test counts of the final gate on Windows and Linux.
2. What was started and not finished, and why.
3. What was reverted, and why.
4. Deviations from designs and decisions taken alone.
5. Questions for the owner.
6. Whether anything is committed, on which branch, and the list of commits.
7. Containers and files left outside the repository.
