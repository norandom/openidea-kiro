# Bounded design validation

Reviewed on 2026-11-09. Scope: the amended code-quality program and its five active designs. This report records the design-generation snapshot before the subsequent user instruction to produce tasks. Current phase/approval state and the suite-registry clarification are recorded in [task-plan-validation.md](task-plan-validation.md); the counts and pending approvals below describe the earlier snapshot. No prototype results are approved by this report.

## Outcome

PASS for design generation and structural consistency. The designs define ownership, exclusions, dependencies, typed contracts, file plans, acceptance traceability, first observable deliverables and prototype gates. This is not a runtime correctness or implementation-completion verdict.

| Specification | Acceptance criteria | Components | Planned files | Scoped gate errors / warnings |
|---|---:|---:|---:|---|
| [cli-contracts](../cli-contracts/design.md) | 11 | 3 | 11 | 0 / 0 |
| [verification-evidence](../verification-evidence/design.md) | 27 | 8 | 17 | 0 / 0 |
| [code-constraints](design.md) | 74 | 13 | 27 | 0 / 0 |
| [clone-constraints](../clone-constraints/design.md) | 15 | 6 | 9 | 0 / 0 |
| [code-locality-data](../code-locality-data/design.md) | 21 | 8 | 10 | 0 / 0 |

## Checks performed

- Each of the 108 archived acceptance criteria has exactly one primary successor, with additional references for composite obligations, in [criterion-migration.json](criterion-migration.json).
- All 148 active criteria have exactly one design traceability row. The 25 capability entries in [breadth.toml](../../breadth.toml) cover these criteria exactly and remain planned, with no claimed test execution or evidence.
- File-plan ownership has no cross-spec duplicate paths. Shared integration points have an upstream owner and serialized integration procedure in [the strategy](../../steering/code-quality-strategy.md).
- All roadmap dependencies resolve and the graph is acyclic. Relative document links resolve; active designs contain no unfinished template placeholders.
- Existing `kiro-cli check --project . --feature <name> --fail-on warning --format json` passed for each of the five specs. Informational findings remain. Separate status checks confirm that task plans have deliberately not been generated at this phase.
- Whole-workspace checking reported zero errors and 117 warnings in other specifications. Those warnings prevent a whole-workspace warning-level pass and are outside this amendment.
- `git diff --check` and a separate check of new documents were used to verify whitespace and conflict-marker hygiene.

Machine-readable results and the one-off structural audit are in `D:/BuildArtifacts/openidea-kiro/spec-design-20261109/`. These local validation artifacts are not portable capability evidence and are not registered as passing product tests.

## Inline review and repairs

The review checked reuse before custom implementation, fidelity to sparse scitools measurements, metric provenance/comparability, clone-family semantics, advisory limits for new locality/data heuristics, independent execution evidence, and shared-file ownership. Refinements added explicit upstream metric mappings, input-change detection, exact required-test accounting, compatible CLI output routing and independently acceptable foundation scopes. Four new EARS multi-obligation warnings were repaired; fresh scoped checks then passed.

The archived monolith is retained under [history](history/2026-11-09-monolith/README.md); its task plan is superseded. All five metadata files record `design-generated`, approved requirements, unapproved designs, tasks not generated and `ready_for_implementation: false`.

## Remaining gates and next step

Human design review precedes fresh task generation, beginning with CLI contracts and verification evidence. Task plans must place pinned dependency prototypes and a real-binary fail/repair/pass slice before broad rule implementation. G1-G7 remain unexecuted: dependency feasibility, detector quality, performance, Windows/Linux runtime behavior, evidence rejection cases and rollout are implementation work. No dependency was adopted, runtime implementation changed or future end-to-end test claimed as passing in this design pass.
