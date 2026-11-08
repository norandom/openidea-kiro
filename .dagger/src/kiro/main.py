import dagger
from dagger import dag, function, object_type


@object_type
class Kiro:

  @function
  async def check(self, source: dagger.Directory) -> str:
    """Runs the full SDLC quality gate in an isolated, reproducible Rust container."""
    cargo_cache = dag.cache_volume("cargo-cache")
    target_cache = dag.cache_volume("target-cache")

    container = (
        dag.container()
        .from_("rust:latest")
        .with_exec(["rustup", "target", "add", "wasm32-wasip2"])
        .with_exec(["rustup", "component", "add", "clippy", "rustfmt"])
        .with_exec([
            "sh",
            "-c",
            (
                "curl -sL"
                " https://github.com/EmbarkStudios/cargo-deny/releases/download/0.18.3/cargo-deny-0.18.3-x86_64-unknown-linux-musl.tar.gz"
                " | tar -xz -C /usr/local/cargo/bin --strip-components=1"
            ),
        ])
        .with_mounted_cache("/usr/local/cargo/registry", cargo_cache)
        .with_mounted_cache("/workspace/target", target_cache)
        .with_directory("/workspace", source)
        .with_workdir("/workspace")
        .with_exec(["cargo", "fmt", "--all", "--", "--check"])
        .with_exec([
            "cargo",
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .with_exec(["cargo", "test", "--workspace"])
        .with_exec(["cargo", "test", "--workspace", "--doc"])
        .with_exec(["cargo", "deny", "check"])
        .with_exec(
            ["cargo", "build", "-p", "kiro-core", "--target", "wasm32-wasip2"]
        )
        .with_exec([
            "cargo",
            "clippy",
            "--manifest-path",
            "plugins/kiro/Cargo.toml",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .with_exec(
            ["cargo", "test", "--manifest-path", "plugins/kiro/Cargo.toml"]
        )
        .with_exec([
            "cargo",
            "deny",
            "--manifest-path",
            "plugins/kiro/Cargo.toml",
            "check",
        ])
        .with_exec([
            "cargo",
            "build",
            "--manifest-path",
            "plugins/kiro/Cargo.toml",
            "--release",
            "--target",
            "wasm32-wasip2",
        ])
        .with_exec([
            "cargo",
            "run",
            "-p",
            "kiro-cli",
            "--",
            "check",
            "--project",
            ".",
            "--all",
            "--fail-on",
            "error",
        ])
    )
    return await container.stdout()
