# Design review: risks, assumptions and reuse

Reviewed 2026-11-10. Scope: the five active designs of the code-quality program (cli-contracts, verification-evidence, code-constraints, clone-constraints, code-locality-data) and `steering/code-quality-strategy.md`. This is a document review. Nothing was prototyped or executed; crate versions come from `cargo search` and `cargo info` on the review date.

Owner decisions that this review applies:

- scitools-hook will be archived. It depends on a licensed analyzer, is not built for this purpose and is too complex. Its ideas are kept; compatibility with it is no longer a goal.
- The tooling must run on Windows, Linux and macOS with open-source dependencies only.
- `../lesscode` completes the stack: it reduces existing code; this project keeps new code maintainable.

## Conclusion

The designs are structurally sound and honest about what is unproven, but they still carry scitools-hook compatibility as a design constraint in four places, they select three parsing stacks independently, and they place 38 criteria of process tooling ahead of the first code finding. Remove the compatibility work, choose one parse layer for the whole program in a single prototype, and approve tasks only up to that prototype.

## A. Consequences of archiving scitools-hook

| # | Where | Now | Change |
|---|---|---|---|
| A1 | code-constraints 1.2, 3.5, 4.5, 11.5; `kiro-code/src/scitools.rs`; upstream-metric table; scitools-source journey | Imports scitools-hook schema 2 reports and keeps Understand metric identities | Remove. No user of this project will have such a report. Saves one adapter, one dialect, one journey and the sparse-import rules that exist only for it. |
| A2 | code-constraints technology table, "Comparison oracle: pinned Lizard and sibling scitools fixtures" | Oracle regenerated from a tool that needs a licence | Use the hand-counted corpus (already required by 12.1) as the only oracle. Lizard may stay as a development cross-check; it needs Python, which is absent on the owner's Windows machine. |
| A3 | clone-constraints goal "Preserve scitools-compatible token-family behavior"; SequenceMatcher parity, asymmetric score order, tie behaviour | Byte parity with a Python implementation of an archived tool | Define the family rule by this project's labeled corpus (3.2, 3.4). Parity has no remaining user value and forces a custom scorer. |
| A4 | code-constraints candidate limits (10, 3, 60, 40, 5, 500, 25, 20, 10) | Numbers measured with Understand definitions | Keep as starting values, state that they are priors, and name the calibration corpus. The calibration manifest exists; the corpus is not named. |
| A5 | `docs/code-feature-map.md` (planned) | Written during implementation | Freeze now from `research.md`, with the scitools-hook commit hash, so the ideas survive the archive. |

A1 and A3 change approved requirements and need the owner's approval before the designs and tasks are regenerated.

## B. Where the designs rebuild something that exists

