$ErrorActionPreference = "Stop"

# Ensure wasm32-wasip2 target is installed
$targets = rustup target list --installed
if ($targets -notcontains "wasm32-wasip2") {
    Write-Host "Adding rustup target wasm32-wasip2..."
    rustup target add wasm32-wasip2
}

# Build release component
Write-Host "Building release component for wasm32-wasip2..."
cargo build --release --target wasm32-wasip2 --manifest-path (Join-Path $PSScriptRoot "Cargo.toml")

# Ensure dist directory exists
$distDir = Join-Path $PSScriptRoot "dist"
if (-not (Test-Path $distDir)) {
    New-Item -ItemType Directory -Path $distDir | Out-Null
}

# Copy wasm component to dist/plugin.wasm
$wasmSource = Join-Path $PSScriptRoot "target\wasm32-wasip2\release\openidea_plugin_kiro.wasm"
$wasmTarget = Join-Path $distDir "plugin.wasm"

Copy-Item -LiteralPath $wasmSource -Destination $wasmTarget -Force
$size = (Get-Item -LiteralPath $wasmTarget).Length
Write-Host ("Built dist/plugin.wasm ({0:N0} bytes)" -f $size)
