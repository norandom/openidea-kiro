//! Settings of the rules that count across files (clone-constraints 3.1,
//! code-locality-data 1.3). Expected values were decided by hand.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_core::Severity;
use kiro_core::code::rules::RuleSettings;
use kiro_core::code::{CodeConfig, parse_config};

fn rules(text: &str) -> RuleSettings {
    parse_config(Some(text)).unwrap().rules().clone()
}

fn errors(text: &str) -> Vec<(String, String)> {
    parse_config(Some(text))
        .unwrap_err()
        .into_iter()
        .map(|error| (error.key, error.message))
        .collect()
}

#[test]
fn without_a_file_the_built_in_settings_apply() {
    let built_in = CodeConfig::default();
    let rules = built_in.rules();
    assert_eq!(rules, &RuleSettings::default());
    assert_eq!(
        (rules.duplicate_block.severity, rules.min_lines),
        (Severity::Warning, 12)
    );
    assert_eq!(rules.similar_family.severity, Severity::Warning);
    assert!((rules.threshold - 0.9).abs() < 1e-12);
    assert_eq!(rules.min_statements, 6);
    assert_eq!(rules.scattered_definition.severity, Severity::Warning);
    assert_eq!(rules.definition_max_files, 3);
    assert_eq!(rules.scattered_state.severity, Severity::Info);
    assert_eq!((rules.state_max_files, rules.min_states), (3, 2));
    assert_eq!(rules.positional_access.severity, Severity::Warning);
    assert!(rules.duplicate_block.enabled && rules.scattered_state.enabled);
    assert_eq!(
        built_in
            .effective_text()
            .lines()
            .filter(|line| line.starts_with("rule "))
            .collect::<Vec<_>>(),
        [
            "rule duplicate_block: warning, min_lines 12",
            "rule similar_family: warning, threshold 0.9, min_statements 6",
            "rule scattered_definition: warning, max_files 3",
            "rule scattered_state: info, max_files 3, min_states 2",
            "rule positional_access: warning",
        ]
    );
}

#[test]
fn a_stricter_setting_needs_no_reason() {
    let rules = rules(
        "[rules.duplicate_block]\nseverity = \"error\"\nmin_lines = 8\n\n[rules.similar_family]\nthreshold = 0.85\nmin_statements = 4\n\n[rules.scattered_definition]\nmax_files = 2\n\n[rules.scattered_state]\nseverity = \"warning\"\nmax_files = 2\nmin_states = 2\n",
    );
    assert_eq!(
        (rules.duplicate_block.severity, rules.min_lines),
        (Severity::Error, 8)
    );
    assert!((rules.threshold - 0.85).abs() < 1e-12);
    assert_eq!(rules.min_statements, 4);
    assert_eq!(rules.definition_max_files, 2);
    assert_eq!(rules.scattered_state.severity, Severity::Warning);
    assert_eq!(rules.state_max_files, 2);
    // The other rule keeps its own number of files.
    assert_eq!(RuleSettings::default().state_max_files, 3);
}

#[test]
fn a_setting_that_reports_less_needs_a_reason() {
    for (text, key) in [
        (
            "[rules.duplicate_block]\nseverity = \"off\"\n",
            "rules.duplicate_block",
        ),
        (
            "[rules.duplicate_block]\nseverity = \"info\"\n",
            "rules.duplicate_block",
        ),
        (
            "[rules.duplicate_block]\nmin_lines = 20\n",
            "rules.duplicate_block",
        ),
        (
            "[rules.similar_family]\nthreshold = 0.95\n",
            "rules.similar_family",
        ),
        (
            "[rules.similar_family]\nmin_statements = 10\n",
            "rules.similar_family",
        ),
        (
            "[rules.similar_family]\nignored_names = [\"visit\"]\n",
            "rules.similar_family",
        ),
        (
            "[rules.scattered_definition]\nmax_files = 5\n",
            "rules.scattered_definition",
        ),
        (
            "[rules.scattered_state]\nseverity = \"off\"\n",
            "rules.scattered_state",
        ),
        (
            "[rules.scattered_state]\nmin_states = 3\n",
            "rules.scattered_state",
        ),
        (
            "[rules.positional_access]\nseverity = \"off\"\n",
            "rules.positional_access",
        ),
        // A reason of blanks is no reason.
        (
            "[rules.positional_access]\nseverity = \"off\"\nreason = \"  \"\n",
            "rules.positional_access",
        ),
    ] {
        assert_eq!(
            errors(text),
            [(
                key.to_string(),
                "reports less than the built-in rule and needs a reason".to_string()
            )],
            "{text}"
        );
    }
}

