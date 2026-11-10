# Clone Constraints

## Problem and Outcome

Owns duplicate blocks, similar-routine families, clone-source adapters and clone quality/performance acceptance. Consumes measurement, coverage, ownership and report contracts; does not own metric baselines or semantic-equivalence claims.

## Boundary Candidates

- CloneModel: Clone ranges, token shapes and detector identity.
- CloneRules: Pure family grouping, exclusions and findings.
- CloneSources: Adopted duplicate detector and shape extraction.
- CloneConfig: Typed clone settings and catalogue fragment.
- CloneTests: Reuse comparison and differential corpus.
- CloneDocs: Detector definitions, thresholds and limitations.

## Upstream / Downstream

- Upstream: code-constraints.
- Downstream: user and CI acceptance.
- Acceptance: the named deliverables in design.md, each exercised through its own journey and current evidence.

## Existing Spec Touchpoints

Consumes the existing core model/ownership join and cli-gate boundary; shared registration edits are explicit and serialized. See [program strategy](../../steering/code-quality-strategy.md) and [criterion migration](../code-constraints/split-map.md).

## Status

Requirements amended under the user's 2026-11-09 instruction; design generated for approval. No implementation tasks are authorized by this document. The historical monolithic plan is preserved under code-constraints/history/2026-11-09-monolith and is superseded.
