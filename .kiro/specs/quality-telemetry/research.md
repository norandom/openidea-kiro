# Research & Design Decisions

## Summary

Owner request, 2026-11-10: the performance and metric report is a case for OpenTelemetry and OpenObserve; a JSONL file in the repository is a poor fit; side-by-side dashboards of codebase and specification KPIs are wanted for developer, architect and code-health readers. This spec is the result. Nothing here was prototyped.

## Findings

- `cargo search` on 2026-11-10: `opentelemetry`, `opentelemetry_sdk` and `opentelemetry-otlp` are at 0.33.1. Exact versions, HTTP client and feature set are pinned in T0.
- OpenObserve is expected to accept OTLP over HTTP for metrics, logs and traces and to export dashboards as JSON. Endpoint paths, authentication and the created stream and field names were not verified from this workstation; T0 records them against a pinned release.
- The core already returns everything the first deliverable needs: findings, term and contradiction counts, status and the ownership join coverage. No new check is required.
- The core has no clock. Run and phase durations must be measured in the CLI.
- The plugin sandbox has no network, so only the CLI can export.

## Decisions

- **Three signals, by fit.** Gauges for bounded aggregates, one log record per finding for detail, one trace per run for timing. Per-routine detail as metric attributes would create unbounded series.
- **Pure snapshot, native export.** Follows the existing renderer pattern and keeps the sandbox build unchanged. The exporter sits behind a build feature because the OpenTelemetry crates bring an HTTP client and possibly an async runtime.
- **Adopt, do not build.** Official OpenTelemetry crates for the protocol, OpenObserve for storage and charts, standard `OTEL_` variables for configuration. No custom protocol, store, agent or chart code. An OpenTelemetry Collector between the CLI and the backend remains possible without a change here.
- **Absent is not zero.** A source that did not run sends nothing and is listed in coverage, consistent with the trust-signal rule of the gates.
- **Export never changes a verdict.** It runs after the exit decision; failure is one line on standard error. A hook must not fail because a backend is down.
- **Trends from `ci` runs of the default branch only.** Hook and agent runs are many and partial; they serve the developer view.
- **Dashboards as committed definitions**, tested against the catalogue so a renamed metric cannot silently empty a panel.
- **Counts, not ratios, are sent.** Ratios are derived in the dashboard so that aggregation across specs stays correct.

## Alternatives considered

- JSONL in the repository: rejected by the owner.
- Prometheus exposition file or push gateway: no finding detail and no traces; a second system beside OpenObserve.
- Writing OTLP JSON to a file for a Collector to pick up: avoids the dependency cost and stays possible as a fallback if T0 finds the crates too heavy; needs a Collector on every machine.
- Direct OpenObserve JSON ingestion API: simpler, but ties the tool to one backend.

## Risks

- Dependency weight and build time of the exporter; mitigated by the feature gate and the T0 cost record.
- Credentials in hooks and agent sessions; environment only, never printed.
- Many short-lived gauge series from branches; bounded by the attribute set and by trend panels reading the default branch.
- Dashboard format changes between OpenObserve releases; the release is pinned and the definitions are re-exported on upgrade.

## Consulted

kiro-spec-requirements, kiro-spec-design and kiro-spec-tasks conventions as used by the five code-quality specs; steering product, tech and structure; `code-quality-strategy.md`; the design review of 2026-11-10.

## T0 record, task 1.1 (2026-11-10, Windows 11, rustc 1.98.1)

Probe: a disposable crate outside the repository (`D:\BuildArtifacts\openidea-kiro\telemetry-t0`) sent one gauge, one log record and one two-span trace to a local OpenObserve container.

