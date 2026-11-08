# OpenIDEA Kiro (`openidea-kiro`)

[![CI](https://github.com/norandom/openidea-kiro/actions/workflows/ci.yml/badge.svg)](https://github.com/norandom/openidea-kiro/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

**Kiro** is a Spec-Driven Development (SDD) tooling ecosystem and OpenIDEA IDE plugin. It brings formal requirement constraints, domain vocabulary grounding, structural specification visualization, bidirectional code ↔ spec navigation, and automated CI quality gates to modern software development.

---

## Architecture Overview

```
                                      ┌────────────────────────────────────────┐
                                      │              kiro-core                 │
                                      │ Pure domain model, parsers, EARS rules │
                                      │    BDD contradiction search, graph     │
                                      │        Target: native & WASI           │
                                      └──────────────────┬─────────────────────┘
                                                         │
                                ┌────────────────────────┴────────────────────────┐
                                │                                                 │
                                ▼                                                 ▼
             ┌─────────────────────────────────────┐           ┌─────────────────────────────────────┐
             │              kiro-cli               │           │         openidea-plugin-kiro        │
             │   Native CI Quality Gate executable │           │  WebAssembly IDE component (WASIp2) │
             │  • check (EARS, terms, BDD logic)   │           │  • 25 static palette commands       │
             │  • status, graph, tasks views       │           │  • Spec ↔ code navigation           │
             │  • vocab import (AutoOntology/SKOS) │           │  • Live editor buffer insertion     │
             └─────────────────────────────────────┘           └─────────────────────────────────────┘
```

The repository is organized into three primary artifacts:
- **[`kiro-core`](crates/kiro-core/)**: Pure, zero-IO Rust engine for parsing specification markdown, validating EARS criteria, grounding terms in domain vocabularies, and checking for logic contradictions using BDDs.
- **[`kiro-cli`](crates/kiro-cli/)**: Standalone, native command-line quality gate designed for pre-commit hooks, local developer workflows, and CI/CD pipelines.
- **[`plugins/kiro`](plugins/kiro/)**: High-performance WebAssembly (`wasm32-wasip2`) component that executes inside OpenIDEA's sandboxed plugin runtime.

---

## Prerequisites

- **Rust toolchain**: Stable 1.85+ (`rustup default stable`)
- **WebAssembly target** (for IDE plugin):
  ```bash
  rustup target add wasm32-wasip2
  ```
- **OpenIDEA editor**: (Optional, for running the IDE plugin)

---

## Installation

### 1. Install the CLI (`kiro-cli`)

Install `kiro-cli` directly into your Cargo binary path:

```bash
cargo install --path crates/kiro-cli
```

Verify the installation:

```bash
kiro-cli --help
```

### 2. Install the OpenIDEA IDE Plugin

#### Step A: Build the WebAssembly Component

From the repository root:

- **Windows (PowerShell)**:
  ```powershell
  cd plugins\kiro
  .\build.ps1
  ```
- **macOS / Linux**:
  ```bash
  cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2
  mkdir -p plugins/kiro/dist
  cp plugins/kiro/target/wasm32-wasip2/release/openidea_plugin_kiro.wasm plugins/kiro/dist/plugin.wasm
  ```

#### Step B: Install in OpenIDEA

##### Option 1: PowerShell Workspace Install (Windows)
Copy the plugin folder into your project's `.openidea\plugins` directory using standard PowerShell:

```powershell
# Create plugin directory and copy files
New-Item -ItemType Directory -Force -Path .openidea\plugins\kiro
Copy-Item -Recurse -Force plugins\kiro\* .openidea\plugins\kiro\
```

For macOS / Linux:
```bash
mkdir -p .openidea/plugins/kiro
cp -R plugins/kiro/* .openidea/plugins/kiro/
```

##### Option 2: GUI Install from Disk
In OpenIDEA:
1. Open **Settings › Plugins** (or press `Ctrl+Alt+S` on Windows / `⌘,` on macOS).
2. Click the gear icon / menu and select **Install Plugin from Disk...**.
3. Select the `plugins/kiro` directory (or a zip containing `plugin.toml` and `dist/plugin.wasm`).
4. Ensure the project is trusted when prompted.
5. In Search Everywhere (`Ctrl+Shift+A` or `Double Shift`), run **Reload Plugins**.

---

## How to Use `kiro-cli`

### 1. Run Specification Quality Checks

Validate EARS syntax, vocabulary adherence, and logical contradictions across all specifications in your project:

```bash
kiro-cli check --all
```

To fail your CI pipeline on any error:

```bash
kiro-cli check --all --fail-on error
```

Check a specific specification feature:

```bash
kiro-cli check --feature cli-gate
```

Output machine-readable JSON for automated scripts or `jq`:

```bash
kiro-cli check --all --format json
```

### 2. Inspect Specification Status & Graphs

View the current lifecycle state, approvals, and metrics of a spec:

```bash
kiro-cli status cli-gate
```

Render requirements dependency graphs or task completion flows as Mermaid diagrams or Markdown tables:

```bash
# Mermaid diagram
kiro-cli graph cli-gate

# Markdown table
kiro-cli graph cli-gate --table

# Task dependency table
kiro-cli tasks cli-gate --table
```

### 3. Manage Domain Vocabularies

Validate `.kiro/vocabulary.toml`:

```bash
kiro-cli vocab validate
```

Import external taxonomies directly from Auto Ontology JSON exports or SKOS RDF Turtle files:

```bash
# Import from Auto Ontology JSON
kiro-cli vocab import terms.json --format auto-ontology-json --topic sdd --merge

# Import from W3C SKOS RDF Turtle
kiro-cli vocab import ontology.ttl --format skos-turtle --topic ops
```

### 4. Use the Cargo Gate Alias

This repository includes a configured Cargo shortcut:

```bash
cargo kiro-check
```

---

## How to Use the OpenIDEA Plugin

Once loaded in OpenIDEA, Kiro exposes actions via the command palette (**`Ctrl+Shift+A`** on Windows/Linux or **`Cmd+Shift+A`** on macOS).

### Key Commands

| Action Title | Command ID | What It Does |
| :--- | :--- | :--- |
| **`Kiro: Spec Context`** | `nav.context` | Identifies which spec, component, and tasks own the active file. |
| **`Kiro: Check Spec`** | `check.spec` | Runs EARS, vocabulary, and contradiction checks on the active spec. |
| **`Kiro: Spec Status`** | `viz.status` | Shows lifecycle phase, approvals, and task progress. |
| **`Kiro: Spec Graph`** | `viz.graph` | Displays the requirement-to-file traceability graph. |
| **`Kiro: Task Graph`** | `viz.tasks` | Visualizes task dependencies and execution readiness. |
| **`Kiro: List Vocabulary Terms`** | `vocab.list` | Browses all domain terms in scope for the active spec. |
| **`Kiro: Vocabulary Term`** | `vocab.term` | Looks up definitions, kinds, parent terms, and replacements. |

> **Tip**: All visualization and check commands support editor insertion variants (e.g., `viz.graph.insert`, `check.spec.insert`) to embed formatted diagrams and reports directly into your markdown notes.

---

## Spec-Driven Development (SDD) Workflow

Projects using Kiro store specifications under `.kiro/`:

```
.kiro/
├── steering/                   # Project policies, roadmap, architecture
│   ├── roadmap.md
│   └── tech.md
├── vocabulary.toml             # Domain vocabulary definitions & topics
└── specs/                      # Feature specifications
    └── my-feature/
        ├── spec.json           # Lifecycle phase, approvals, domain topics
        ├── requirements.md     # EARS-compliant requirements & acceptance criteria
        ├── design.md           # Architecture, component interfaces, file structure
        └── tasks.md            # Work breakdown structure and dependencies
```

### 3-Phase Approval Lifecycle
1. **Requirements**: EARS ubiquitous, event-driven, or state-driven criteria.
2. **Design**: Architecture boundaries, component interfaces, and File Structure Plans.
3. **Tasks**: Fine-grained implementation items mapped to design components.
4. **Implementation**: Code changes verified against the specification gate.

---

## Formal Verification Strategy: Ontologies & BDDs

Traditional software specifications force teams to choose between **accessible natural language** (which silently accumulates synonym drift and logical contradictions) and **classical formal methods** (such as TLA+ or SMT solvers like Z3/CVC5, which require heavyweight C++ runtimes and suffer from undecidable timeouts).

Kiro provides mathematical rigor directly within your Markdown files by combining **Domain Ontologies** with **Reduced Ordered Binary Decision Diagrams (ROBDDs)**:

```
Natural Language Criteria ──▶ Ontology Grounding ──▶ Region Finitization ──▶ Canonical BDD
  "depth > 1000 and safe"    (enum / int facets)     (one-hot variables)     O(1) Sat Check
```

### Why BDDs Instead of SMT Solvers?

| Criterion | SMT Solvers (Z3, CVC5) | Kiro BDD Engine (`biodivine-lib-bdd`) |
| :--- | :--- | :--- |
| **Canonicity** | Non-canonical (formulas have no unique representation). | **Strictly canonical** (identical boolean functions share the exact same node). |
| **Satisfiability Check** | NP-complete to Undecidable; requires DPLL(T) solver loop. | **$O(1)$ constant time** (pointer comparison against the False sink $\bot$). |
| **Counterexample Witness** | Heuristic existential search over uninterpreted models. | **$O(\|V\|)$ linear time** path traversal from root to the True sink $\top$. |
| **Runtime & Dependencies** | Massive C++ dependencies (>500k LOC, >25 MB binary). | **Pure, safe Rust** with zero native dependencies. |
| **WebAssembly Sandbox** | Incompatible or bloated for WASI environments. | Runs seamlessly inside OpenIDEA's `wasm32-wasip2` sandbox in **< 5 ms**. |
| **Resource Safety** | Solvers may time out or hang unpredictably. | Enforces strict bounds (`max_variables = 512`, `max_nodes = 100,000`). |

### How Vocabulary Grounding Makes BDDs Possible

BDDs operate strictly on finite boolean variables, whereas real software specifications express business rules over domain entities, enum states, and numerical thresholds. 

Kiro bridges this gap through two techniques:
1. **Ontology Topic Scoping (`.kiro/vocabulary.toml`)**: Terms are declared with explicit kinds (`entity`, `bool`, `enum`, `int`) and scoped by topic in `spec.json`. This prevents semantic collision across independent features.
2. **Dynamic Region Finitization (`finitize.rs`)**: Rather than bit-blasting integers, Kiro collects all cut-points across the criteria and partitions the integer domain into contiguous one-hot regions (e.g., `depth < 500`, `depth = 500`, `depth in 501..1000`, `depth > 1000`).

When criteria prescribe conflicting effects (e.g., Criterion 3.1 commands `armed = true` while Criterion 3.2 commands `armed = false`), the BDD engine calculates their joint guard conjunction. If satisfiable, it emits a `CONTRADICTION` error with an **exact counterexample witness state**:
```text
.kiro/specs/safety-brake/requirements.md:14: ERROR CONTRADICTION criteria 3.1 and 3.2 can both apply (effect armed: true vs false); witness: depth >= 1001, mode = safe
```

If problem size exceeds configured limits, Kiro safely degrades by emitting `SEARCH_REFUSED` rather than hanging or guessing.

> 📖 **Deep Dive**: For full proofs, mathematical formulas, and algorithmic workflows, read the complete guide: **[Formal Verification Strategy: Domain Ontologies, Finitization, and BDDs](docs/verification.md)**.

---

## Documentation Index

- **[CLI Command Reference](docs/cli.md)**: Detailed syntax, options, and exit codes for `kiro-cli`.
- **[CI/CD Integration Guide](docs/ci.md)**: GitHub Actions workflows, shell scripts, and JSON reporting.
- **[Formal Verification Strategy](docs/verification.md)**: Mathematical foundations, SMT vs. BDD trade-offs, domain ontologies, and finitization algorithms.
- **[Vocabulary Guide](docs/vocabulary.md)**: Schema rules, topic scoping, and Auto Ontology / SKOS import pipelines.
- **[Constraint Checks Reference](docs/checks.md)**: EARS pattern rules, term resolution, and BDD contradiction search.

---

## Contributing & Development

Run the full quality gate locally:

```bash
# Code formatting check
cargo fmt --check

# Workspace clippy with warnings denied
cargo clippy --workspace --all-targets -- -D warnings

# Run all native unit, integration, and doc tests
cargo test --workspace

# Build WebAssembly components
cargo build -p kiro-core --target wasm32-wasip2
cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2

# Run self-tree specification check
cargo kiro-check
```

---

## License

This project is licensed under the [MIT License](LICENSE).
