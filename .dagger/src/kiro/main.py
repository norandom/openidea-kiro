import dagger
from dagger import dag, function, object_type

CARGO_DENY = (
    "curl -sL"
    " https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz"
    " | tar -xz -C /usr/local/cargo/bin --strip-components=1"
)

PLUGIN = ["--manifest-path", "plugins/kiro/Cargo.toml"]

# The quality gate, in the order it runs. Each entry is one command.
GATE_STEPS = [
    ["cargo", "fmt", "--all", "--", "--check"],
    ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", "--workspace"],
    ["cargo", "test", "--workspace", "--doc"],
    ["cargo", "deny", "check"],
    ["cargo", "build", "-p", "kiro-core", "--target", "wasm32-wasip2"],
    ["cargo", "clippy", *PLUGIN, "--all-targets", "--", "-D", "warnings"],
    ["cargo", "test", *PLUGIN],
    ["cargo", "deny", *PLUGIN, "check"],
    ["cargo", "build", *PLUGIN, "--release", "--target", "wasm32-wasip2"],
    ["cargo", "run", "-p", "kiro-cli", "--", "check", "--project", ".", "--all", "--fail-on", "error"],
]


def code_gate(base: str) -> list[str]:
  """The code gate over the files that differ from `base`, with the rules that read the whole project."""
  return ["cargo", "run", "-p", "kiro-cli", "--", "--project", ".", "code", "check", "--changed", base]


def toolchain() -> dagger.Container:
  """A Rust container with the wasm target, clippy, rustfmt and cargo-deny."""
  return (
      dag.container()
      .from_("rust:latest")
      .with_exec(["rustup", "target", "add", "wasm32-wasip2"])
      .with_exec(["rustup", "component", "add", "clippy", "rustfmt"])
      .with_exec(["sh", "-c", CARGO_DENY])
  )


@object_type
class Kiro:

  @function
  async def check(self, source: dagger.Directory, base: str = "") -> str:
    """Runs the full SDLC quality gate in an isolated, reproducible Rust container.

    With `base` (a Git revision such as origin/main) the code gate also judges
    the files that differ from it. The source must then carry its Git history.
    """
    container = (
        toolchain()
        .with_mounted_cache("/usr/local/cargo/registry", dag.cache_volume("cargo-cache"))
        .with_mounted_cache("/workspace/target", dag.cache_volume("target-cache"))
        .with_directory("/workspace", source)
        .with_workdir("/workspace")
    )
    steps = GATE_STEPS + ([code_gate(base)] if base else [])
    for step in steps:
      container = container.with_exec(step)
    return await container.stdout()
