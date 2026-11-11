# Vision: verified spec-driven development

Spec-driven development with coding agents fails quietly in three places. This project puts a check at each of them, so that a specification is something a machine can verify and not only something an agent was asked to follow.

The workflow underneath is [cc-sdd](https://github.com/gotalab/cc-sdd) in the kiro layout: requirements, then design, then tasks, then implementation, with a human approval between phases. This project adds the verification. It does not run agents.

## The three quiet failures

1. The requirements disagree with themselves. Two criteria can both apply in some state and demand opposite effects, or the same word means two things in two documents. Nobody notices until the code is written.
2. "Done" is a claim. A task is checked off and a test file exists, but no run shows that the required tests were collected, executed and passed on the code as it is now.
3. The code degrades. Each task is reviewed alone. Routines grow, files collect unrelated responsibilities, logic is copied, and the same constant or state value is spelled out in many files. After some months agents fail on a code base that agents produced.

## The verification ladder

Each level answers one question. A level is useful without the levels above it.

| Level | Question | Mechanism | Status |
|---|---|---|---|
| 1. Consistent requirements | Can two criteria contradict each other? Does every term have one meaning? | EARS shape, topic-scoped vocabulary, contradiction search on binary decision diagrams with a witness state | Implemented. [Verification strategy](verification.md), [checks](checks.md), [vocabulary](vocabulary.md) |
| 2. Traceable work | Which criterion, task and design component owns this file? | Join over the design's File Structure Plan and task boundaries; navigation and graphs in the IDE and the CLI | Implemented. [CLI](cli.md) |
| 3. Executed evidence | Which tests prove this criterion, and did they pass on the current inputs? | Capability ledger `.kiro/breadth.toml`, real-binary journeys, acceptance that refuses missing, skipped or stale runs | Specified, not implemented. Specs `cli-contracts`, `verification-evidence` |
| 4. Maintainable code | Did this task make the code harder to change? | Routine and file limits, a ratchet against the previous state, duplicate blocks and similar routines, scattered definitions and state, role-named data access, each finding attributed to its spec and task | Implemented in part: `kiro-cli code check`. [Code rules](code-rules.md). Not built: grounding of column names in the vocabulary, scattered schema, SARIF output, strict mode |
| 5. Less code | Can existing code be replaced by something the framework already provides, with the same behaviour? | Measured before and after comparison with receipts | Separate project: [lesscode](https://github.com/norandom/lesscode) |

Level 1 is where formal verification works today. Acceptance criteria written in EARS are grounded in a declared vocabulary, turned into boolean guards over finite regions, and checked for joint satisfiability. The result is exact for what it covers and it refuses when a problem is too large.

Level 4 runs as `kiro-cli code check`. Its routine limits block in Rust and Python, where they were calibrated on six repositories; in TypeScript and JavaScript they warn. Level 3 is designed and reviewed but not built, and no command for it exists yet. The open design questions are recorded in the [design review of 2026-11-10](../.kiro/specs/code-constraints/design-review-2026-11-10.md).

## Seeing the levels together

The spec `quality-telemetry` writes a gate run as OpenTelemetry signal files, keyed by spec, which an OpenTelemetry Collector ships to OpenObserve. The writer, the metric catalogue and a Collector example exist ([Telemetry](telemetry.md)). Writing is off by default, opens no network connection and never changes a gate verdict. Four dashboards are planned and not built: one for a developer (what did my task change), one for an architect (which spec or component is drifting), one for code health over time, and one that puts specification health beside code health per spec.

## What each level does not show

- Level 1 compares criteria that share a topic. It does not show that the requirements are the right ones.
- Level 2 follows the file plan in the design. A file the design does not mention has no owner.
- Level 3 will show that named tests passed on named inputs. It will not show that the tests are sufficient.
- Level 4 measures structure. A routine within every limit can still be wrong or unclear.
- Level 5 compares finite recorded behaviour. It is not an equivalence proof.

## Where the checks sit in the agent workflow

| Workflow step | Check |
|---|---|
| Requirements written | `kiro-cli check` on the spec: EARS, terms, contradictions |
| Tasks generated | Planned capabilities entered in the ledger (level 3, planned) |
| A task is implemented | `kiro-cli code check --task <spec>:<id>`: the task's files against the limits, with hints the agent can act on |
| Before a commit | `kiro-cli code check --changed HEAD --local`: the changed files on their own |
| Before a push and in CI | `kiro-cli code check --changed origin/main`: also the rules that read the whole project |
| Implementation validated | Acceptance of current evidence (level 3, planned) |
| Later, on grown code | A lesscode reduction, driven by the findings of level 4 |

## Two projects, one stack

This project prevents. lesscode reduces. The division is deliberate and neither repeats the other.

| | openidea-kiro | lesscode |
|---|---|---|
| Acts on | New and changed code, per task | Existing code, per capability |
| Asks | Is this change within the agreed limits, and who owns it? | Is this rewrite a substantial reduction with the same behaviour? |
| Output | Findings with a spec, a task and a hint | Receipts for a before and after pair |
| Does not do | Judge behavioural equivalence | Gate on policy or attribute to specs |

The handoff runs one way. A level 4 finding with a `shrink:`, `delete:` or `collect:` hint names a candidate. lesscode measures whether the replacement is a real reduction. Its receipt can be cited as evidence in the ledger.

lesscode ended its own policy engine for this reason: a per-function budget cannot show that a capability became simpler. The budget belongs in the gate, the proof of simplification belongs in lesscode.

## Origin of the code rules

The level 4 rules come from `scitools-hook`, an earlier project by the same author that gated agent output with a commercial static analyzer. Its measurements showed which rules agents respond to. The project is being archived: it depends on a licensed tool, it was not built for spec-driven work, and it grew too complex. The ideas are carried over here on open-source parsers that run on Windows, Linux and macOS. Compatibility with scitools-hook is not a goal.

## The hypothesis

Limits on size, complexity, duplication and locality, reported per task with a concrete hint, keep a code base workable for agents. This is a hypothesis. It is wrong if agents satisfy the limits by moving code around while review effort stays the same, and the program should be judged against that.
