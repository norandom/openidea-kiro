# Code-quality delivery strategy

Owner decisions, 2026-11-09: apply the code-constraints review amendments, proceed to bounded designs, then produce tasks for all five specs. Requirements and designs are approved for that progression; fresh task plans are generated and await review. No implementation or prototype result is claimed.

## Dependency waves and deliverables

| Wave | Spec | Boundary | First deliverable |
|---|---|---|---|
| 1 | cli-contracts | Shared groups and compatible existing CLI | Existing check migrated with unchanged behavior |
| 2 | verification-evidence | Ledger, shared harness, current-execution acceptance | Existing-binary journey and false-claim refusal |
| 3 | code-constraints | Comparable metrics, refactoring-safe ratchet, attribution | Rust task-scoped fail/fix/pass and baseline regression |
| 4 | clone-constraints | Duplicate blocks, families and jscpd reports | Introduced-copy and repair journey |
| 4 | code-locality-data | Scattered policy/state/schema and named data | Consolidation journey |

This is a dependency sequence, not permission to dispatch agents. The foreground workflow remains in force. The prior code-constraints implementation preferences remain historical context until implementation is explicitly requested.

## Shared contracts and ownership

- Existing kiro-core owns Source, Finding, catalogue registration and the file/component/task join. Feature owners request additive registration changes at a serialized join.
- cli-contracts owns common argument/exit/output groups and existing command migrations; each feature owns its new command behavior.
- verification-evidence owns the plan/run-record schema, independent identity calculation, test runner and common harness. Feature specs own their test assertions and exact required test identities.
- code-constraints owns CodeFacts, metric semantics, numeric baseline/comparison and RuleOutcome. Consumers do not add private detector state to numeric baselines.
- clone-constraints owns clone settings/facts/detectors; code-locality-data owns locality/data settings/facts/patterns.
- Shared Cargo manifests, lib.rs/catalogue, CLI dispatch and skill files are serialized integration points, not parallel-edit targets. Implementation tasks name the owning boundary and additive change.
- In kiro-impl/kiro-validate-impl, verification-evidence owns evidence workflow sections and code-constraints owns code-check sections. No duplicate whole-file ownership.
- One first usable journey ships with each implementation slice. Reuse tests/common instead of copying harnesses.

## Development gates

| Gate | Pass evidence | Failure response |
|---|---|---|
| G0: scope/contracts | Every original criterion has an owner; concrete schemas/interfaces, output compatibility and planned ledger | Repair design before task generation |
| G1: reuse prototype | Pinned packages compared with hand-counted/labeled corpus; semantic/build/licence/determinism differences recorded; adoption decision | Revise design or authorize only a demonstrated missing implementation |
| G2: first vertical slice | Real binary fails, code/fixture is repaired, same command passes; detector-disable variant breaks journey | Do not expand feature scope |
| G3: measurement integrity | Comparable identities, sparse imports, parse/floor/decomposition guards, signature trust and rename/delete accounting | No baseline adoption |
| G4: quality and scale | Known corpus positives/negatives correct; precision/recall and representative findings reviewed; task <10s / all <60s on recorded release corpus; work exhaustion is incomplete | Keep rule advisory or revise approach |
| G5: evidence integrity | Exact required tests collected/executed/passed; no required skip; independent identity agrees; false-claim cases rejected | Capability remains partial/unverified |
| G6: integration | Old CLI behavior, consumer contract fixtures, Windows/Linux/macOS and sandbox boundaries; one canonical suite runner | Repair affected contract and revalidate dependents |
| G7: rollout | Advisory inventory, human-reviewed config/baseline and documented rollback | Do not enable blocking adoption |

Prototype gates are implementation deliverables, not satisfied by this design pass. Future tasks must put G1/G2 before broad implementations. Each spec's design names the milestone-specific prerequisites and observable results.

## Breadth and evidence

.kiro/breadth.toml is the committed capability plan. Entries seeded at design split remain planned; planned_checks are not executable tests. Actual test_paths/e2e_paths and execution references are added as implementation produces them.

breadth check will validate references and structural support. breadth accept will validate current executed behavior against independently computed identity. Until these commands are implemented, run the existing spec gate and structural document audits; never imply a future command has run.

No required missing, skipped, stale or empty suite can establish acceptance, including exit-zero runs. Historical evidence remains historical. Full-program acceptance requires all in-scope obligations, or explicit owner-approved deferrals; a foundation spec can be accepted on its own while downstream work remains planned.

Tested-input identity includes relevant source/spec/config/lock/tests/binary/platform and handles dirty content. Evidence outputs themselves are excluded so adding a record does not invalidate its tested inputs. Retain replayable fixture/setup/environment identity, stdout/stderr and file-effect assertions. Use configured developer caches and D:\BuildArtifacts tool/project directories for disposable Windows artifacts.

## Task generation and completion

After each design is approved, generate fresh tasks for that spec using the named component boundaries and deliverable gates. The archived 53-node monolithic tasks are superseded, not complete. Do not copy their dependency order into the split.

