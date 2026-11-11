# Documentation

Start with the [vision](vision.md) if you want to know what this project verifies and what it does not. The table below is ordered by what you want to do.

## Use what works today

| You want to | Read |
|---|---|
| Install the CLI or the IDE plugin | [README](../README.md#installation) |
| Run the spec checks locally | [CLI reference](cli.md) |
| Run them in CI and read the JSON report | [CI integration](ci.md) |
| Understand a finding code | [Constraint checks](checks.md) |
| Declare terms and topics, or import a vocabulary | [Vocabulary guide](vocabulary.md) |
| Understand how contradictions are found and why the search is bounded | [Verification strategy](verification.md) |
| Look up a telemetry metric (catalogue exists; export is in progress) | [Telemetry](telemetry.md) |

## Understand the direction

| You want to | Read |
|---|---|
| See all verification levels and their status | [Vision](vision.md) |
| See how this project and lesscode divide the work | [Vision: two projects, one stack](vision.md#two-projects-one-stack) |
| Follow the delivery order of the planned code-quality specs | [Code-quality strategy](../.kiro/steering/code-quality-strategy.md) |
| Read the open risks and reuse decisions | [Design review, 2026-11-10](../.kiro/specs/code-constraints/design-review-2026-11-10.md) |
| See which scitools-hook ideas were carried over | [Feature map in the research log](../.kiro/specs/code-constraints/research.md) |

## Specifications

Every feature has a spec under `.kiro/specs/<name>/` with requirements, design and tasks. The [roadmap](../.kiro/steering/roadmap.md) lists them in dependency order.

| Level | Specs | Status |
|---|---|---|
| 1. Consistent requirements | `kiro-core`, `vocabulary-grounding`, `spec-constraints` | Implemented |
| 2. Traceable work | `plugin-core`, `spec-navigation`, `spec-visualization`, `cli-gate` | Implemented |
| 3. Executed evidence | `cli-contracts`, `verification-evidence` | Specified |
| 4. Maintainable code | `code-constraints`, `clone-constraints`, `code-locality-data` | Specified |
| Across levels: dashboards | `quality-telemetry` | Specified |

No document here describes a planned command as available. Where a page covers planned work it says so at the top.
