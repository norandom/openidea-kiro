//! Native command-line interface and CI quality gate for Kiro.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

pub mod check;
pub mod cli;
pub mod exit;
pub mod fs;
pub mod report;

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
        Command::Check(ref args) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let selection = match args.selection() {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let config = match args.config() {
                Ok(c) => c,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let discovered = match fs::discover_features(&root) {
                Ok(d) => d,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            if cli.verbose {
                eprintln!("discovered specs: {}", discovered.join(", "));
            }
            let specs = match check::select(&selection, &discovered) {
                Ok(s) => s,
                Err(failure) => {
                    let msg = if failure.0 == "no specs found under .kiro/specs" {
                        format!("no specs found under {}/.kiro/specs", root.display)
                    } else {
                        failure.0
                    };
                    eprintln!("{msg}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let source = fs::FsSource::new(root.clone(), cli.verbose);
            let report = check::run(&root, source, discovered, specs, &config, args.fail_on);

            let output = match args.format {
                cli::ReportFormat::Text => report::text(&report),
                cli::ReportFormat::Json => match report::json(&report) {
                    Ok(j) => j,
                    Err(failure) => {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                },
            };
            if let Err(e) = std::io::stdout().write_all(output.as_bytes()) {
                eprintln!("failed to write report to stdout: {e}");
                return ExitCode::from(Exit::Failure);
            }
            ExitCode::from(report.exit)
        }
        _ => ExitCode::from(Exit::Clean),
    }
}
