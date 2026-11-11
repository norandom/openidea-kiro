# Implementation Plan

Generated on 2026-11-10 at the owner's request and approved the same day. Revised the same day after the owner chose OTLP JSON files over an in-process exporter; the revision follows that decision and the prototype record in research.md.

## Execution contract

Requires cli-contracts C1 (shared groups and the migrated check) from task 2.3 on. It does not require verification-evidence or any analyzer spec. Shared registrations (core `lib.rs`, CLI dispatch) are serialized through their existing owners. Use the design's File Structure Plan for file ownership and the configured build and cache directories for disposable artifacts. The telemetry directory used in tests and examples is never inside the repository.

A task completes only with its observable result and fresh scoped verification. Keep `.kiro/breadth.toml` planned until implementation supplies real tests and current acceptance. The planned journey entrypoint is declared in [the suite registry](../../verification-suites.toml).

The struct names for existing report, status and join types in the design are placeholders; use the types the core already exports.

## Backlog (owner, 2026-11-10)

Dashboards are deferred: tasks 5.2, 5.3 and the dashboard parts of 5.4 and 6.2 wait until the scitools-hook port (code-constraints, clone-constraints, code-locality-data) is complete. The writer, the Collector example and the catalogue are done and stay as they are; analyzer specs add their metric points through the provider contract meanwhile.

## Milestone gates

| Milestone | Complete through task | Observable deliverable |
|---|---|---|
| T0 | 1.2 | Files ingested by pinned Collector and backend; decoder accepts the lines |
| T1 | 2.3 | Snapshot of this repository printed by the dry run |
| T2 | 4.1 | Spec and traceability metrics written by the real binary and decoded |
| T3 | 6.2 | Collector example, four dashboards, consistency test, three platforms |

## Tasks

- [x] 1. Prove ingestion before building
- [x] 1.1 Send the three signal kinds to a pinned backend
  - Sent one gauge, one log record and one trace to a local OpenObserve, first with the exporter crates, then as hand-written OTLP JSON lines through a direct HTTP post and through the Collector's JSON file receiver.
  - Recorded endpoint paths, authentication, the streams and field names the backend creates, the dependency cost of the exporter crates and the 32 second stall on an unreachable endpoint.
  - Done: all three signals were queryable on all three paths; see the T0 records in research.md.
  - _Boundary: Shipping_
  - _Requirements: 6.1_

- [x] 1.2 Confirm the decoder and the dependency boundary
  - Decode the prototype's three lines with the official protocol definitions and their serde support as a development dependency; record the crate version and feature set.
  - Confirm that the runtime dependency graph of the CLI and the core sandbox-target build are unchanged by that development dependency.
  - Done when the decode test passes on the recorded lines and a deliberately malformed line fails it; if the definitions cannot decode the JSON encoding, stop and return to design.
  - _Boundary: TelemetryTests_
  - _Depends: 1.1_
  - _Requirements: 6.1, 6.2_

- [x] 2. Build the snapshot from existing reports
- [x] 2.1 Implement the snapshot model and provider contract
  - Implement points, attributes, units, coverage, stable ordering and the provider trait as pure data with no clock, network or filesystem.
  - Reject unknown names, unlisted attribute keys, nonfinite values and duplicates; list an expected source without a provider as absent.
  - Done when unit tests show equal snapshots for equal input, an absent source in coverage with no zero point, and a second provider added without touching the mapping.
  - _Boundary: MetricSnapshot_
  - _Depends: 1.2_
  - _Requirements: 1.1, 1.3, 1.4, 1.5, 1.6_

- [x] 2.2 Implement the catalogue and its documentation check
  - Declare every metric with unit, allowed attributes, source and meaning, the reserved prefixes of later providers and the backend stream and field names recorded in the prototype.
  - Write the catalogue section of the telemetry documentation from the same table and test their equality.
  - Done when a metric added to code without documentation, or the reverse, fails the test.
  - _Boundary: MetricCatalogue, TelemetryDocs_
  - _Requirements: 1.2, 1.4_

- [x] 2.3 Derive specification and traceability points
  - Map phase, approvals, task counts, findings by code and severity, term and contradiction counts, checked criteria and the four traceability gaps from the values the core already returns.
  - Add the dry run to the telemetry command so the snapshot of a tree can be read as text or JSON.
  - Done when the fixture tree's expected snapshot matches and the dry run over this repository prints one row per spec and writes nothing.
  - _Boundary: SpecMetrics, TelemetryCommand, TelemetryTests_
  - _Requirements: 1.3, 2.1, 2.2, 2.3, 2.4, 2.5, 3.6_