| Item | Observed |
|---|---|
| Backend | `public.ecr.aws/zinclabs/openobserve@sha256:26618f379c86b253c0d91624c487dd14b71293699845ecbc8457c94dca14819f`, commit `d16d693ba99bc20621a58b948ecbcad7ec7fa882`; the image reports an empty version string, so the digest is the pin |
| Crates | `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` all `=0.33.1`, default features off; otlp features `http-proto`, `reqwest-blocking-client`, `metrics`, `trace`, `logs` |
| Endpoint | `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:5080/api/<org>`; the crate appends `/v1/metrics`, `/v1/logs`, `/v1/traces` |
| Authentication | `OTEL_EXPORTER_OTLP_HEADERS=Authorization=Basic%20<base64 of user:password>`; read from the environment by the crate, no code |
| Result | All three exports returned Ok and all three signals were found by SQL search |
| Metrics stream | One stream per metric, dots become underscores: `kiro.spec.findings` is stream `kiro_spec_findings`; the value is field `value`; attributes become fields (`kiro_spec`, `kiro_code`, `kiro_severity`); resource attributes become fields (`service_name`, `kiro_run_kind`, `vcs_ref_head_name`) |
| Logs stream | `default`; message in `body`; `severity`, `severity_number`; attributes and resource attributes as fields with underscores |
| Traces stream | `default`; span name in `operation_name`; `duration` in microseconds; resource attributes prefixed `service_` (`service_kiro_run_kind`) |
| Dependency cost | 127 crates in the probe's normal dependency tree, including tokio through reqwest and hyper; probe release binary 2.8 MB; clean release build 21 s |

Findings that change the design:

- **An unreachable endpoint took 32 seconds**, not 5. The metric exporter reported a 5 second timeout; the log and trace providers returned Ok from shutdown although nothing was delivered, and the whole run took 32 seconds; which exporter spent the time was not isolated. The exporter's own timeout and return values are therefore not enough for criteria 3.4 and 6.3. The export needs one overall deadline enforced by this project, and delivery failure must be detected independently of the shutdown result.
- **Resource attributes are named differently per signal** in OpenObserve (`kiro_run_kind` in metrics and logs, `service_kiro_run_kind` in traces). Dashboard queries and the consistency test must use the per-signal names.
- **Each metric is its own stream.** The side-by-side view joins streams on `kiro_spec`; the catalogue test maps metric names to stream names by replacing dots with underscores.
- The blocking client works without starting an async runtime in this project's code, but tokio is still compiled in. Whether 127 crates behind a build feature is acceptable is the owner's decision in task 1.2.

## Owner decision and second T0 record (2026-11-10)

Decision: OTLP JSON files, shipped by a Collector, instead of the in-process exporter. The gate then needs no network code, no async runtime and cannot stall on a backend.

Verified the same day with hand-written OTLP JSON lines (one request object per line; files `metrics.jsonl`, `logs.jsonl`, `traces.jsonl`):

| Path | Observed |
|---|---|
| Direct HTTP post | `POST <base>/api/default/v1/metrics`, `/v1/logs`, `/v1/traces` with `Content-Type: application/json` and a Basic authorization header returned 200 for each line; all three were found by SQL search |
| Collector | `otel/opentelemetry-collector-contrib` 0.165.0, receiver `otlpjsonfile` with `include: ["/data/*.jsonl"]` and `start_at: beginning`, used by a metrics, a logs and a traces pipeline, exporter `otlphttp` to `<base>/api/default`; all three were found by SQL search |
| Naming | The Collector warns that `otlpjsonfile` and `otlphttp` are deprecated aliases of `otlp_json_file` and `otlp_http`; the example uses the new names |
| Mixed directory | One receiver reading all three files served the three pipelines; no error line appeared in the Collector log for the foreign-signal files |

Consequences recorded in the design: the mapping moves into the core as pure serde structures; the CLI appends lines and adds no dependency; `opentelemetry-proto` is a development dependency used only to decode written lines; credentials are the shipper's concern; a telemetry directory inside the project root is refused.

Not yet verified: decoding with `opentelemetry-proto` (task 1.2), whole-line appends under parallel runs on the three platforms, and the receiver's behaviour when a file is removed or rotated while it is read.

The exporter probe and both JSON probes live in `D:\BuildArtifacts\openidea-kiro\telemetry-t0`. The containers `kiro-t0-openobserve` and `kiro-t0-collector` are stopped, not removed.

## Task 1.2 record (2026-11-10)

