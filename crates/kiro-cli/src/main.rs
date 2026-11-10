//! Native command-line interface and CI quality gate for Kiro.
//!
//! `main` parses the command line and resolves the shared parameters; each
//! command is one function that returns its exit verdict or a failure. A
//! failure is printed as one line on standard error and exits with code 2.

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

use cli::{
    CheckArgs, Cli, CodeCheckArgs, CodeCommand, Command, GraphArgs, ImportArgs, ReportFormat,
    TelemetryCommand, TelemetryExportArgs, ValidateArgs, ViewArgs, VocabCommand,
};
use exit::{Exit, FailOn, Failure};
use fs::{FsSource, ProjectRoot};
use kiro_cli_common::{
    CliContext, EnvironmentValues, resolve_context, resolve_gate, resolve_report, write_report,
};
use telemetry_io::{RunTimer, TelemetryEnvironment};

/// What every command needs from the invocation.
struct Session {
    context: CliContext,
    environment: EnvironmentValues,
    telemetry_environment: TelemetryEnvironment,
}

impl Session {
    fn verbose(&self) -> bool {
        self.context.verbose()
    }

    fn root(&self) -> Result<ProjectRoot, Failure> {
        fs::resolve_root(&self.context.project)
    }

    fn source(&self, root: &ProjectRoot) -> FsSource {
        FsSource::new(root.clone(), self.verbose())
    }
}

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
    match run(&cli) {
        Ok(exit) => ExitCode::from(exit),
        Err(failure) => {
            eprintln!("{failure}");
            ExitCode::from(Exit::Failure)
        }
    }
}

fn run(cli: &Cli) -> Result<Exit, Failure> {
    // Shared parameters: command line, then environment, then default.
    let environment = EnvironmentValues::capture();
    let session = Session {
        context: resolve_context(&cli.global, &environment)?,
        environment,
        telemetry_environment: TelemetryEnvironment::capture(),
    };
    match &cli.command {
        Command::Check(args) => run_check(&session, args),
        Command::Status(args) => run_status(&session, args),
        Command::Graph(args) => run_graph(&session, args),
        Command::Tasks(args) => run_tasks(&session, args),
        Command::Code(CodeCommand::Check(args)) => run_code_check(&session, args),
        Command::Code(CodeCommand::Config) => run_code_config(&session),
        Command::Code(CodeCommand::Baseline) => run_code_baseline(&session),
        Command::Telemetry(TelemetryCommand::Export(args)) => run_telemetry_export(&session, args),
        Command::Vocab(VocabCommand::Validate(args)) => run_vocab_validate(&session, args),
        Command::Vocab(VocabCommand::Import(args)) => run_vocab_import(&session, args),
    }
}

/// Selects the specs of a check; an empty tree names the directory looked in.
fn select_specs(
    root: &ProjectRoot,
    selection: &cli::Selection,
    discovered: &[String],
) -> Result<Vec<String>, Failure> {
    check::select(selection, discovered).map_err(|failure| {
        if failure.0 == "no specs found under .kiro/specs" {
            Failure(format!("no specs found under {}/.kiro/specs", root.display))
        } else {
            failure
        }
    })
}

fn run_check(session: &Session, args: &CheckArgs) -> Result<Exit, Failure> {
    let settings = resolve_report(&args.report, &session.environment)?;
    let fail_on = resolve_gate(&args.gate, &session.environment)?;
    let root = session.root()?;
    let selection = args.selection()?;
    let config = args.config()?;
    let telemetry_plan = telemetry::plan(
        &root,
        args.telemetry.telemetry_dir.as_deref(),
        &session.telemetry_environment,
    )?;
    let mut timer = RunTimer::start();
    let discovered = fs::discover_features(&root)?;
    timer.phase("discover");
    if session.verbose() {
        eprintln!("discovered specs: {}", discovered.join(", "));
    }
    let specs = select_specs(&root, &selection, &discovered)?;
    let (report, features) = check::run_with_features(
        &root,
        session.source(&root),
        discovered,
        specs,
        &config,
        fail_on,
    );
    timer.phase("check");

    let output = match settings.format {
        ReportFormat::Text => report::text(&report),
        ReportFormat::Json => report::json(&report)?,
    };
    write_report(&settings.destination, output.as_bytes())?;
    timer.phase("render");
    // Telemetry is written after the report and the exit decision; a
    // failure is reported and changes neither.
    if let Some(plan) = &telemetry_plan
        && let Err(failure) = telemetry::emit(
            plan,
            &root,
            &session.telemetry_environment,
            &report,
            &features,
            timer,
        )
    {
        eprintln!("{failure}");
    }
    Ok(report.exit)
}

fn run_status(session: &Session, args: &ViewArgs) -> Result<Exit, Failure> {
    let root = session.root()?;
    let discovered = fs::discover_features(&root)?;
    let text = viz::status(&root, session.source(&root), &discovered, &args.spec)?;
    print!("{text}");
    Ok(Exit::Clean)
}

fn run_graph(session: &Session, args: &GraphArgs) -> Result<Exit, Failure> {
    let root = session.root()?;
    let discovered = fs::discover_features(&root)?;
    let text = viz::graph(
        &root,
        session.source(&root),
        &discovered,
        &args.spec,
        args.table,
    )?;
    print!("{text}");
    Ok(Exit::Clean)
}

fn run_tasks(session: &Session, args: &GraphArgs) -> Result<Exit, Failure> {
    let root = session.root()?;
    let discovered = fs::discover_features(&root)?;
    let text = viz::tasks(
        &root,
        session.source(&root),
        &discovered,
        &args.spec,
        args.table,
    )?;
    print!("{text}");
    Ok(Exit::Clean)
}

