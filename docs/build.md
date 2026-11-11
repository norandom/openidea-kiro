# Build guide

For use, install a prebuilt binary from a [release](../README.md#install-from-a-release). Build from source when you change the code, need a platform without a release asset, or want the plugin from the current tree.

## Prerequisites

- **Rust toolchain**: stable, 1.85 or newer (`rustup default stable`).
- **WebAssembly target**, for the engine check and the editor plugin: `rustup target add wasm32-wasip2`.
- **A C compiler**, for the tree-sitter grammars that `kiro-code` links.
- **cargo-deny**, for the dependency check of the gate: `cargo install cargo-deny --locked`.
- **OpenIDEA**, only to run the plugin.

## Workspace

| Crate | Role | Targets |
| :--- | :--- | :--- |
| [`crates/kiro-core`](../crates/kiro-core/) | The engine: parsers for the spec documents, EARS rules, vocabulary, contradiction search, traceability, code limits, ratchet and rule judgement. It reads no file. | native, `wasm32-wasip2` |
| [`crates/kiro-code`](../crates/kiro-code/) | Reads source files, parses them with tree-sitter grammars through `ast-grep`, and measures them. | native |
| [`crates/kiro-cli-common`](../crates/kiro-cli-common/) | Shared command-line options: report format, destination, gate threshold. | native |
| [`crates/kiro-cli`](../crates/kiro-cli/) | The command-line gate. | native |
| [`plugins/kiro`](../plugins/kiro/) | The OpenIDEA plugin, a WebAssembly component with its own lock file. | `wasm32-wasip2` |

## Build and install the CLI

```bash
cargo install --path crates/kiro-cli
kiro-cli --version
```

Or build without installing; the binary is `target/release/kiro-cli` (`kiro-cli.exe` on Windows), or under `CARGO_TARGET_DIR` when that is set:

```bash
cargo build --release -p kiro-cli
```

## Build the plugin

Windows (PowerShell):

```powershell
cd plugins\kiro
.\build.ps1
```

Linux and macOS:

```bash
cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2
mkdir -p plugins/kiro/dist
cp plugins/kiro/target/wasm32-wasip2/release/openidea_plugin_kiro.wasm plugins/kiro/dist/plugin.wasm
```

### Install the built plugin

Into one project, by copying it to the project's plugin directory:

```powershell
New-Item -ItemType Directory -Force -Path .openidea\plugins\kiro
Copy-Item -Recurse -Force plugins\kiro\* .openidea\plugins\kiro\
```

```bash
mkdir -p .openidea/plugins/kiro
cp -R plugins/kiro/* .openidea/plugins/kiro/
```

Or from the editor: *Settings › Plugins*, *Install Plugin from Disk...*, select `plugins/kiro` (or a zip with `plugin.toml` and `dist/plugin.wasm`), trust the project when asked, then run *Reload Plugins*.

## The quality gate

CI runs these steps in a container through Dagger ([CI integration](ci.md)). Run them locally before a push:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo build -p kiro-core --target wasm32-wasip2
cargo build --manifest-path plugins/kiro/Cargo.toml --release --target wasm32-wasip2

cargo kiro-check                                              # kiro-cli check --all --fail-on error on this repository
cargo run -p kiro-cli -- code check --changed origin/main     # the code gate on what the branch changed
```

`cargo kiro-check` is an alias in `.cargo/config.toml`. The code gate holds this repository to its own limits: a routine above a limit in Rust or Python fails the build unless it was already above it on `main`.

## Releases

A tag that matches `v*` starts the release workflow. It builds `kiro-cli` for Linux x86_64, macOS on Apple Silicon and Intel, and Windows x86_64, builds the plugin, writes `SHA256SUMS.txt`, and publishes a GitHub release with generated notes. A tag with `-alpha`, `-beta`, `-rc` or `-pre` is published as a pre-release.

```bash
git tag -a v1.0.0 -m "OpenIDEA Kiro 1.0.0"
git push origin v1.0.0
```

Before tagging, set the same version in the four crate manifests, `plugins/kiro/Cargo.toml`, `plugins/kiro/plugin.toml` and `CITATION.cff`. Zenodo archives each published release and assigns a new version DOI; the DOI for all versions, 10.5281/zenodo.24128225, stays the same. After a release, set the new version DOI in `CITATION.cff` (`doi` and the first identifier) and in the BibTeX entry of the README.