`opentelemetry-proto` `=0.33.1` with features `gen-tonic-messages`, `with-serde`, `metrics`, `logs`, `trace` (default features off) is a development dependency of `kiro-cli`. `crates/kiro-cli/tests/e2e_telemetry.rs` decodes the three lines that OpenObserve ingested and rejects a wrong shape, a non-hex trace identifier and a truncated line: 4 tests pass. The sorted `cargo tree -p kiro-cli -e normal` output is byte-identical before and after; the `kiro-core` build for `wasm32-wasip2` succeeds; clippy with warnings denied, `cargo fmt --check` and `cargo deny check` pass. The development dependency adds no tokio. T0 is complete.

## Implementation record: T1 (2026-11-10)

- Snapshot model, provider contract, catalogue (15 metrics), spec and traceability metrics and the text and JSON renderers are in `crates/kiro-core/src/telemetry/`. `kiro-cli telemetry export --all|--feature <name> --dry-run` prints the snapshot of a project; without `--dry-run` it refuses, because file writing is task 3.3.
- `SpecMetrics` takes the loaded `Feature` plus the check report and findings of the run. The placeholder type names of the design were replaced by these existing types.
- Deviations from the design text, all recorded in the catalogue: `kiro.spec.approved` is sent as 1 or 0 per document so that an unapproved document is distinguishable from a missing spec; `kiro.spec.terms` also carries `mismatched` and `suggested`; checked criteria are the criteria that carry an annotation, the same number the `CHECK_COVERAGE` finding reports.
- A first version counted untasked criteria over major tasks only and reported 3 of 4 for the fixture. Cross-checking against `tasks.md` showed all four are referenced; the count now walks sub-tasks too and a test pins 0.
- On this repository the dry run prints 256 points for 13 specs.
- Verification on Windows: all workspace tests pass, clippy with warnings denied, format check, wasm build of the core, `cargo deny check`.

## Implementation record: T2 and task 5.1 (2026-11-10)

- `crates/kiro-core/src/telemetry/otlp.rs` maps snapshot, findings and run identity to three OTLP JSON lines. `kiro-cli check --telemetry-dir <DIR>` (or `KIRO_TELEMETRY_DIR`) and `telemetry export --telemetry-dir <DIR>` append them. The snapshot is built from the report the check already produced; nothing is checked twice.
- Real-binary journey (`crates/kiro-cli/tests/e2e_telemetry.rs`, 18 tests): every written line decodes with `opentelemetry-proto`; gauges equal the dry-run values; one log record per finding; root span plus one span per phase; report and exit code identical with telemetry on and off; unwritable directory keeps the gate result; a directory inside the project (direct, nested, via `..`, relative) is refused before the gate runs; invalid `KIRO_RUN_KIND` fails only when telemetry is on; a dry run ignores the environment directory; 8 parallel runs leave only whole lines (Windows); the record cap drops the least severe records and reports the count.
- End to end with this repository's data: `telemetry/otelcol.yaml` loaded in Collector contrib 0.165.0 without a deprecation warning and delivered the files to OpenObserve; a query returned the task totals of all 13 specs, 242 finding records and the four spans of the run. A second export was posted line by line to `/v1/metrics`, `/v1/logs` and `/v1/traces`; each returned 200 and the data was queryable.
- Deviations and gaps:
  - Criterion 4.2 names hint and owners on a finding record. Today's `Finding` has neither; the record carries code, severity, spec, file, line and message. Hint and owners arrive with the code findings of code-constraints.
  - The design placed the directory check and the line append in `telemetry_io.rs`. They are in `fs.rs`, because a test confines `std::fs` to that module. `telemetry_io.rs` holds environment, Git identity, timing and identifiers.
  - Git identity uses a read-only `git` subprocess; this is the first subprocess in `kiro-cli`.
  - The runtime dependency graph of `kiro-cli` gained only the workspace crate `kiro-cli-common`; no external package.
- Still unverified: Linux and macOS runs (task 6.1), Collector behaviour when a file is removed or rotated while read, and the dashboards (tasks 5.2 to 5.4).
- Verification on Windows: 434 workspace tests pass, clippy with warnings denied, format check, wasm build of the core, `cargo deny check`.