/// The text report of a code check, with the optional highest values and
/// the verdict line.
fn code_text(run: &code::CodeRun, fail_on: FailOn, verdict: Exit) -> String {
    let mut text = kiro_core::code::report::text(&run.report);
    if !run.highest.is_empty() {
        text.push_str("\nHighest values (a distribution, not a verdict):\n");
        for line in &run.highest {
            text.push_str(&format!("  {line}\n"));
        }
    }
    let word = match fail_on {
        FailOn::Error => "error",
        FailOn::Warning => "warning",
    };
    let result = if verdict == Exit::Clean {
        "pass"
    } else {
        "fail"
    };
    text.push_str(&format!("fail-on {word}: {result}\n"));
    text
}

fn run_code_check(session: &Session, args: &CodeCheckArgs) -> Result<Exit, Failure> {
    let settings = resolve_report(&args.report, &session.environment)?;
    let fail_on = resolve_gate(&args.gate, &session.environment)?;
    let selection = code::selection(args)?;
    let root = session.root()?;
    let discovered = fs::discover_features(&root)?;
    let run = code::run(
        &root,
        session.source(&root),
        discovered,
        &selection,
        args.show_highest,
    )?;
    let verdict = exit::verdict(fail_on, run.report.blocking_findings());
    let output = match settings.format {
        ReportFormat::Text if args.summary => code_summary(&run, fail_on, verdict),
        ReportFormat::Text => code_text(&run, fail_on, verdict),
        ReportFormat::Json => kiro_core::code::report::json(&run.report),
    };
    write_report(&settings.destination, output.as_bytes())?;
    Ok(verdict)
}

/// The hook summary of a code check: what improved and what worsened.
fn code_summary(run: &code::CodeRun, fail_on: FailOn, verdict: Exit) -> String {
    let blocking = run
        .report
        .blocking_findings()
        .filter(|finding| exit::meets(fail_on, finding.severity))
        .count();
    let word = match fail_on {
        FailOn::Error => "error",
        FailOn::Warning => "warning",
    };
    let result = if verdict == Exit::Clean {
        "PASS"
    } else {
        "FAIL"
    };
    kiro_core::code::report::summary(&run.report, result, word, blocking)
}

/// Measures the whole tree and writes the baseline the ratchet compares
/// with. Only the named baseline file is written.
fn run_code_baseline(session: &Session) -> Result<Exit, Failure> {
    let root = session.root()?;
    let document = code::baseline_document(&root)?;
    fs::write_output(&root.join(code::BASELINE_PATH), &document)?;
    eprintln!("wrote {}", code::BASELINE_PATH);
    Ok(Exit::Clean)
}

/// Prints the effective code configuration with origins and reasons.
fn run_code_config(session: &Session) -> Result<Exit, Failure> {
    let root = session.root()?;
    let config = code::load_config(&root)?;
    print!("{}", config.effective_text());
    println!("digest {}", config.digest());
    Ok(Exit::Clean)
}

fn run_telemetry_export(session: &Session, args: &TelemetryExportArgs) -> Result<Exit, Failure> {
    let settings = resolve_report(&args.report, &session.environment)?;
    let selection = cli::selection_of(args.all, &args.features)?;
    let root = session.root()?;
    // A dry run prints the snapshot and writes nothing, whatever the
    // environment says about a telemetry directory.
    let telemetry_plan = if args.dry_run {
        None
    } else {
        let plan = telemetry::plan(
            &root,
            args.telemetry.telemetry_dir.as_deref(),
            &session.telemetry_environment,
        )?;
        Some(plan.ok_or_else(|| {
            Failure::new(
                "no telemetry directory: pass --telemetry-dir, set KIRO_TELEMETRY_DIR, or use --dry-run",
            )
        })?)
    };
    let mut timer = RunTimer::start();
    let discovered = fs::discover_features(&root)?;
    timer.phase("discover");
    let specs = check::select(&selection, &discovered)?;
    let config = kiro_core::checks::CheckConfig::default();
    let (report, features) = check::run_with_features(
        &root,
        session.source(&root),
        discovered,
        specs,
        &config,
        FailOn::Error,
    );
    timer.phase("check");
    match telemetry_plan {
        None => {
            let snapshot = telemetry::snapshot_of(&report, &features);
            let output = match settings.format {
                ReportFormat::Text => kiro_core::telemetry::snapshot_text(&snapshot),
                ReportFormat::Json => kiro_core::telemetry::snapshot_json(&snapshot),
            };
            write_report(&settings.destination, output.as_bytes())?;
        }
        // The export is this command's whole purpose, so a failed write is
        // a failure here.
        Some(plan) => telemetry::emit(
            &plan,
            &root,
            &session.telemetry_environment,
            &report,
            &features,
            timer,
        )?,
    }
    Ok(Exit::Clean)
}

fn run_vocab_validate(session: &Session, args: &ValidateArgs) -> Result<Exit, Failure> {
    let root = session.root()?;
    let (rep, exit) = vocab::validate(
        &root,
        session.source(&root),
        args.file.as_deref(),
        args.fail_on,
    )?;
    let output = match args.format {
        ReportFormat::Text => report::vocab_text(&rep),
        ReportFormat::Json => report::vocab_json(&rep)?,
    };
    std::io::stdout()
        .write_all(output.as_bytes())
        .map_err(|e| Failure::new(format!("failed to write report to stdout: {e}")))?;
    Ok(exit)
}

fn run_vocab_import(session: &Session, args: &ImportArgs) -> Result<Exit, Failure> {
    let root = session.root()?;
    vocab::import(&root, args, session.verbose())
}
