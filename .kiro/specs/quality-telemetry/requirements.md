# Requirements Document

## Introduction

Quality Telemetry. Owns the metric snapshot built from the gate's own reports, its export as OpenTelemetry signal files, and the dashboard definitions that show specification health and code health side by side. It owns no rule, no measurement and no storage: values come from reports that other specs produce, and the store is an external OpenTelemetry backend, with OpenObserve as the reference.

## Scope and Adjacent Expectations

Consumes: cli-contracts. Requested by the owner on 2026-11-10 to replace a metric log file in the repository. The first deliverable uses only what the spec gate reports today. code-constraints, clone-constraints, code-locality-data and verification-evidence contribute points through this spec's contract when they ship; this spec does not wait for them and does not depend on them. Running or hosting a backend, alerting, and telemetry from the IDE plugin are out of scope, the last because the plugin sandbox has no network.

## Requirements

### Requirement 1: Metric Snapshot
**Objective:** As a maintainer, I want every gate run reduced to one list of named values, so that every view reads the same numbers the reports show.

#### Acceptance Criteria
1. The Telemetry Engine shall build one snapshot of metric points from the reports of a gate run, each point carrying a name, a unit, a value and attributes, without reading a clock, the network or the filesystem.
2. The Telemetry Engine shall name every metric under the `kiro.` prefix from one catalogue that states unit, attributes, source report and meaning, with a test that the catalogue and its documentation are equal.
3. Where the source report names a spec, a design component or a task, the Telemetry Engine shall attach them as attributes of the point.
4. The Telemetry Engine shall keep routine names, file paths and message texts out of metric attributes, carrying such detail only in finding records.
5. If a source report is absent from a run, the Telemetry Engine shall omit its points and name the source in the snapshot's coverage, never reporting an absent value as zero.
6. The Telemetry Engine shall accept points from other specifications through one provider contract, so that a new report adds metrics without a change to the exporter.

### Requirement 2: Specification and Traceability Metrics
**Objective:** As an architect, I want the state of the specifications as numbers per spec, so that drift in the requirements is visible before it reaches the code.

#### Acceptance Criteria
1. The Telemetry Engine shall report per spec the phase, the approval state of each document and the completed and total task counts.
2. The Telemetry Engine shall report per spec the number of findings by finding code and severity.
3. The Telemetry Engine shall report per spec the declared, resolved, undeclared, ambiguous and deprecated term counts and the considered, found and refused contradiction pairs.
4. The Telemetry Engine shall report per spec the share of acceptance criteria that the constraint checks actually covered.
5. The Telemetry Engine shall report per spec the criteria referenced by no task, the tasks naming an unknown boundary, the components with no file and the files mapped by more than one spec.

### Requirement 3: Export
**Objective:** As an operator, I want the snapshot written in the standard OpenTelemetry file format outside the repository, so that the collector I already run ships it and the gate itself needs no network.

#### Acceptance Criteria
1. The kiro-cli shall offer `telemetry export` for an explicit export and an automatic export at the end of a gate command where `--telemetry-dir` or `KIRO_TELEMETRY_DIR` names an output directory.
2. The kiro-cli shall write signals as OpenTelemetry protocol JSON, one request object per line, appended to `metrics.jsonl`, `logs.jsonl` and `traces.jsonl` in that directory, so that an OpenTelemetry Collector or an HTTP post ships them unchanged.
3. The kiro-cli shall open no network connection for telemetry.
4. If a telemetry file cannot be written, the kiro-cli shall report the failure on standard error and keep the verdict and exit code of the gate command.
5. If the telemetry directory lies inside the project root, the kiro-cli shall refuse it with a usage error before the gate runs, so that no metric file enters the repository.
6. When `telemetry export --dry-run` is given, the kiro-cli shall print the snapshot to standard output in the selected report format and write nothing.
7. The kiro-cli shall attach to every signal the service name and version, the repository, branch and revision when Git supplies them, a run kind of `hook`, `agent`, `ci` or `manual` taken from `KIRO_RUN_KIND`, and the attributes of `OTEL_RESOURCE_ATTRIBUTES`.
8. The kiro-cli shall write no absolute path, user name, source text, credential or other environment value into any signal.

### Requirement 4: Signals
**Objective:** As a dashboard author, I want aggregates, detail and timing in the signal that suits each, so that trends stay cheap and detail stays available.

#### Acceptance Criteria
1. The kiro-cli shall write every metric point as a gauge data point with its attributes.
2. The kiro-cli shall write every finding as one log record carrying code, severity, spec, file, line, message, hint and owners, up to a configured maximum per run, reporting the number of records dropped beyond it.
3. The kiro-cli shall write one trace per gate run with one span per phase and the verdict and finding counts on the root span, so that the duration of a run and of its phases is recorded.
4. The kiro-cli shall produce byte-identical text and JSON reports for the same input whether telemetry is enabled or not.

### Requirement 5: Dashboards
**Objective:** As a developer, an architect or a maintainer, I want a view made for my question, so that I do not assemble it from raw streams.

#### Acceptance Criteria
1. The repository shall provide OpenObserve dashboard definitions for a developer view, an architect view, a code-health view and a side-by-side view of specification health and code health per spec.
2. The repository shall provide a test that every metric name and attribute used in a dashboard definition exists in the catalogue.
3. While a source contributes no points, the dashboards shall show its panels as without data and never as zero.
4. The repository shall document the backend setup, the Collector and direct-post shipping paths, file growth and removal after shipping, the environment variables, the dashboard import and the rule that trend panels read `ci` runs of the default branch only.

### Requirement 6: Prototype, Packaging and Acceptance
**Objective:** As the owner, I want the written files proved against a real backend and the gate kept free of telemetry dependencies before the feature grows.

#### Acceptance Criteria
1. The development workflow shall verify that the written files are ingested by a pinned OpenObserve release through a pinned OpenTelemetry Collector and through a direct HTTP post, recording endpoint paths, authentication and the stream and field names the backend creates.
2. The workspace shall add no network, asynchronous-runtime or protocol dependency to the kiro-cli for telemetry, leaving the engine sandbox-target build and its dependency graph unchanged.
3. The acceptance suite shall drive the real binary, decode every written line with the official OpenTelemetry protocol definitions, assert the points, records and spans, and assert an unchanged exit code for an unwritable directory.
4. The acceptance suite shall run on Windows, Linux and macOS.
5. The first deliverable shall export the specification and traceability metrics of this repository from the existing check command before any code analyzer exists.
6. The repository shall provide an OpenTelemetry Collector configuration example that reads the telemetry directory and forwards to OpenObserve with credentials taken from the Collector environment.
