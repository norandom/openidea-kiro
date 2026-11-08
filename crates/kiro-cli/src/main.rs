//! Native command-line interface and CI quality gate for Kiro.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

pub mod check;
pub mod cli;
pub mod exit;
pub mod fs;
pub mod report;
pub mod viz;
pub mod vocab;

use cli::{Cli, Command, VocabCommand};
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
        Command::Status(ref args) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
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
            let source = fs::FsSource::new(root.clone(), cli.verbose);
            match viz::status(&root, source, &discovered, &args.spec) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::from(Exit::Clean)
                }
                Err(failure) => {
                    eprintln!("{failure}");
                    ExitCode::from(Exit::Failure)
                }
            }
        }
        Command::Graph(ref args) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
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
            let source = fs::FsSource::new(root.clone(), cli.verbose);
            match viz::graph(&root, source, &discovered, &args.spec, args.table) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::from(Exit::Clean)
                }
                Err(failure) => {
                    eprintln!("{failure}");
                    ExitCode::from(Exit::Failure)
                }
            }
        }
        Command::Tasks(ref args) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
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
            let source = fs::FsSource::new(root.clone(), cli.verbose);
            match viz::tasks(&root, source, &discovered, &args.spec, args.table) {
                Ok(text) => {
                    print!("{text}");
                    ExitCode::from(Exit::Clean)
                }
                Err(failure) => {
                    eprintln!("{failure}");
                    ExitCode::from(Exit::Failure)
                }
            }
        }
        Command::Vocab(VocabCommand::Validate(ref args)) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let source = fs::FsSource::new(root.clone(), cli.verbose);
            let (rep, exit) =
                match vocab::validate(&root, source, args.file.as_deref(), args.fail_on) {
                    Ok(pair) => pair,
                    Err(failure) => {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                };
            let output = match args.format {
                cli::ReportFormat::Text => report::vocab_text(&rep),
                cli::ReportFormat::Json => match report::vocab_json(&rep) {
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
            ExitCode::from(exit)
        }
        Command::Vocab(VocabCommand::Import(ref args)) => {
            let root = match fs::resolve_root(&cli.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            match vocab::import(&root, args) {
                Ok(exit) => ExitCode::from(exit),
                Err(failure) => {
                    eprintln!("{failure}");
                    ExitCode::from(Exit::Failure)
                }
            }
        }
    }
}
