//! Native command-line interface and CI quality gate for Kiro.

use std::process::ExitCode;

pub mod exit;

fn main() -> ExitCode {
    // Minimal entry point for task 1.1; command-line parser wired in task 2.2
    ExitCode::from(exit::Exit::Clean)
}
