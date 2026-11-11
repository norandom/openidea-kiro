# Telemetry

Status: in progress (spec `quality-telemetry`). The snapshot, the catalogue, the file writer and the Collector example exist and are tested. Dashboards are not built yet.

In one paragraph: a gate run is reduced to metric points, written as OpenTelemetry protocol JSON lines into a directory outside the repository, and shipped by an OpenTelemetry Collector to OpenObserve. The gate itself opens no network connection, and telemetry never changes a verdict.

## Metric catalogue

Every metric the gate may emit is listed here. The table is generated from `crates/kiro-core/src/telemetry/catalogue.rs`; a test fails when the two differ.

| Metric | Unit | Attributes | Source | Meaning |
| --- | --- | --- | --- | --- |
| `kiro.code.breaches` | count | `kiro.spec`, `kiro.code`, `kiro.severity` | code | Code findings that are not pre-existing, by finding code and severity |
| `kiro.code.changed` | count | `kiro.metric`, `kiro.status` | code | Routine metrics that differ from the before side: improved or worsened |
| `kiro.code.files` | count | `kiro.status` | code | Selected files by state: measured, failed, unmeasured, ignored |
| `kiro.code.highest` | value | `kiro.spec`, `kiro.metric` | code | Highest value of a routine metric over the judged files of a spec |
| `kiro.code.pre_existing` | count | `kiro.spec` | code | Breaches that existed before and did not worsen |
| `kiro.code.routines` | count | `kiro.spec` | code | Routines measured in the judged files of a spec |
| `kiro.code.total` | value | `kiro.spec`, `kiro.metric` | code | Sum of a routine metric over the judged files of a spec |
| `kiro.run.duration` | milliseconds | `kiro.phase` | run | Duration of one phase of a gate run |
| `kiro.run.findings.dropped` | count | none | run | Finding records left out because the per-run maximum was reached |
| `kiro.spec.approved` | value | `kiro.spec`, `kiro.document` | status | 1 when the document of a spec is approved, otherwise 0 |
| `kiro.spec.contradiction.pairs` | count | `kiro.spec`, `kiro.status` | check | Criterion pairs by contradiction search outcome: considered, found, refused |
| `kiro.spec.criteria.checked` | count | `kiro.spec` | check | Acceptance criteria the constraint checks covered |
| `kiro.spec.criteria.total` | count | `kiro.spec` | check | Acceptance criteria of a spec |
| `kiro.spec.findings` | count | `kiro.spec`, `kiro.code`, `kiro.severity` | check | Findings by finding code and severity |
| `kiro.spec.phase` | value | `kiro.spec`, `kiro.phase` | status | 1 for the phase a spec is in |
| `kiro.spec.tasks.done` | count | `kiro.spec` | status | Completed tasks of a spec |
| `kiro.spec.tasks.total` | count | `kiro.spec` | status | Tasks of a spec |
| `kiro.spec.terms` | count | `kiro.spec`, `kiro.status` | check | Terms by state: declared, resolved, deprecated, mismatched, undeclared, ambiguous, suggested |
| `kiro.trace.components.unmapped` | count | `kiro.spec` | join | Design components with no file |
| `kiro.trace.criteria.untasked` | count | `kiro.spec` | join | Acceptance criteria referenced by no task |
| `kiro.trace.files.multi_owned` | count | `kiro.spec` | join | Files mapped by more than one spec |
| `kiro.trace.tasks.unknown_boundary` | count | `kiro.spec` | join | Tasks naming a boundary that no design component matches |

A metric is omitted when its source report did not run. It is never sent as zero.

Phase and approval are sent as value 1 with the state in an attribute, so that a dashboard can count specs by state. Ratios are not sent: a dashboard derives the share of checked criteria from `kiro.spec.criteria.checked` and `kiro.spec.criteria.total`, so that sums across specs stay correct.

### Attribute keys

`kiro.spec`, `kiro.component`, `kiro.task`, `kiro.code`, `kiro.severity`, `kiro.metric`, `kiro.language`, `kiro.phase`, `kiro.document`, `kiro.status`. Routine names, file paths and message texts are never metric attributes; they appear only in finding records.