| # | Design intent | Existing work | Recommendation |
|---|---|---|---|
| B1 | Native duplicate detector plus import of two jscpd JSON dialects | `jscpd` 5.4.3 is now a Rust crate and CLI (MIT) with `cpd-core` 0.1.23 and `cpd-tokenizer` 0.1.22. The design pins 0.1.20 and 0.1.19, three releases behind within days. | Link the library and drop report import. The two dialects belong to the older Node releases. A 0.1.x API that moves this fast needs an exact pin and a thin adapter. |
| B2 | Family score and connected components | `difflib-fast` 0.6.0 (MIT) states an exact Ratcliff-Obershelp ratio with single-linkage clustering; `difflib` 0.4.0 is a direct port | Try `difflib-fast` first. With A3 applied, any documented ratio is acceptable. |
| B3 | `rust-code-analysis` as preferred metric provider | Still 0.0.25 on crates.io; a third-party fork exists only to move it to tree-sitter 0.26 | Treat as unmaintained. To my knowledge it has no nesting-depth metric, which is one of the five required routine metrics; verify in G1. |
| B4 | Three separate G1 choices: metric library, clone tokenizer, tree-sitter queries or ast-grep | `../lesscode` already pins `ast-grep-core` 0.51.0 and `syn` and ships statement and line counts for Rust, Java and TypeScript | Decide one parse layer for `kiro-code` in one joint prototype. `ast-grep-core` is the candidate that serves metrics and locality patterns and matches lesscode. Three stacks can mean three tree-sitter runtimes in one binary; whether they link cleanly on Windows is unverified. |
| B5 | Breach detection for size, parameters, nesting, complexity | clippy (`too_many_lines`, `too_many_arguments`, `excessive_nesting`, `cognitive_complexity`), ruff (C901, PLR0912, PLR0913, PLR0915), ESLint and Biome | The measurement is a commodity. What no linter has is the value, the ratchet, the spec and task attribution and the hint contract. Keep the custom work there. Replace the scitools import (A1) with a generic SARIF import so linter findings can enter the report with attribution. |
| B6 | Custom input manifest that enumerates and hashes files | Git tree hash, plus a hash of the dirty diff, plus `Cargo.lock`, `rustc -Vv` and the target triple | Use Git identity in E2. Write a file enumerator only if a failing case shows Git identity is insufficient. |
| B7 | RunRecord normalised from JUnit | `cargo nextest list --message-format json` and nextest JUnit give exact test identities | Already planned. The custom part is only the criterion-to-test mapping; keep it that small. |
| B8 | `scripts/verify.ps1` as the canonical runner | The repository already has Dagger and Cargo aliases | A third orchestration layer that needs PowerShell 7 on Linux and macOS. A `cargo xtask verify` binary needs no extra runtime on any of the three systems. |
| B9 | `GlobalArgs { verbose: u8, quiet: bool, color }` with hand-written environment precedence | clap `env` attributes, `clap-verbosity-flag`, `colorchoice-clap` | Use them. The earlier design did; the split lost it. |
| B10 | File and function size ceilings | `lesscode-core budgets` (Rust only: 400 lines, 300 syntax units, 80 lines per function, pre-push hook) | See section E. One of the two must own this. |

## C. Implementation risks

| # | Risk | Why it matters | Suggested response |
|---|---|---|---|
| C1 | Splitting a god file is punished. A routine that is over a limit and moves to another file disappears and reappears; the design gives heuristic pairs regression only, never pre-existing status. | The `split:` hint asks for exactly this move. The agent follows the hint and the gate blocks it. | Add a move rule: identical normalized body and signature in the same change keeps the debt status. Add it to the ratchet corpus before M2. |
| C2 | Task plans exist for work whose provider is unknown. Each design says it is revised if G1 fails. | 100 subtasks rest on file plans that G1 can invalidate. | Approve cli-contracts, verification-evidence E0 and E1, and one joint G1 prototype. Regenerate the three analyzer task plans after G1. |
| C3 | Two foundation specs (38 criteria) precede the first code finding. | The gap named in the roadmap, code that degrades under agents, stays open while tooling about tooling is built. | Let code-constraints M1 depend on the E1 harness only. `breadth accept` can follow. |
| C4 | Scattered-state groups by subject spelling. `status`, `kind`, `mode`, `state` occur in most projects. | Warnings by construction. Agents respond to warnings with churn. | Ship at info. Require a second signal, such as the same literal set, before warning. |
| C5 | Locality and clone rules need the whole project even for a task-scoped check, with a 10 second budget and no cache. | The budget is stated against a "recorded corpus" that is not defined. | Name the corpus and its size. Measure whole-bundle extraction in G1. |
| C6 | Two before-sides: committed baseline and Git revision. | A committed JSON baseline conflicts on every parallel branch. "Agents cannot recapture" has no mechanism in the design. | Start with the Git before-side only. If the baseline file stays, state the enforcement: code owners and branch protection on `.kiro/code-rules.toml` and the baseline. |
| C7 | Evidence identity contains the binary hash. Rust builds are not reproducible by default. | A record can only be verified in the job that produced it. The meaning of `passed` in a committed ledger for a later reader is not stated. | State that acceptance is same-job. Define `passed` in the ledger as a historical claim that references a CI run. |
| C8 | macOS is absent from every design. Line endings are not addressed. | Git blobs are LF, a Windows working tree may be CRLF. Line counts, clone hashes and initializer comparison must agree across both sides. | Add macOS to required platforms. Normalize line endings in the source bundle and test it. |
| C9 | Limits can be satisfied by extraction without simplification, and ignore entries carry reasons the agent writes itself. | The decomposition guard covers one case. | State that configuration and ignore diffs need human review, and put that in the review skill. |
| C10 | Shared files (`lib.rs` catalogue, manifests, three skills) are edited by five specs. | The strategy requires foreground sequential work; `code-constraints/spec.json` notes still ask for parallel subagents. | Decide which instruction holds and remove the other. |

