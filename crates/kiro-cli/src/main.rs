//! Native command-line interface and CI quality gate for Kiro.

use std::io::Write;
use std::process::ExitCode;

use clap::Parser;

pub mod check;
pub mod cli;
pub mod code;
pub mod exit;
pub mod fs;
pub mod import;
pub mod report;
pub mod telemetry;
pub mod telemetry_io;
pub mod viz;
pub mod vocab;

use cli::{Cli, CodeCommand, Command, TelemetryCommand, VocabCommand};
use exit::Exit;
use kiro_cli_common::{
    EnvironmentValues, resolve_context, resolve_gate, resolve_report, write_report,
};
use telemetry_io::{RunTimer, TelemetryEnvironment};

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

    // Shared parameters: command line, then environment, then default.
    let environment = EnvironmentValues::capture();
    let context = match resolve_context(&cli.global, &environment) {
        Ok(c) => c,
        Err(failure) => {
            eprintln!("{failure}");
            return ExitCode::from(Exit::Failure);
        }
    };
    let verbose = context.verbose();
    let telemetry_environment = TelemetryEnvironment::capture();

    match cli.command {
        Command::Check(ref args) => {
            let settings = match resolve_report(&args.report, &environment) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let fail_on = match resolve_gate(&args.gate, &environment) {
                Ok(f) => f,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let root = match fs::resolve_root(&context.project) {
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
            let telemetry_plan = match telemetry::plan(
                &root,
                args.telemetry.telemetry_dir.as_deref(),
                &telemetry_environment,
            ) {
                Ok(p) => p,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let mut timer = RunTimer::start();
            let discovered = match fs::discover_features(&root) {
                Ok(d) => d,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            timer.phase("discover");
            if verbose {
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
            let source = fs::FsSource::new(root.clone(), verbose);
            let (report, features) =
                check::run_with_features(&root, source, discovered, specs, &config, fail_on);
            timer.phase("check");

            let output = match settings.format {
                cli::ReportFormat::Text => report::text(&report),
                cli::ReportFormat::Json => match report::json(&report) {
                    Ok(j) => j,
                    Err(failure) => {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                },
            };
            if let Err(failure) = write_report(&settings.destination, output.as_bytes()) {
                eprintln!("{failure}");
                return ExitCode::from(Exit::Failure);
            }
            timer.phase("render");
            // Telemetry is written after the report and the exit decision;
            // a failure is reported and changes neither.
            if let Some(plan) = &telemetry_plan
                && let Err(failure) = telemetry::emit(
                    plan,
                    &root,
                    &telemetry_environment,
                    &report,
                    &features,
                    timer,
                )
            {
                eprintln!("{failure}");
            }
            ExitCode::from(report.exit)
        }
        Command::Status(ref args) => {
            let root = match fs::resolve_root(&context.project) {
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
            let source = fs::FsSource::new(root.clone(), verbose);
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
            let root = match fs::resolve_root(&context.project) {
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
            let source = fs::FsSource::new(root.clone(), verbose);
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
            let root = match fs::resolve_root(&context.project) {
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
            let source = fs::FsSource::new(root.clone(), verbose);
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
        Command::Code(CodeCommand::Check(ref args)) => {
            let settings = match resolve_report(&args.report, &environment) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let fail_on = match resolve_gate(&args.gate, &environment) {
                Ok(f) => f,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let selection = match code::selection(args) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let root = match fs::resolve_root(&context.project) {
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
            let source = fs::FsSource::new(root.clone(), verbose);
            let run = match code::run(&root, source, discovered, &selection, args.show_highest) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let verdict = exit::verdict(fail_on, run.report.base_findings());
            let output = match settings.format {
                cli::ReportFormat::Text => {
                    let mut text = kiro_core::code::report::text(&run.report);
                    if !run.highest.is_empty() {
                        text.push_str("\nHighest values (a distribution, not a verdict):\n");
                        for line in &run.highest {
                            text.push_str(&format!("  {line}\n"));
                        }
                    }
                    let word = match fail_on {
                        exit::FailOn::Error => "error",
                        exit::FailOn::Warning => "warning",
                    };
                    let result = if verdict == Exit::Clean {
                        "pass"
                    } else {
                        "fail"
                    };
                    text.push_str(&format!("fail-on {word}: {result}\n"));
                    text
                }
                cli::ReportFormat::Json => kiro_core::code::report::json(&run.report),
            };
            if let Err(failure) = write_report(&settings.destination, output.as_bytes()) {
                eprintln!("{failure}");
                return ExitCode::from(Exit::Failure);
            }
            ExitCode::from(verdict)
        }
        Command::Telemetry(TelemetryCommand::Export(ref args)) => {
            let settings = match resolve_report(&args.report, &environment) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let selection = match cli::selection_of(args.all, &args.features) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let root = match fs::resolve_root(&context.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            // A dry run prints the snapshot and writes nothing, whatever
            // the environment says about a telemetry directory.
            let telemetry_plan = if args.dry_run {
                None
            } else {
                match telemetry::plan(
                    &root,
                    args.telemetry.telemetry_dir.as_deref(),
                    &telemetry_environment,
                ) {
                    Ok(Some(p)) => Some(p),
                    Ok(None) => {
                        eprintln!(
                            "no telemetry directory: pass --telemetry-dir, set KIRO_TELEMETRY_DIR, or use --dry-run"
                        );
                        return ExitCode::from(Exit::Failure);
                    }
                    Err(failure) => {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                }
            };
            let mut timer = RunTimer::start();
            let discovered = match fs::discover_features(&root) {
                Ok(d) => d,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            timer.phase("discover");
            let specs = match check::select(&selection, &discovered) {
                Ok(s) => s,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let source = fs::FsSource::new(root.clone(), verbose);
            let config = kiro_core::checks::CheckConfig::default();
            let (report, features) = check::run_with_features(
                &root,
                source,
                discovered,
                specs,
                &config,
                exit::FailOn::Error,
            );
            timer.phase("check");
            match telemetry_plan {
                None => {
                    let snapshot = telemetry::snapshot_of(&report, &features);
                    let output = match settings.format {
                        cli::ReportFormat::Text => kiro_core::telemetry::snapshot_text(&snapshot),
                        cli::ReportFormat::Json => kiro_core::telemetry::snapshot_json(&snapshot),
                    };
                    if let Err(failure) = write_report(&settings.destination, output.as_bytes()) {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                }
                Some(plan) => {
                    // The export is this command's whole purpose, so a
                    // failed write is a failure here.
                    if let Err(failure) = telemetry::emit(
                        &plan,
                        &root,
                        &telemetry_environment,
                        &report,
                        &features,
                        timer,
                    ) {
                        eprintln!("{failure}");
                        return ExitCode::from(Exit::Failure);
                    }
                }
            }
            ExitCode::from(Exit::Clean)
        }
        Command::Vocab(VocabCommand::Validate(ref args)) => {
            let root = match fs::resolve_root(&context.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            let source = fs::FsSource::new(root.clone(), verbose);
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
            let root = match fs::resolve_root(&context.project) {
                Ok(r) => r,
                Err(failure) => {
                    eprintln!("{failure}");
                    return ExitCode::from(Exit::Failure);
                }
            };
            match vocab::import(&root, args, verbose) {
                Ok(exit) => ExitCode::from(exit),
                Err(failure) => {
                    eprintln!("{failure}");
                    ExitCode::from(Exit::Failure)
                }
            }
        }
    }
}
