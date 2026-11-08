//! Command line interface definition and configuration translation.

use std::collections::HashSet;
use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use kiro_core::checks::CheckConfig;
use kiro_core::finding::CHECK_CODES;

use crate::exit::{FailOn, Failure, parse_severity};

/// Global command-line arguments and subcommands for `kiro-cli`.
#[derive(Parser, Debug, Clone)]
#[command(
    name = "kiro-cli",
    version,
    about = "Spec constraint gate and tooling for kiro / cc-sdd projects"
)]
pub struct Cli {
    /// Path to the project root directory containing .kiro/
    #[arg(long, visible_alias = "root", global = true, default_value = ".")]
    pub project: PathBuf,

    /// Verbose diagnostic output on stderr
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Color configuration for output
    #[arg(long, global = true, value_enum, default_value = "auto")]
    pub color: ColorChoice,

    /// Command to execute
    #[command(subcommand)]
    pub command: Command,
}

/// Color choices for terminal output.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

/// Available subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Run the spec checks over the project and exit non-zero on findings.
    Check(CheckArgs),
    /// Print the status view of a spec.
    Status(ViewArgs),
    /// Print the spec graph of a spec.
    Graph(GraphArgs),
    /// Print the task graph of a spec.
    Tasks(GraphArgs),
    /// Vocabulary tools.
    #[command(subcommand)]
    Vocab(VocabCommand),
}

/// Output format for reports.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Text,
    Json,
}

/// Names of individual spec check suites.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckName {
    Ears,
    Terms,
    Contradictions,
}

/// Arguments for the `check` subcommand.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct CheckArgs {
    /// Explicit spec features to check (repeatable)
    #[arg(long = "feature")]
    pub features: Vec<String>,

    /// Check all discovered specs
    #[arg(long)]
    pub all: bool,

    /// Output report format
    #[arg(long, value_enum, default_value = "text")]
    pub format: ReportFormat,

    /// Severity threshold that triggers exit code 1
    #[arg(long, value_enum, default_value = "error")]
    pub fail_on: FailOn,

    /// Subset of check suites to run (ears, terms, contradictions)
    #[arg(long, value_delimiter = ',', value_enum)]
    pub checks: Option<Vec<CheckName>>,

    /// Report informational EARS pattern matches
    #[arg(long)]
    pub patterns: bool,

    /// Disable automatic vocabulary term suggestions
    #[arg(long)]
    pub no_suggest: bool,

    /// Maximum variables cap for contradiction checking
    #[arg(long)]
    pub max_variables: Option<usize>,

    /// Maximum BDD nodes cap for contradiction checking
    #[arg(long)]
    pub max_nodes: Option<usize>,

    /// Severity override for a finding code (CODE=LEVEL, e.g. EARS_NO_SHALL=info)
    #[arg(long = "severity", value_name = "CODE=LEVEL")]
    pub severities: Vec<String>,
}

/// Selection of specs to check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    All,
    Features(Vec<String>),
}

impl CheckArgs {
    /// Resolves the spec selection from `--feature` or `--all`.
    ///
    /// Requires either `--feature` or `--all`. Deduplicates features preserving
    /// first-seen order.
    pub fn selection(&self) -> Result<Selection, Failure> {
        if self.all && !self.features.is_empty() {
            return Err(Failure::new(
                "specify either --all or one or more --feature arguments, not both",
            ));
        }
        if !self.all && self.features.is_empty() {
            return Err(Failure::new(
                "specify either --all or one or more --feature arguments",
            ));
        }
        if self.all {
            return Ok(Selection::All);
        }
        let mut seen = HashSet::new();
        let mut deduped = Vec::new();
        for f in &self.features {
            if seen.insert(f) {
                deduped.push(f.clone());
            }
        }
        Ok(Selection::Features(deduped))
    }

