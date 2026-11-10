# Code Locality and Named Data Access

## Problem and Outcome

Owns scattered definitions/state/schema and vocabulary-backed named data access. Consumes confined parsing, coverage, ownership and vocabulary resolution. Does not infer semantic equivalence, implement dependency graphs or extend IDE behavior.

## Boundary Candidates

- LocalityFacts: Bindings, comparisons, qualified enum evidence and data access facts.
- LocalityExtract: Supported native extraction over shared source texts.
- DataPatterns: Receiver-aware pattern matching and custom-query validation.
- LocalityRules: Scattered-definition/state/schema aggregation.
- DataGrounding: Positional and topic-scoped data-name policy.
- LocalityConfig: Rule settings, ignores, reasons and default severities.
- LocalityTests: Labeled patterns and ambiguity corpus.
- LocalityDocs: Patterns, heuristic limits and calibration.

## Upstream / Downstream

- Upstream: code-constraints, vocabulary-grounding.
- Downstream: user and CI acceptance.
- Acceptance: the named deliverables in design.md, each exercised through its own journey and current evidence.

## Existing Spec Touchpoints

Consumes the existing core model/ownership join and cli-gate boundary; shared registration edits are explicit and serialized. See [program strategy](../../steering/code-quality-strategy.md) and [criterion migration](../code-constraints/split-map.md).

## Status

Requirements amended under the user's 2026-11-09 instruction; design generated for approval. No implementation tasks are authorized by this document. The historical monolithic plan is preserved under code-constraints/history/2026-11-09-monolith and is superseded.
