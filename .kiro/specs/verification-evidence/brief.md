# Breadth and Execution Evidence

## Problem and Outcome

Owns capability planning, reference validation, execution evidence acceptance and a reusable real-binary harness/runner. Consumes feature journeys without owning their business assertions or requiring code analysis to exist.

## Boundary Candidates

- BreadthLedger: Parse ledger and resolve criteria/status/path declarations.
- EvidencePolicy: Pure execution-claim validation.
- BreadthCommand: Ledger and acceptance subcommand adapters.
- EvidenceInputs: Confined manifest/artifact reads and independent identity calculation.
- JourneyHarness: Reuse fixture setup and binary execution.
- EvidenceTests: Ledger lifecycle and false-claim journeys.
- VerificationRunner: Portable acceptance orchestrator with explicit suite selection.
- EvidenceDocs: Schema, statuses, collection, replay and scope policy.

## Upstream / Downstream

- Upstream: cli-contracts.
- Downstream: code-constraints.
- Acceptance: the named deliverables in design.md, each exercised through its own journey and current evidence.

## Existing Spec Touchpoints

Consumes the existing core model/ownership join and cli-gate boundary; shared registration edits are explicit and serialized. See [program strategy](../../steering/code-quality-strategy.md) and [criterion migration](../code-constraints/split-map.md).

## Status

Requirements amended under the user's 2026-11-09 instruction; design generated for approval. No implementation tasks are authorized by this document. The historical monolithic plan is preserved under code-constraints/history/2026-11-09-monolith and is superseded.