Seed/update the ledger during task generation and link actual tests during implementation. Multi-task capabilities remain partial until all obligations are implemented; only current required execution evidence can support passed. Do not invoke unavailable code/evidence commands before their foundation milestone exists.

The five task plans contain 100 executable subtasks in 38 groups, ordered sequentially in the foreground with explicit component boundaries and milestone gates. `.kiro/verification-suites.toml` declares five planned journey entrypoints with empty required-test sets; it is not a runnable registry until verification-evidence validates implemented tests and exact identities. Breadth capabilities remain planned. See `../specs/code-constraints/task-plan-validation.md` for coverage and graph checks.

The runner will use nextest for native component/journey partitions and cargo test --doc separately, with no duplicate suites. Scope checks during development, common joins at integration boundaries, and full release acceptance only at the corresponding milestone.

## Retained vision

structure-constraints: file/architecture/call cycles, layers, new dependencies, fan-in/out, coupling and reachable complexity.
lean-code: unused routines/parameters/classes/variables, pass-through, single implementation and over-export.
Separate future specs retain intentions/precedence, supersession/envelope comparison, production-code separation, environment doctor, vocabulary labeler and plugin report readout. These remain deferred, with no current completion claim.

The complete original-criterion migration is in ../specs/code-constraints/split-map.md. No original requirement is silently dropped by the split.

## Owner decisions, 2026-11-10

- scitools-hook will be archived: it depends on a licensed analyzer, is not built for spec-driven work and is too complex. Its ideas stay; compatibility with it is no longer a goal. Report import, Understand metric identities and SequenceMatcher parity are candidates for removal (review items A1-A5). Removing them changes approved requirements and waits for owner approval.
- Required platforms are Windows, Linux and macOS, with open-source dependencies only.
- `../lesscode` completes the stack. This project prevents (limits, ratchet, attribution per task); lesscode reduces (measured before/after receipts). Neither repeats the other: no equivalence or mutation checks here, no spec attribution there. File size ceilings overlap with `lesscode-core budgets` until code-constraints M1 ships.
- The vision and the five verification levels are in `docs/vision.md`. Review findings on risks, unstated assumptions and reuse are in `../specs/code-constraints/design-review-2026-11-10.md`; its proposal is one joint G1 prototype for the parse layer before the three analyzer task plans are approved.

## Owner decisions applied, 2026-11-10 (second round)

- Approved: scitools-hook report import and scorer parity are removed. code-constraints 1.2 is now the report-source contract with no base adapter; clone-constraints 1.2 and 1.6 no longer name SequenceMatcher or scitools measurements. In exchange code-constraints gains Requirement 13: an ASCII hook summary of improvements and regressions (`code check --summary`) and an optional JSONL metric log (`--metrics-log`). The gate installs no hook.
- Parsing is always adopted. No lexer or parser for source code is written in this program; analyzers write queries and counting rules over a library's syntax tree. G1 is one joint prototype for code-constraints, clone-constraints and code-locality-data.
- Implementation may run in the foreground or through subagents, whichever fits the task; Fable 5.2 is an accepted subagent model. Shared integration files stay serialized.
- Archiving the scitools-hook repository is handled by the owner outside this project.
- Also applied from the review: macOS added to required platforms, line endings normalized in the source bundle, and a move rule so that splitting a file keeps the debt status of moved routines.

The three analyzer task plans were patched for these changes, not regenerated. They remain unapproved and should be regenerated after the joint G1 result.

## Quality telemetry (owner request, 2026-11-10)

A sixth spec, quality-telemetry, replaces the metric log file. The engine reduces gate reports to a pure metric snapshot; the gate writes gauges, one log record per finding and one trace per run as OTLP JSON lines into a directory outside the repository, and an OpenTelemetry Collector (or a plain HTTP post) ships them; OpenObserve is the reference backend. The owner chose files over an in-process exporter after the prototype measured 127 dependencies and a 32 second stall on an unreachable backend. Four dashboard definitions are committed: developer, architect, code health, and specification health beside code health per spec. Writing is off by default, opens no network connection and never changes a verdict.

It depends only on cli-contracts. Its first deliverable exports today's specification and traceability metrics, so it can ship before any analyzer. Analyzer specs contribute points through its provider contract; code-constraints Requirement 13 now does this and keeps only the ASCII hook summary as local output. Ingestion is verified by prototype; the spec is approved.

## Priority change (owner, 2026-11-10): complete the scitools-hook port first

- Dashboards (quality-telemetry 5.2 to 5.4, 6.2) go to the backlog. The telemetry writer, Collector example and catalogue are done; analyzers contribute metric points through the provider contract as they ship.
- The port is the three analyzer specs: code-constraints, then clone-constraints and code-locality-data. Work starts with the joint G1 prototype for the parse layer, then the analyzer task plans are regenerated from its result.
- Prerequisites relaxed to keep the port unblocked: code-constraints starts on cli-contracts C1 (done) and the existing real-binary test helper. The remaining cli-contracts migrations and verification-evidence follow the port instead of preceding it; journeys written for the port are registered with the evidence runner when it exists. The breadth ledger stays planned meanwhile.