    /// Translates check arguments to the engine's [`CheckConfig`].
    pub fn config(&self) -> Result<CheckConfig, Failure> {
        let mut config = CheckConfig::default();

        if let Some(ref checks) = self.checks {
            config.ears = false;
            config.terms = false;
            config.contradictions = false;
            for check in checks {
                match check {
                    CheckName::Ears => config.ears = true,
                    CheckName::Terms => config.terms = true,
                    CheckName::Contradictions => config.contradictions = true,
                }
            }
        }

        config.report_patterns = self.patterns;
        config.suggest_terms = !self.no_suggest;

        if let Some(max_var) = self.max_variables {
            if max_var == 0 {
                return Err(Failure::new("--max-variables must be greater than zero"));
            }
            config.max_variables = max_var;
        }

        if let Some(max_nodes) = self.max_nodes {
            if max_nodes == 0 {
                return Err(Failure::new("--max-nodes must be greater than zero"));
            }
            config.max_nodes = max_nodes;
        }

        for entry in &self.severities {
            let (code, level_str) = entry.split_once('=').ok_or_else(|| {
                Failure::new(format!(
                    "invalid severity override `{entry}`, expected CODE=LEVEL"
                ))
            })?;

            if !CHECK_CODES.contains(&code) {
                return Err(Failure::new(format!("unknown check code `{code}`")));
            }

            let severity = parse_severity(level_str)
                .ok_or_else(|| Failure::new(format!("unknown severity `{level_str}`")))?;

            config.severity_overrides.insert(code.to_string(), severity);
        }

        Ok(config)
    }
}

/// Arguments for the `status` command.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ViewArgs {
    /// Name of the spec to inspect
    pub spec: String,
}

/// Arguments for `graph` and `tasks` commands.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct GraphArgs {
    /// Name of the spec to inspect
    pub spec: String,

    /// Render as text table instead of Mermaid diagram
    #[arg(long)]
    pub table: bool,
}

/// Vocabulary subcommands.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum VocabCommand {
    /// Validate a vocabulary TOML file
    Validate(ValidateArgs),
    /// Import terms from an external vocabulary file into TOML
    Import(ImportArgs),
}

/// Arguments for `vocab validate`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ValidateArgs {
    /// Explicit vocabulary file to validate (defaults to .kiro/vocabulary.toml under root)
    pub file: Option<PathBuf>,

    /// Report format
    #[arg(long, value_enum, default_value = "text")]
    pub format: ReportFormat,

    /// Severity threshold triggering exit code 1
    #[arg(long, value_enum, default_value = "error")]
    pub fail_on: FailOn,
}

/// Input formats supported by `vocab import`.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputFormat {
    /// NVIDIA Auto Ontology terms JSON export (page object or array)
    #[value(name = "auto-ontology-json")]
    AutoOntologyJson,
    /// W3C SKOS Turtle file
    #[value(name = "skos-turtle")]
    SkosTurtle,
}

/// Default term kind for imported terms.
#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum KindArg {
    #[default]
    Entity,
    Bool,
    Enum,
    Int,
}

/// Arguments for `vocab import`.
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct ImportArgs {
    /// One or more input vocabulary files to import
    #[arg(required = true)]
    pub files: Vec<PathBuf>,

    /// Format of the input files
    #[arg(long, value_enum)]
    pub format: InputFormat,

    /// Topic identifier assigned to imported terms
    #[arg(long)]
    pub topic: String,

    /// Term kind assigned when not specified by input
    #[arg(long, value_enum, default_value = "entity")]
    pub kind: KindArg,

    /// Destination file path (defaults to .kiro/vocabulary.toml, '-' for stdout)
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Merge into existing output file instead of refusing overwrite
    #[arg(long)]
    pub merge: bool,
}