### Reserved prefixes

Each analyzer spec owns a prefix: `kiro.code.` (code-constraints, defined above), and `kiro.clone.` (clone-constraints), `kiro.locality.` (code-locality-data) and `kiro.evidence.` (verification-evidence), which are reserved until those specs ship. `kiro-cli code check --telemetry-dir <DIR>` writes the `kiro.code.*` points of the run; its finding records also carry `kiro.tag`, `kiro.hint`, `kiro.metric`, `kiro.component`, `kiro.task` and `kiro.pre_existing`. Files no spec maps carry the spec `(none)`.

## Names in OpenObserve

Observed with the pinned backend in the prototype (see the spec's research log):

- Each metric becomes its own metrics stream, with dots replaced by underscores: `kiro.spec.findings` is the stream `kiro_spec_findings`. The value is the field `value`.
- Attribute keys become fields the same way: `kiro.spec` is `kiro_spec`.
- Resource attributes are plain fields in metrics and logs (`kiro_run_kind`) and carry a `service_` prefix in traces (`service_kiro_run_kind`).

## Writing telemetry

Telemetry is off unless a directory is named:

```bash
kiro-cli check --all --telemetry-dir /var/tmp/kiro-telemetry
# or
KIRO_TELEMETRY_DIR=/var/tmp/kiro-telemetry kiro-cli check --all
```

Each run appends one line to each of three files in that directory:

| File | Content |
| --- | --- |
| `metrics.jsonl` | One gauge data point per metric point of the catalogue, plus the phase durations |
| `logs.jsonl` | One log record per finding, errors first, at most 2000 per run; the number left out is the metric `kiro.run.findings.dropped` |
| `traces.jsonl` | One trace per run: a root span `kiro.gate` with the verdict and finding counts, and one child span per phase |

Each line is a complete OpenTelemetry protocol request in the JSON encoding. Rules:

- The directory must lie outside the project. A directory inside it is refused with exit code 2 before the gate runs, so that no metric file enters the repository.
- The report and the exit code are the same with and without telemetry. If a file cannot be written, `check` prints one line on standard error and keeps its result.
- The gate never removes or rotates the files. They grow until the shipper removes them.
- `kiro-cli telemetry export --all --telemetry-dir <DIR>` writes the same three lines without printing a report; there a failed write is a failure (exit code 2). With `--dry-run` it prints the snapshot and writes nothing.

### Identity of a run

Every line carries these resource attributes:

| Attribute | Source |
| --- | --- |
| `service.name` | `OTEL_SERVICE_NAME`, default `kiro-cli` |
| `service.version` | the `kiro-cli` version |
| `vcs.repository.url.full`, `vcs.ref.head.name`, `vcs.ref.head.revision` | Git, when the project is a repository; credentials are removed from the address |
| run kind, `kiro.run.kind` | `KIRO_RUN_KIND`: `hook`, `agent`, `ci` or `manual` (default); another value is a failure |
| further pairs | `OTEL_RESOURCE_ATTRIBUTES` (`key=value,key=value`); they cannot replace the attributes above |

No other environment value is read. No absolute path, user name or source text is written: file paths in finding records are project-relative.

## Shipping to OpenObserve

Two ways were verified against OpenObserve with files written by `kiro-cli` on this repository.

**OpenTelemetry Collector.** [`telemetry/otelcol.yaml`](../telemetry/otelcol.yaml) is a working example for the contrib distribution (0.165.0): it reads `*.jsonl` in the telemetry directory and forwards metrics, logs and traces. The Collector's environment supplies `KIRO_TELEMETRY_DIR`, `OPENOBSERVE_ENDPOINT` (for example `http://localhost:5080/api/default`) and `OPENOBSERVE_AUTH` (`Basic <base64 of user:password>`). Credentials never reach `kiro-cli`.

**Direct post.** Each line can be posted unchanged with content type `application/json` to `<endpoint>/v1/metrics`, `<endpoint>/v1/logs` and `<endpoint>/v1/traces`.

Not yet verified: how the Collector's file receiver behaves when a file is removed or rotated while it is being read.
