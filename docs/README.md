# Documentation

Start with the [vision](vision.md) if you want to know what this project verifies and what it does not. The tables below are ordered by what you want to do. Every page of this directory is listed once.

## Get it running

| You want to | Read |
|---|---|
| Install a prebuilt binary or the editor plugin | [README: install from a release](../README.md#install-from-a-release) |
| Build from source, build the plugin, run the quality gate, cut a release | [Build guide](build.md) |
| Look up a command, an option, an environment variable or an exit code | [CLI reference](cli.md) |
| Run the gate in CI and read the JSON report | [CI integration](ci.md) |

## Check a specification

| You want to | Read |
|---|---|
| Understand a finding code of `kiro-cli check` | [Constraint checks](checks.md) |
| Declare terms and topics, or import a vocabulary | [Vocabulary guide](vocabulary.md) |
| Understand how contradictions are found and why the search is bounded | [Verification strategy](verification.md) |
| Use the commands inside the OpenIDEA editor | [Plugin guide](../plugins/kiro/README.md) |

## Check code

| You want to | Read |
|---|---|
| Understand a finding code of `kiro-cli code check`, the limits and what blocks | [Code rules](code-rules.md) |
| Change a limit, set a rule's severity or turn a rule off | [Code rules: configuration](code-rules.md#configuration) |
| Choose what runs in a pre-commit hook, before a push and in CI | [Code rules: in a hook and in CI](code-rules.md#in-a-hook-and-in-ci) |
| Write gate runs as OpenTelemetry files and look up a metric | [Telemetry](telemetry.md) |

## Understand the direction

| You want to | Read |
|---|---|
| See all verification levels, their status and what each does not show | [Vision](vision.md) |
| See how this project and lesscode divide the work | [Vision: two projects, one stack](vision.md#two-projects-one-stack) |
| Follow the owner decisions on the code-quality work | [Code-quality strategy](../.kiro/steering/code-quality-strategy.md) |
| See which scitools-hook ideas were carried over, and the calibration of the limits | [Research log of code-constraints](../.kiro/specs/code-constraints/research.md) |
| Read the open risks and reuse decisions | [Design review, 2026-11-10](../.kiro/specs/code-constraints/design-review-2026-11-10.md) |

## Figures

| Figure | File |
|---|---|
| Inputs, program parts and outputs | [figures/architecture.svg](figures/architecture.svg) |

## Specifications

Every feature has a spec under `.kiro/specs/<name>/` with requirements, design and tasks. The [roadmap](../.kiro/steering/roadmap.md) lists them in dependency order.

| Level | Specs | Status |
|---|---|---|
| 1. Consistent requirements | `kiro-core`, `vocabulary-grounding`, `spec-constraints` | Implemented |
| 2. Traceable work | `plugin-core`, `spec-navigation`, `spec-visualization`, `cli-gate` | Implemented |
| 3. Executed evidence | `cli-contracts`, `verification-evidence` | Shared command-line options implemented; evidence runner specified |
| 4. Maintainable code | `code-constraints`, `clone-constraints`, `code-locality-data` | Implemented in part: see [Code rules](code-rules.md) |
| Across levels | `quality-telemetry` | File writer and metric catalogue implemented; dashboards in the backlog |

No document here describes a planned command as available. Where a page covers planned work it says so at the top.
