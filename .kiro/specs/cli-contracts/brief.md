# Common CLI Contracts

## Problem and Outcome

Owns shared parameter groups, command capability profiles, compatible migration, output routing and exit conventions. Does not own individual code rules, evidence semantics or external analyzers.

## Boundary Candidates

- CliGroups: Shared parameter and exit types.
- CliMigration: Command profiles, group composition and legacy flags.
- CliConformance: Every profile and legacy invocation regression.

## Upstream / Downstream

- Upstream: cli-gate.
- Downstream: verification-evidence, code-constraints.
- Acceptance: the named deliverables in design.md, each exercised through its own journey and current evidence.

## Existing Spec Touchpoints

Consumes the existing core model/ownership join and cli-gate boundary; shared registration edits are explicit and serialized. See [program strategy](../../steering/code-quality-strategy.md) and [criterion migration](../code-constraints/split-map.md).

## Status

Requirements amended under the user's 2026-11-09 instruction; design generated for approval. No implementation tasks are authorized by this document. The historical monolithic plan is preserved under code-constraints/history/2026-11-09-monolith and is superseded.