## D. Assumptions that are implied but not written down

| Assumption | What breaks if it is false | How to find out |
|---|---|---|
| Size, complexity, duplication and locality limits make code easier for agents to change. | The gate costs time and changes nothing, or causes churn. | Compare agent task outcomes with and without the gate on the same tasks. No design plans this. G2 tests the binary, not the agent. |
| The File Structure Plan of each design is current. | Findings go to the wrong spec or to no spec. | `CODE_FILE_UNOWNED` counts over time on this repository. |
| Agents read the hint line and act on it. | Hints and tags are unused output. | Inspect agent transcripts in the first journeys. |
| A human owns limits, ignores and the baseline. | The ratchet is reset silently. | Enforcement is outside the tool; name it (C6). |
| CI is trusted; a run record is not a signature. | Stated in verification-evidence. | No action; keep the sentence visible in the user documentation. |
| Git is installed and history is complete. | `--changed` exits 2. | Stated. CI checkouts are shallow by default, so document the fetch depth. |
| Data topics exist in the vocabulary. | Column grounding never runs. | Nobody is named as their author. Ship one example topic. |
| Rust, Python and TypeScript/JavaScript are enough. | lesscode also covers Java. | Decide whether the two tools promise the same languages. |

The first row is the hypothesis the program rests on. We are wrong if agents meet the limits by moving code around while review effort stays the same.

## E. Division of work with lesscode

lesscode dropped its own preventive policy engine on 2026-11-01 (`../lesscode/.kiro/specs/agent-quality-rules/direction-amendment.md`): per-function budgets cannot establish that a capability became simpler. That leaves a clean division.

| Concern | Owner | Other side |
|---|---|---|
| Limits on new and changed code, ratchet, attribution to spec and task | openidea-kiro | lesscode does not gate on policy |
| Finding candidates: over-limit routines, duplicate families, scattered definitions | openidea-kiro findings with `shrink:`, `delete:`, `collect:` hints | lesscode takes them as input for a reduction |
| Proving a rewrite is a real reduction with the same behaviour | lesscode receipts (`density`, `compare`) | kiro does not judge equivalence |
| State write sites and dynamic escape hatches | lesscode `deconvolute analyze` | kiro scattered-state stays textual and advisory |
| File size ceilings | Overlap: `lesscode-core budgets` and code-constraints file limits | Proposal: kiro owns ceilings once code-constraints M1 ships; until then this repository can use `lesscode-core budgets` as is |

Rules against redundancy: one parse stack where possible (B4); kiro does not add equivalence or mutation checks; lesscode does not add spec attribution; a lesscode receipt can be referenced as evidence in the breadth ledger instead of being recomputed.

## Proposed next steps

1. Owner approves or rejects A1 and A3 (requirement changes).
2. One joint G1 prototype: parse layer, the five routine metrics on a hand-counted Rust corpus, `cpd-core` blocks, `difflib-fast` families, on Windows, Linux and macOS.
3. Regenerate code-constraints, clone-constraints and code-locality-data designs and tasks from the result; add C1 and C8.
4. Proceed with cli-contracts and verification-evidence E0 and E1 meanwhile.

## Status after owner decisions (2026-11-10)

| Item | Outcome |
|---|---|
| A1, A2, A3 | Approved and applied. Report import and scorer parity are removed. In exchange code-constraints gains Requirement 13: ASCII hook summary and optional JSONL metric log. |
| A4 | Applied: limits are stated as priors and the calibration corpus is named in the design. |
| A5 | Open. The owner archives the repository; the feature map stays in `research.md` until `docs/code-feature-map.md` is written. |
| B2, B3, B4 | Applied as a rule: parsing and scoring are adopted from libraries, nothing is hand-written, one joint G1 prototype. Library choice itself waits for G1. |
| B1 | Pins updated. jscpd report import (clone-constraints 1.7, 2.1) is unchanged; removing it was not decided. |
| C1, C8 | Applied: move rule in the ratchet, macOS required, line endings normalized. |
| C10 | Decided: foreground or subagents as fits, Fable 5.2 accepted. |
| B5-B10, C2-C7, C9 | Open. |
