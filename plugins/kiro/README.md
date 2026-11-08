# OpenIDEA Kiro Plugin (`openidea-plugin-kiro`)

The `kiro` plugin integrates Kiro-style Spec-Driven Development (SDD) into the OpenIDEA editor as a sandboxed WebAssembly component (`wasm32-wasip2`). It bridges the pure `kiro-core` engine to OpenIDEA's extension host.

---

## 1. Building the Component

The WebAssembly component binary is built at `dist/plugin.wasm`. You can build it using any of three routes:

### Route A: PowerShell Script (Recommended on Windows)
From the package root (`plugins/kiro/`):
```powershell
.\build.ps1
```
This script ensures the `wasm32-wasip2` rustup target is installed, compiles the release component with `opt-level = "s"`, strips symbols, and copies the resulting binary to `dist/plugin.wasm`.

### Route B: Host Repository Build Script
From the OpenIDEA host checkout (`../openidea`):
```bash
./plugins/build.sh <absolute-path-to-plugins/kiro>
```

### Route C: Plain Cargo
From the package root (`plugins/kiro/`):
```bash
cargo build --release --target wasm32-wasip2
mkdir -p dist
cp target/wasm32-wasip2/release/openidea_plugin_kiro.wasm dist/plugin.wasm
```

---

## 2. Windows Dev-Plugin Installation

To install and test this plugin as a live developer plugin in OpenIDEA on Windows:

1. Create the plugin folder inside your project's `.openidea` configuration directory:
   ```powershell
   New-Item -ItemType Directory -Force -Path .openidea\plugins
   ```
2. Create an NTFS directory junction pointing to the plugin package directory:
   ```powershell
   cmd /c mklink /J .openidea\plugins\kiro <absolute-path-to-openidea-kiro>\plugins\kiro
   ```
3. Open or switch to the OpenIDEA IDE window.
4. Ensure the project is trusted (Plugins do not run in untrusted mode).
5. Open **Settings › Plugins** or execute the action **Reload Plugins** from Search Everywhere (`Ctrl+Shift+A`).

---

## 3. Which Project to Open

Always open the project whose `.kiro` tree you wish to inspect and work on:
- **This repository (`openidea-kiro`)**: Open this repository in OpenIDEA to navigate and inspect its own specs (`.kiro/specs/kiro-core`, `plugin-core`, etc.).
- **Host repository (`openidea`)**: Open the OpenIDEA host repository when testing host-side integration or when working on the host editor itself.

---

## 4. Installation from Disk & Rebuilding

- **Rebuilding & Live Reload**: When you modify Rust code in `plugins/kiro/src/` or `crates/kiro-core/`:
  1. Rebuild the component using `.\build.ps1`.
  2. In OpenIDEA, run the action **Reload Plugins** (`Ctrl+Shift+A` › "Reload Plugins").
  3. The IDE unloads the old component and reloads `dist/plugin.wasm` without restarting the IDE.
- **Disk Installation**: When packaging for distribution, zip the package containing `plugin.toml` and `dist/plugin.wasm`, then choose **Install Plugin from Disk...** in OpenIDEA's Plugin Settings.

---

## 5. Command Reference

The static commands declared in `plugin.toml` match the registry's static list:

| Command ID | Action Title | Spec-Addressable | Insert-Capable | Description |
| :--- | :--- | :---: | :---: | :--- |
| `ws.specs` | `Kiro: List Specs` | No | Yes | Lists all known specs, discovery sources, `spec.json` readability, phases, and per-spec command variants. |
| `ws.specs.insert` | `Kiro: List Specs — Insert at Cursor` | No | Variant | Full text of `ws.specs` inserted into the active editor buffer. |
| `ws.current` | `Kiro: Current Spec` | Yes | No | Reports the active spec name, resolution source, document presence, and findings. |

*Note*: For spec-addressable commands like `ws.current`, dynamic per-spec variants (`ws.current.<spec>`, e.g., `ws.current.plugin-core`) are registered automatically for each known spec in the active project.

---

## 6. Three-Tier Output Contract

The plugin provides three complementary presentation tiers:

1. **Notification Balloon (IDE Toast / Summary)**:
   - Displays immediate status and summary metrics.
   - Capped at **15 KiB** (below the host's 16 KiB limit).
   - When text exceeds 15 KiB, it is cleanly cut at a line boundary with an omitted-lines trailer:
     - For insert-capable commands: `… N more lines — run <id>.insert for the full text`
     - For other commands: `… N more lines`
2. **Editor Insertion (`.insert` variant or `insert` argument)**:
   - Inserts the complete, unclipped text payload at the cursor in the active editor buffer.
   - Capped at **1 MiB - 1 KiB** (respecting host memory buffers).
3. **CLI Durable Record (`kiro-cli` JSON)**:
   - Command-line invocations produce structured JSON or CLI tables for scripting, Git pre-commit hooks, and CI/CD pipelines.

---

## 7. Argument Grammar for Programmatic Callers

When triggering commands via Search Everywhere or programmatically:

- Arguments are tokenized by whitespace (`split_whitespace()`).
- The keyword `insert` in any token position sets the insertion flag and is stripped from arguments.
- For commands with `positional = Positional::Spec` (such as `ws.current`):
  - The first remaining token specifies the target spec name (e.g. `ws.current plugin-core`).
  - Spec names must match `[a-z0-9][a-z0-9._-]*` and contain no path separators (`/`, `\`) or `..`.
- If an explicit spec is supplied both in a variant ID (`ws.current.demo`) and in the argument (`old`), conflicting values are rejected.
- All other tokens pass through to the owning feature module.

---

## 8. Host Sandbox Limits & Budgets

The WebAssembly plugin runs in a sandboxed host environment under strict resource budgets:

- **Notification Budget**: 16 KiB hard ceiling; plugin clips at 15 KiB on line boundaries.
- **Title Budget**: 256 bytes hard ceiling; cut on UTF-8 character boundaries.
- **Editor Insertion Budget**: 1 MiB hard ceiling; plugin clips at 1,047,552 bytes on line boundaries.
- **Command Registration Limit**: 256 total registered commands per plugin instance (static + dynamic per-spec variants).
- **Execution Timeout**: Host cancels commands that take longer than 5 seconds (standard Kiro operations complete in < 100 ms).