#[test]
fn with_a_reason_a_rule_is_relaxed_or_off_and_the_reason_is_shown() {
    let config = parse_config(Some(
        "[rules.similar_family]\nseverity = \"off\"\nreason = \"not adopted yet\"\n\n[rules.scattered_definition]\nmax_files = 5\nignored_names = [\"VERSION\", \"NAME\"]\nreason = \"every package states its own\"\n",
    ))
    .unwrap();
    let rules = config.rules();
    assert!(!rules.similar_family.enabled);
    assert_eq!(rules.definition_max_files, 5);
    assert_eq!(rules.ignored_bindings, ["VERSION", "NAME"]);
    let text = config.effective_text();
    assert!(
        text.contains(
            "rule similar_family: off, threshold 0.9, min_statements 6 reason: not adopted yet\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("rule scattered_definition: warning, max_files 5, ignored names VERSION NAME reason: every package states its own\n"),
        "{text}"
    );
    // The digest follows the rule settings.
    assert_ne!(config.digest(), CodeConfig::default().digest());
}

#[test]
fn wrong_values_and_keys_are_named() {
    assert_eq!(
        errors("[rules.duplicate_block]\nseverity = \"loud\"\n"),
        [(
            "rules.duplicate_block.severity".to_string(),
            "unknown severity; expected error, warning, info or off".to_string()
        )]
    );
    assert_eq!(
        errors("[rules.duplicate_block]\nmin_lines = 0\n"),
        [(
            "rules.duplicate_block.min_lines".to_string(),
            "must be at least 1".to_string()
        )]
    );
    assert_eq!(
        errors("[rules.similar_family]\nthreshold = 1.5\n"),
        [(
            "rules.similar_family.threshold".to_string(),
            "must be greater than 0 and at most 1".to_string()
        )]
    );
    // A key of another rule.
    assert_eq!(
        errors("[rules.positional_access]\nmax_files = 2\n"),
        [(
            "rules.positional_access.max_files".to_string(),
            "this rule has no such setting".to_string()
        )]
    );
    // An unknown rule and an unknown key are refused by the file format.
    assert!(
        errors("[rules.magic]\nseverity = \"off\"\n")[0]
            .1
            .contains("magic")
    );
    assert!(
        errors("[rules.duplicate_block]\nlines = 3\n")[0]
            .1
            .contains("lines")
    );
}

#[test]
fn a_level_sets_the_severity_of_its_findings_or_drops_them() {
    use kiro_core::code::locality::{Binding, LocalityFacts, definition_findings};
    let bindings = ["a", "b", "c", "d"]
        .iter()
        .map(|path| Binding {
            path: (*path).to_string(),
            line: 1,
            name: "K".to_string(),
            initializer: "1".to_string(),
        })
        .collect();
    let facts = LocalityFacts {
        bindings,
        comparisons: Vec::new(),
        files: 4,
    };
    let found = definition_findings(&facts, &|_| true, 3);
    assert_eq!(found.len(), 4);
    let strict = rules("[rules.scattered_definition]\nseverity = \"error\"\n");
    let raised = strict.scattered_definition.apply(found.clone());
    assert!(
        raised
            .iter()
            .all(|entry| entry.finding.severity == Severity::Error)
    );
    let off = rules("[rules.scattered_definition]\nseverity = \"off\"\nreason = \"later\"\n");
    assert!(off.scattered_definition.apply(found).is_empty());
}
