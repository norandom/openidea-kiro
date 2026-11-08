# kiro-cli

Native command-line interface and continuous-integration quality gate for Kiro / cc-sdd workspaces.

## Documentation

- **Command Reference**: See [`docs/cli.md`](../../docs/cli.md) for full usage, options, default values, examples, and exit codes.
- **CI/CD Integration**: See [`docs/ci.md`](../../docs/ci.md) for GitHub Actions jobs, generic shell scripts, exit code semantics, and JSON report queries.
- **Vocabulary Guide**: See [`docs/vocabulary.md`](../../docs/vocabulary.md) for vocabulary schema, EARS constraints, and Auto Ontology / SKOS import instructions.

## Quick Start

Run the quality gate across all specifications in the project:

```bash
cargo run -p kiro-cli -- check --all --fail-on error
```

Inspect specification lifecycle status:

```bash
cargo run -p kiro-cli -- status <feature-name>
```

Render requirement graph or task graph:

```bash
cargo run -p kiro-cli -- graph <feature-name>
cargo run -p kiro-cli -- tasks <feature-name> --table
```

Validate or import project vocabularies:

```bash
cargo run -p kiro-cli -- vocab validate
cargo run -p kiro-cli -- vocab import terms.json --format auto-ontology-json --topic sdd
```
