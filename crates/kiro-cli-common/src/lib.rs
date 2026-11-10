//! Shared command-line contract for kiro tools (spec `cli-contracts`).
//!
//! One implementation of the parameter groups, their environment fallbacks,
//! the 0/1/2 exit model and report routing. A command composes only the
//! groups that mean something for it and declares that in a
//! [`CommandProfile`].
//!
//! Precedence for every shared parameter: command line, then the
//! environment variable, then the built-in default. The library reads no
//! source tree and interprets no rule.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{ArgAction, Args, ValueEnum};
use kiro_core::finding::{Finding, Severity};

pub mod output;

pub use output::{Destination, write_report};

/// Environment variable naming the project root.
pub const ENV_PROJECT: &str = "KIRO_PROJECT";
/// Environment variable naming the report format.
pub const ENV_FORMAT: &str = "KIRO_FORMAT";
/// Environment variable naming the gate threshold.
pub const ENV_FAIL_ON: &str = "KIRO_FAIL_ON";
/// Disables colour when present and not empty.
pub const ENV_NO_COLOR: &str = "NO_COLOR";
/// Forces colour when present and not `0`.
pub const ENV_CLICOLOR_FORCE: &str = "CLICOLOR_FORCE";

// ---------------------------------------------------------------- exit model

/// Process exit verdict for command execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// Success / clean execution (exit code 0).
    Clean,
    /// Execution completed with findings meeting the `--fail-on` threshold (exit code 1).
    Findings,
    /// Unrecoverable CLI, I/O, or usage failure (exit code 2).
    Failure,
}

impl Exit {
    /// Returns the integer exit code value (0, 1, or 2).
    #[must_use]
    pub fn code(self) -> u8 {
        match self {
            Exit::Clean => 0,
            Exit::Findings => 1,
            Exit::Failure => 2,
        }
    }
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(exit.code())
    }
}

/// A single-line error reason printed to stderr by main; always produces exit code 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure(pub String);

impl Failure {
    /// Constructs a new `Failure`, flattening any newlines into `"; "` and trimming parts.
    pub fn new(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        let parts: Vec<&str> = msg
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        Self(parts.join("; "))
    }
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for Failure {}

/// Finding severity threshold required to trigger non-zero exit (`Exit::Findings`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FailOn {
    /// Fails only on `Severity::Error`.
    Error,
    /// Fails on `Severity::Error` or `Severity::Warning`.
    Warning,
}

/// Determines whether a finding's severity meets the `--fail-on` threshold.
///
/// Error always meets; Warning meets when `fail_on == FailOn::Warning`; Info never meets.
#[must_use]
pub fn meets(fail_on: FailOn, severity: Severity) -> bool {
    match (fail_on, severity) {
        (_, Severity::Error) | (FailOn::Warning, Severity::Warning) => true,
        (FailOn::Error, Severity::Warning) | (_, Severity::Info) => false,
    }
}

/// Computes the process exit verdict over an iterator of findings.
///
/// The caller has already removed findings that are not eligible to block
/// (for example pre-existing debt); this function interprets no rule.
pub fn verdict<'a>(fail_on: FailOn, findings: impl IntoIterator<Item = &'a Finding>) -> Exit {
    if findings.into_iter().any(|f| meets(fail_on, f.severity)) {
        Exit::Findings
    } else {
        Exit::Clean
    }
}

/// Parses a severity name (case-insensitive) into `Severity`.
#[must_use]
pub fn parse_severity(s: &str) -> Option<Severity> {
    if s.eq_ignore_ascii_case("error") {
        Some(Severity::Error)
    } else if s.eq_ignore_ascii_case("warning") {
        Some(Severity::Warning)
    } else if s.eq_ignore_ascii_case("info") {
        Some(Severity::Info)
    } else {
        None
    }
}

// ---------------------------------------------------------- parameter groups

/// Color choices for terminal output.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

/// Output format for reports.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Text,
    Json,
}

/// Parameters every command accepts.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct GlobalArgs {
    /// Path to the project root directory containing .kiro/ [env: KIRO_PROJECT] [default: .]
    #[arg(long, visible_alias = "root", global = true, value_name = "PROJECT")]
    pub project: Option<PathBuf>,

    /// Verbose diagnostic output on stderr (repeat for more)
    #[arg(short, long, global = true, action = ArgAction::Count)]
    pub verbose: u8,

    /// Suppress diagnostic output on stderr
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Color configuration for output [env: NO_COLOR, CLICOLOR_FORCE] [default: auto]
    #[arg(long, global = true, value_enum, value_name = "COLOR")]
    pub color: Option<ColorChoice>,
}

