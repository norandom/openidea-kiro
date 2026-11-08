# CI/CD Integration and Quality Gate Guide

This guide describes how to integrate `kiro-cli` into continuous integration (CI) workflows as an automated quality gate for spec-driven development (cc-sdd) projects.

---

## 1. Overview

`kiro-cli` acts as the native validation gate for specifications, constraint consistency (EARS criteria, vocabulary grounding, and contradiction analysis), and task tracking. In CI environments, `kiro-cli check` returns deterministic exit codes to enforce merge gating and quality thresholds.

### Exit Codes

| Exit Code | Name | Meaning |
|-----------|------|---------|
| `0` | Clean | Validation succeeded; no findings met or exceeded the `--fail-on` threshold. |
| `1` | Findings | Validation failed; findings were detected that meet or exceed `--fail-on` (default: `error`). |
| `2` | Failure | Process failure; invalid arguments, unresolvable spec selection, missing project root, I/O errors, or unparseable input. |

---

## 2. GitHub Actions Job

The following GitHub Actions workflow job executes the full validation gate:

```yaml
name: CI Quality Gate

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  gate:
    name: Specification & Build Gate
    runs-on: ubuntu-latest
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-wasip2
          components: clippy, rustfmt

      - name: Rust Cache
        uses: Swatinem/rust-cache@v2

      - name: Install cargo-nextest and cargo-deny
        run: |
          cargo install cargo-nextest --locked
          cargo install cargo-deny --locked

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Workspace Clippy
        run: cargo clippy --workspace --all-targets -- -D warnings

      - name: Run native tests
        run: |
          cargo nextest run --workspace
          cargo test --workspace --doc

      - name: Dependency and License Check
        run: cargo deny check

      - name: Build and test plugin component
        run: |
          cargo clippy --manifest-path plugins/kiro/Cargo.toml --all-targets -- -D warnings
          cargo test --manifest-path plugins/kiro/Cargo.toml
          cargo deny --manifest-path plugins/kiro/Cargo.toml check
          cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2

      - name: Build and Run kiro-cli Quality Gate
        run: cargo kiro-check
```

---

## 3. Generic Shell Snippet

For non-GitHub CI environments (GitLab CI, Jenkins, Cirrus, local git hooks), use the following generic script:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "=== Installing toolchain targets & tools ==="
rustup target add wasm32-wasip2
which cargo-nextest >/dev/null 2>&1 || cargo install cargo-nextest --locked
which cargo-deny >/dev/null 2>&1 || cargo install cargo-deny --locked

echo "=== Code style & lints ==="
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

echo "=== Running tests ==="
cargo nextest run --workspace
cargo test --workspace --doc

echo "=== Cargo Deny security audit ==="
cargo deny check

echo "=== Building and testing plugin component ==="
cargo clippy --manifest-path plugins/kiro/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path plugins/kiro/Cargo.toml
cargo deny --manifest-path plugins/kiro/Cargo.toml check
cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2

echo "=== Running kiro-cli Gate ==="
cargo kiro-check
```

---

## 4. Machine-Readable JSON Output

In automated workflows, invoke `kiro-cli check` with `--format json` to pipe structured reports to logging systems, metrics collectors, or `jq`.

### Example JSON Report

```json
{
  "schema_version": "1",
  "specs": [
    "cli-gate"
  ],
  "checked": [
    {
      "spec": "cli-gate",
      "topics": [],
      "vocabulary": "loaded",
      "checks": {
        "ears": {
          "status": "ran",
          "findings": 0
        },
        "terms": {
          "status": "skipped",
          "reason": "spec declares no topics"
        },
        "contradictions": {
          "status": "skipped",
          "reason": "spec declares no topics"
        }
      }
    }
  ],
  "summary": {
    "errors": 0,
    "warnings": 0,
    "info": 0
  },
  "fail_on": "error",
  "verdict": "pass",
  "findings": []
}
```

### Useful `jq` Queries

Extract summary and verdict:
```bash
kiro-cli check --all --format json | jq '{verdict: .verdict, summary: .summary}'
```

List all finding codes and messages:
```bash
kiro-cli check --all --format json | jq -r '.findings[] | "\(.file):\(.line // 0): [\(.severity)] \(.code) - \(.message)"'
```

Count errors per spec:
```bash
kiro-cli check --all --format json | jq '[.findings[] | select(.severity == "error")] | group_by(.spec) | map({spec: .[0].spec, count: length})'
```