- [x] 3. Write the signal files
- [x] 3.1 Map snapshot, findings and timing to OTLP JSON lines
  - Serialize points as gauges, findings as log records with the record cap and dropped count, and one trace per run with a span per phase, as one request object per signal.
  - Keep the function pure: resource, timestamps, durations and identifiers are inputs.
  - Done when every produced line decodes with the official definitions into exactly the expected gauges, records and spans for a fixture input, including the cap case.
  - _Boundary: OtlpJson, TelemetryTests_
  - _Depends: 2.3_
  - _Requirements: 3.2, 4.1, 4.2, 4.3, 6.2_

- [x] 3.2 Build resource identity and redaction
  - Attach service name and version, repository, branch, revision, run kind and the operator's resource attributes; read Git through a read-only subprocess with separate arguments and omit its values when absent.
  - Strip credentials from the repository address; write only catalogue attributes and listed resource keys; read no other environment value.
  - Done when tests find no absolute path, user name, credential or other environment value in any line, with and without Git.
  - _Boundary: TelemetryCommand, OtlpJson_
  - _Requirements: 3.7, 3.8_

- [x] 3.3 Integrate the directory, enablement and failure behaviour into the CLI
  - Add `telemetry export`, `--telemetry-dir` and `KIRO_TELEMETRY_DIR` on check through its command profile; time the existing phases with a monotonic clock and derive run identifiers without a new dependency.
  - Refuse a directory inside the project root and an invalid run kind as usage errors before the gate runs; append one whole line per file after the report and exit decision; report write failures on standard error only.
  - Done when reports are byte-identical with telemetry on and off, no network code is linked for telemetry, and parallel runs leave only whole lines.
  - _Boundary: TelemetryCommand_
  - _Depends: 3.1, 3.2_
  - _Requirements: 3.1, 3.3, 3.4, 3.5, 4.3, 4.4, 6.2_

- [x] 4. Deliver the first exported view
- [x] 4.1 Prove the export through the real binary
  - Drive the built binary over a fixture project and over this repository with a temporary telemetry directory; decode every line and assert points, records and spans.
  - Repeat with an unwritable directory and with a directory inside the project; assert the gate's own exit code in the first case and the usage error in the second.
  - Done when the journey passes and removing the telemetry directory setting makes its decoding assertion fail.
  - _Boundary: TelemetryTests_
  - _Depends: 3.3_
  - _Requirements: 3.4, 3.5, 4.4, 6.3, 6.5_

- [ ] 5. Provide shipping and dashboards
- [x] 5.1 Provide the Collector example and ship real files
  - Write the Collector example with the JSON file receiver and the HTTP exporter, credentials from the Collector's environment; load it in the pinned Collector against the pinned backend.
  - Ship the files written in 4.1 through the Collector and, separately, by direct post.
  - Done when this repository's spec and traceability metrics are queryable in the backend by both paths.
  - _Boundary: Shipping_
  - _Depends: 4.1_
  - _Requirements: 6.1, 6.6_

- [ ] 5.2 Author and commit the four dashboard definitions
  - Build the developer, architect, code-health and side-by-side views in the pinned backend from the shipped data; restrict trend panels to `ci` runs of the default branch using the per-signal field names.
  - Leave panels of sources that send nothing in the backend's no-data state; export the definitions and commit them.
  - Done when the definitions import into a fresh backend and show this repository's spec and traceability data.
  - _Boundary: Dashboards_
  - _Depends: 5.1_
  - _Requirements: 5.1, 5.3_

- [ ] 5.3 Test dashboards against the catalogue
  - Extract stream, metric and field names from each definition's queries and compare them with the catalogue; reject a query that turns a missing value into zero.
  - Done when renaming a metric without updating a dashboard fails the test.
  - _Boundary: TelemetryTests, Dashboards_
  - _Requirements: 5.2, 5.3_

- [ ] 5.4 Document setup and use
  - Describe backend setup, the Collector and direct-post paths, file growth and removal after shipping, environment variables, use in hooks, agents and CI, dashboard import, the trend rule and what each view answers.
  - Done when a reader can reach a populated side-by-side view from a clean checkout by following the page.
  - _Boundary: TelemetryDocs_
  - _Requirements: 1.2, 5.4_

- [ ] 6. Integrate across platforms and CI
- [ ] 6.1 Run the journey on the three platforms
  - Run the decode journey and the dependency-boundary checks on Windows, Linux and macOS through the existing runner.
  - Done when all three pass and the runtime dependency graph and sandbox build are unchanged on each.
  - _Boundary: TelemetryTests_
  - _Depends: 5.3_
  - _Requirements: 6.2, 6.4_

- [ ] 6.2 Write telemetry in the default-branch pipeline
  - Add an opt-in step with run kind `ci` that writes to a directory outside the checkout and ships it with the pipeline's own secrets, never failing the pipeline.
  - Done when a default-branch run appears in the trend panels and a run without secrets skips shipping visibly.
  - _Boundary: TelemetryCommand, TelemetryDocs_
  - _Depends: 6.1_
  - _Requirements: 3.1, 3.4, 5.4, 6.5_