/// Parameters of a command that prints a report.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct ReportArgs {
    /// Output report format [env: KIRO_FORMAT] [default: text]
    #[arg(long, value_enum, value_name = "FORMAT")]
    pub format: Option<ReportFormat>,

    /// Write the report to this file instead of standard output ('-' for standard output)
    #[arg(long, value_name = "PATH")]
    pub output: Option<PathBuf>,
}

/// Parameters of a command that gates on findings.
#[derive(Args, Debug, Clone, PartialEq, Eq, Default)]
pub struct GateArgs {
    /// Severity threshold that triggers exit code 1 [env: KIRO_FAIL_ON] [default: error]
    #[arg(long, value_enum, value_name = "FAIL_ON")]
    pub fail_on: Option<FailOn>,
}

// ------------------------------------------------------------- environment

/// The environment values the shared groups may read. Nothing else in the
/// environment is inspected.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EnvironmentValues {
    pub project: Option<String>,
    pub format: Option<String>,
    pub fail_on: Option<String>,
    pub no_color: Option<String>,
    pub clicolor_force: Option<String>,
}

impl EnvironmentValues {
    /// Reads the five supported variables from the process environment.
    #[must_use]
    pub fn capture() -> Self {
        let read = |name: &str| std::env::var(name).ok();
        Self {
            project: read(ENV_PROJECT),
            format: read(ENV_FORMAT),
            fail_on: read(ENV_FAIL_ON),
            no_color: read(ENV_NO_COLOR),
            clicolor_force: read(ENV_CLICOLOR_FORCE),
        }
    }
}

/// How much the command says on standard error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet,
    Normal,
    /// `-v` given this many times (at least 1).
    Verbose(u8),
}

/// The resolved global parameters of one invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliContext {
    pub project: PathBuf,
    pub verbosity: Verbosity,
    pub color: ColorChoice,
}

impl CliContext {
    /// Whether verbose diagnostics were requested.
    #[must_use]
    pub fn verbose(&self) -> bool {
        matches!(self.verbosity, Verbosity::Verbose(_))
    }
}

/// The resolved report parameters of one invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportSettings {
    pub format: ReportFormat,
    pub destination: Destination,
}

fn parse_enum<T: ValueEnum>(variable: &str, value: &str) -> Result<T, Failure> {
    T::from_str(value, true).map_err(|_| {
        let allowed: Vec<String> = T::value_variants()
            .iter()
            .filter_map(|variant| variant.to_possible_value())
            .map(|possible| possible.get_name().to_string())
            .collect();
        Failure::new(format!(
            "invalid value in environment variable {variable}; expected one of: {}",
            allowed.join(", ")
        ))
    })
}

fn non_empty(value: Option<&String>) -> Option<&str> {
    value.map(String::as_str).filter(|text| !text.is_empty())
}

/// Resolves the global parameters: command line, then environment, then default.
///
/// `-q` together with `-v` is a usage error. Colour: an explicit `--color`
/// wins; otherwise a non-empty `NO_COLOR` selects never; otherwise a
/// `CLICOLOR_FORCE` other than `0` selects always; otherwise auto.
pub fn resolve_context(args: &GlobalArgs, env: &EnvironmentValues) -> Result<CliContext, Failure> {
    if args.quiet && args.verbose > 0 {
        return Err(Failure::new(
            "--quiet and --verbose cannot be used together",
        ));
    }
    let project = match (&args.project, non_empty(env.project.as_ref())) {
        (Some(path), _) => path.clone(),
        (None, Some(path)) => PathBuf::from(path),
        (None, None) => PathBuf::from("."),
    };
    let verbosity = if args.quiet {
        Verbosity::Quiet
    } else if args.verbose > 0 {
        Verbosity::Verbose(args.verbose)
    } else {
        Verbosity::Normal
    };
    let color = if let Some(color) = args.color {
        color
    } else if non_empty(env.no_color.as_ref()).is_some() {
        ColorChoice::Never
    } else if non_empty(env.clicolor_force.as_ref()).is_some_and(|value| value != "0") {
        ColorChoice::Always
    } else {
        ColorChoice::Auto
    };
    Ok(CliContext {
        project,
        verbosity,
        color,
    })
}