impl ImportArgs {
    /// Validates that `--topic` satisfies the vocabulary id rule.
    pub fn validate_topic(&self) -> Result<(), Failure> {
        if !kiro_core::vocab::is_id(&self.topic) {
            return Err(Failure::new(format!("invalid topic id `{}`", self.topic)));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::finding::Severity;

    #[test]
    fn selection_table() {
        // Neither
        let args_neither = CheckArgs {
            features: vec![],
            all: false,
            format: ReportFormat::Text,
            fail_on: FailOn::Error,
            checks: None,
            patterns: false,
            no_suggest: false,
            max_variables: None,
            max_nodes: None,
            severities: vec![],
        };
        assert!(args_neither.selection().is_err());

        // Both
        let args_both = CheckArgs {
            features: vec!["alpha".to_string()],
            all: true,
            ..args_neither.clone()
        };
        assert!(args_both.selection().is_err());

        // All
        let args_all = CheckArgs {
            all: true,
            ..args_neither.clone()
        };
        assert_eq!(args_all.selection().unwrap(), Selection::All);

        // Features with duplicates preserved in first-seen order
        let args_features = CheckArgs {
            features: vec![
                "beta".to_string(),
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string(),
            ],
            ..args_neither
        };
        assert_eq!(
            args_features.selection().unwrap(),
            Selection::Features(vec![
                "beta".to_string(),
                "alpha".to_string(),
                "gamma".to_string(),
            ])
        );
    }

    #[test]
    fn config_translation_defaults_and_overrides() {
        let base_args = CheckArgs {
            features: vec![],
            all: true,
            format: ReportFormat::Text,
            fail_on: FailOn::Error,
            checks: None,
            patterns: false,
            no_suggest: false,
            max_variables: None,
            max_nodes: None,
            severities: vec![],
        };

        // All defaults
        let cfg = base_args.config().unwrap();
        assert!(cfg.ears);
        assert!(cfg.terms);
        assert!(cfg.contradictions);
        assert!(!cfg.report_patterns);
        assert!(cfg.suggest_terms);
        assert_eq!(cfg.max_variables, 512);
        assert_eq!(cfg.max_nodes, 100_000);
        assert!(cfg.severity_overrides.is_empty());

        // Specific checks
        let custom_checks = CheckArgs {
            checks: Some(vec![CheckName::Ears, CheckName::Contradictions]),
            patterns: true,
            no_suggest: true,
            max_variables: Some(128),
            max_nodes: Some(50_000),
            severities: vec![
                "EARS_NO_SHALL=info".to_string(),
                "CONTRADICTION=warning".to_string(),
            ],
            ..base_args.clone()
        };
        let cfg_custom = custom_checks.config().unwrap();
        assert!(cfg_custom.ears);
        assert!(!cfg_custom.terms);
        assert!(cfg_custom.contradictions);
        assert!(cfg_custom.report_patterns);
        assert!(!cfg_custom.suggest_terms);
        assert_eq!(cfg_custom.max_variables, 128);
        assert_eq!(cfg_custom.max_nodes, 50_000);
        assert_eq!(
            cfg_custom.severity_overrides.get("EARS_NO_SHALL"),
            Some(&Severity::Info)
        );
        assert_eq!(
            cfg_custom.severity_overrides.get("CONTRADICTION"),
            Some(&Severity::Warning)
        );

        // Rejection of zero caps
        let zero_vars = CheckArgs {
            max_variables: Some(0),
            ..base_args.clone()
        };
        assert!(zero_vars.config().is_err());

        let zero_nodes = CheckArgs {
            max_nodes: Some(0),
            ..base_args.clone()
        };
        assert!(zero_nodes.config().is_err());

        // Rejection of unknown check code
        let unknown_code = CheckArgs {
            severities: vec!["NOT_A_REAL_CODE=error".to_string()],
            ..base_args.clone()
        };
        let err_code = unknown_code.config().unwrap_err();
        assert!(err_code.0.contains("unknown check code `NOT_A_REAL_CODE`"));

        // Rejection of unknown severity level
        let unknown_sev = CheckArgs {
            severities: vec!["EARS_NO_SHALL=critical".to_string()],
            ..base_args
        };
        let err_sev = unknown_sev.config().unwrap_err();
        assert!(err_sev.0.contains("unknown severity `critical`"));
    }

    #[test]
    fn cli_try_parse_observables() {
        // Help prints to stdout (use_stderr is false)
        let help_err = Cli::try_parse_from(["kiro-cli", "--help"]).unwrap_err();
        assert!(!help_err.use_stderr());

        // Unknown subcommand prints to stderr (use_stderr is true)
        let unknown_err = Cli::try_parse_from(["kiro-cli", "nonexistent"]).unwrap_err();
        assert!(unknown_err.use_stderr());

        // Check with no selection parses but selection fails
        let check_cli = Cli::try_parse_from(["kiro-cli", "check"]).unwrap();
        if let Command::Check(args) = check_cli.command {
            assert!(args.selection().is_err());
        } else {
            panic!("expected Command::Check");
        }

        // Check with --all succeeds
        let all_cli = Cli::try_parse_from(["kiro-cli", "check", "--all"]).unwrap();
        if let Command::Check(args) = all_cli.command {
            assert_eq!(args.selection().unwrap(), Selection::All);
        } else {
            panic!("expected Command::Check");
        }
    }

    #[test]
    fn import_args_topic_validation() {
        let valid = ImportArgs {
            files: vec![PathBuf::from("terms.json")],
            format: InputFormat::AutoOntologyJson,
            topic: "valid-topic".to_string(),
            kind: KindArg::Entity,
            out: None,
            merge: false,
        };
        assert!(valid.validate_topic().is_ok());

        let invalid = ImportArgs {
            topic: "Invalid Topic!".to_string(),
            ..valid
        };
        let err = invalid.validate_topic().unwrap_err();
        assert!(err.0.contains("invalid topic id `Invalid Topic!`"));
    }
}
