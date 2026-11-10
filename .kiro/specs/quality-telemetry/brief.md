# Quality Telemetry

## Problem and Outcome

The gate reports are read once and discarded. A metric log file in the repository was proposed and rejected by the owner: it grows, conflicts across branches and cannot be charted. The outcome is every gate run exported as OpenTelemetry signals to OpenObserve, keyed by spec, with dashboards for a developer, an architect and a code-health reader, and one view that puts specification health beside code health.

## Boundary Candidates

- MetricSnapshot: points, coverage and the provider contract.
- MetricCatalogue: names, units, attributes and sources.
- SpecMetrics: specification and traceability points from existing reports.
- OtlpJson: OTLP JSON lines from snapshot, findings and timing.
- Shipping: Collector example for the telemetry directory.
- TelemetryCommand: command, enablement and failure behaviour.
- Dashboards: four OpenObserve definitions.

## Upstream / Downstream

- Upstream: cli-contracts.
- Downstream: code-constraints, clone-constraints, code-locality-data and verification-evidence contribute points through the provider contract.
- Acceptance: the named deliverables in design.md, each exercised through its own journey.

## Existing Spec Touchpoints

Replaces the metric log of code-constraints Requirement 13, which now hands points to this contract. Reads the existing check report, status and ownership join without changing them. See [program strategy](../../steering/code-quality-strategy.md) and [vision](../../../docs/vision.md).

## Status

Approved by the owner on 2026-11-10. The same day the owner chose OTLP JSON files shipped by a Collector over an in-process exporter; requirements, design and tasks were revised accordingly. Ingestion by OpenObserve is verified by prototype (research.md).
