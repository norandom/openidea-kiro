//! Native command-line interface and CI quality gate for Kiro.

use std::process::ExitCode;

use clap::Parser;

pub mod cli;
pub mod exit;
pub mod fs;

use cli::{Cli, Command};
use exit::Exit;

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            if e.use_stderr() {
                eprint!("{e}");
                return ExitCode::from(Exit::Failure);
            }
            print!("{e}");
            return ExitCode::from(Exit::Clean);
        }
    };

    match cli.command {
        Command::Check(ref args) => match args.selection() {
            Ok(_) => ExitCode::from(Exit::Clean),
            Err(failure) => {
                eprintln!("{failure}");
                ExitCode::from(Exit::Failure)
            }
        },
        _ => ExitCode::from(Exit::Clean),
    }
}