/// Resolves format and destination of a report.
pub fn resolve_report(
    args: &ReportArgs,
    env: &EnvironmentValues,
) -> Result<ReportSettings, Failure> {
    let format = match (args.format, non_empty(env.format.as_ref())) {
        (Some(format), _) => format,
        (None, Some(value)) => parse_enum(ENV_FORMAT, value)?,
        (None, None) => ReportFormat::Text,
    };
    let destination = match &args.output {
        Some(path) if path.as_os_str() != "-" => Destination::File(path.clone()),
        _ => Destination::Stdout,
    };
    Ok(ReportSettings {
        format,
        destination,
    })
}

/// Resolves the gate threshold.
pub fn resolve_gate(args: &GateArgs, env: &EnvironmentValues) -> Result<FailOn, Failure> {
    match (args.fail_on, non_empty(env.fail_on.as_ref())) {
        (Some(fail_on), _) => Ok(fail_on),
        (None, Some(value)) => parse_enum(ENV_FAIL_ON, value),
        (None, None) => Ok(FailOn::Error),
    }
}

// ---------------------------------------------------------------- profiles

/// Which shared groups a command composes. The conformance suite reads
/// these declarations; a command without one fails it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandProfile {
    /// Command path with spaces, such as `"vocab import"`.
    pub id: &'static str,
    /// Composes the shared report group (`--format`, `--output`).
    pub reports: bool,
    /// Composes the shared gate group (`--fail-on`).
    pub gates: bool,
    /// Writes an artifact that is not a report.
    pub artifact: bool,
    /// Flags that keep a legacy meaning differing from a shared group, with the reason.
    pub exceptions: &'static [(&'static str, &'static str)],
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    fn env() -> EnvironmentValues {
        EnvironmentValues::default()
    }

    #[test]
    fn project_precedence_is_cli_then_env_then_default() {
        let from_env = EnvironmentValues {
            project: Some("from-env".into()),
            ..env()
        };
        let cli = GlobalArgs {
            project: Some(PathBuf::from("from-cli")),
            ..GlobalArgs::default()
        };
        let cases = [
            (GlobalArgs::default(), env(), "."),
            (GlobalArgs::default(), from_env.clone(), "from-env"),
            (cli.clone(), env(), "from-cli"),
            (cli, from_env, "from-cli"),
            (
                GlobalArgs::default(),
                EnvironmentValues {
                    project: Some(String::new()),
                    ..env()
                },
                ".",
            ),
        ];
        for (args, environment, expected) in cases {
            let context = resolve_context(&args, &environment).unwrap();
            assert_eq!(context.project, PathBuf::from(expected));
        }
    }

    #[test]
    fn verbosity_and_its_conflict() {
        let quiet = GlobalArgs {
            quiet: true,
            ..GlobalArgs::default()
        };
        let verbose = GlobalArgs {
            verbose: 2,
            ..GlobalArgs::default()
        };
        let both = GlobalArgs {
            quiet: true,
            verbose: 1,
            ..GlobalArgs::default()
        };
        assert_eq!(
            resolve_context(&GlobalArgs::default(), &env())
                .unwrap()
                .verbosity,
            Verbosity::Normal
        );
        assert_eq!(
            resolve_context(&quiet, &env()).unwrap().verbosity,
            Verbosity::Quiet
        );
        let context = resolve_context(&verbose, &env()).unwrap();
        assert_eq!(context.verbosity, Verbosity::Verbose(2));
        assert!(context.verbose());
        let failure = resolve_context(&both, &env()).unwrap_err();
        assert!(failure.0.contains("--quiet and --verbose"));
    }

    #[test]
    fn colour_precedence() {
        let no_color = EnvironmentValues {
            no_color: Some("1".into()),
            ..env()
        };
        let force = EnvironmentValues {
            clicolor_force: Some("1".into()),
            ..env()
        };
        let force_zero = EnvironmentValues {
            clicolor_force: Some("0".into()),
            ..env()
        };
        let both = EnvironmentValues {
            no_color: Some("1".into()),
            clicolor_force: Some("1".into()),
            ..env()
        };
        let empty_no_color = EnvironmentValues {
            no_color: Some(String::new()),
            ..env()
        };
        let always = GlobalArgs {
            color: Some(ColorChoice::Always),
            ..GlobalArgs::default()
        };
        let cases = [
            (GlobalArgs::default(), env(), ColorChoice::Auto),
            (GlobalArgs::default(), no_color.clone(), ColorChoice::Never),
            (GlobalArgs::default(), force, ColorChoice::Always),
            (GlobalArgs::default(), force_zero, ColorChoice::Auto),
            (GlobalArgs::default(), both, ColorChoice::Never),
            (GlobalArgs::default(), empty_no_color, ColorChoice::Auto),
            (always, no_color, ColorChoice::Always),
        ];
        for (args, environment, expected) in cases {
            assert_eq!(
                resolve_context(&args, &environment).unwrap().color,
                expected
            );
        }
    }

    #[test]
    fn report_format_precedence_and_invalid_environment() {
        let json_env = EnvironmentValues {
            format: Some("JSON".into()),
            ..env()
        };
        let text_cli = ReportArgs {
            format: Some(ReportFormat::Text),
            ..ReportArgs::default()
        };
        assert_eq!(
            resolve_report(&ReportArgs::default(), &env())
                .unwrap()
                .format,
            ReportFormat::Text
        );
        assert_eq!(
            resolve_report(&ReportArgs::default(), &json_env)
                .unwrap()
                .format,
            ReportFormat::Json
        );
        assert_eq!(
            resolve_report(&text_cli, &json_env).unwrap().format,
            ReportFormat::Text
        );
        let bad = EnvironmentValues {
            format: Some("yaml".into()),
            project: Some("secret-project-path".into()),
            ..env()
        };
        let failure = resolve_report(&ReportArgs::default(), &bad).unwrap_err();
        assert!(failure.0.contains("KIRO_FORMAT"), "{failure}");
        assert!(failure.0.contains("text, json"), "{failure}");
        assert!(!failure.0.contains("yaml"), "the value is not echoed");
        assert!(!failure.0.contains("secret"), "no unrelated value leaks");
    }

    #[test]
    fn report_destination() {
        let dash = ReportArgs {
            output: Some(PathBuf::from("-")),
            ..ReportArgs::default()
        };
        let file = ReportArgs {
            output: Some(PathBuf::from("out/report.json")),
            ..ReportArgs::default()
        };
        assert_eq!(
            resolve_report(&ReportArgs::default(), &env())
                .unwrap()
                .destination,
            Destination::Stdout
        );
        assert_eq!(
            resolve_report(&dash, &env()).unwrap().destination,
            Destination::Stdout
        );
        assert_eq!(
            resolve_report(&file, &env()).unwrap().destination,
            Destination::File(PathBuf::from("out/report.json"))
        );
    }

    #[test]
    fn gate_precedence_and_invalid_environment() {
        let warning_env = EnvironmentValues {
            fail_on: Some("warning".into()),
            ..env()
        };
        let error_cli = GateArgs {
            fail_on: Some(FailOn::Error),
        };
        assert_eq!(
            resolve_gate(&GateArgs::default(), &env()).unwrap(),
            FailOn::Error
        );
        assert_eq!(
            resolve_gate(&GateArgs::default(), &warning_env).unwrap(),
            FailOn::Warning
        );
        assert_eq!(
            resolve_gate(&error_cli, &warning_env).unwrap(),
            FailOn::Error
        );
        let bad = EnvironmentValues {
            fail_on: Some("info".into()),
            ..env()
        };
        let failure = resolve_gate(&GateArgs::default(), &bad).unwrap_err();
        assert!(failure.0.contains("KIRO_FAIL_ON"), "{failure}");
        assert!(failure.0.contains("error, warning"), "{failure}");
    }

    #[test]
    fn exit_codes_and_verdict() {
        assert_eq!(Exit::Clean.code(), 0);
        assert_eq!(Exit::Findings.code(), 1);
        assert_eq!(Exit::Failure.code(), 2);
        let warning = Finding::new(
            kiro_core::finding::codes::SPEC_DOC_MISSING,
            "file.md",
            "warn",
        )
        .severity(Severity::Warning);
        assert_eq!(verdict(FailOn::Error, [&warning]), Exit::Clean);
        assert_eq!(verdict(FailOn::Warning, [&warning]), Exit::Findings);
        assert_eq!(verdict(FailOn::Warning, []), Exit::Clean);
    }

    #[test]
    fn failure_is_one_line() {
        assert_eq!(Failure::new("a\n  b \n\nc").0, "a; b; c");
    }
}
