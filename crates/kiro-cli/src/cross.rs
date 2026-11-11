//! The rules that count across files: duplicate blocks, similar-routine
//! families, scattered definitions and state, and positional data access.
//!
//! They search the whole project: a copy or a repeated definition counts
//! wherever the original stands. Every file is parsed once for all of
//! them. Each rule follows its setting from `[rules]` in the configuration
//! and says in the coverage line what it covered, or that it is off.

use kiro_code::scan::{ProjectScan, ScanSettings, scan_with};
use kiro_core::code::CodeFinding;
use kiro_core::code::clones::{duplicate_blocks, duplicate_findings};
use kiro_core::code::data::positional_findings;
use kiro_core::code::families::{IGNORED_NAMES, families, family_findings};
use kiro_core::code::locality::{LocalityFacts, definition_findings, state_findings};
use kiro_core::code::rules::RuleSettings;

use crate::exit::Failure;
use crate::fs::ProjectRoot;

/// What the rules found, and what they covered.
pub struct CrossFile {
    pub findings: Vec<CodeFinding>,
    /// One line per rule family, for the coverage line.
    pub notes: Vec<String>,
}

fn measurable(path: &str) -> bool {
    kiro_code::profile_for(path).is_some()
}

struct Judge<'a> {
    scan: &'a ProjectScan,
    rules: &'a RuleSettings,
    selected: &'a dyn Fn(&str) -> bool,
    findings: Vec<CodeFinding>,
}

impl Judge<'_> {
    fn duplicate_blocks(&mut self) -> String {
        let level = self.rules.duplicate_block;
        if !level.enabled {
            return "duplicate blocks: off (configuration)".to_string();
        }
        let min_lines = self.rules.min_lines;
        let (blocks, searched) = duplicate_blocks(&self.scan.lines, min_lines);
        self.findings
            .extend(level.apply(duplicate_findings(&blocks, self.selected)));
        format!(
            "duplicate blocks: {} files searched, {} blocks of at least {min_lines} lines, {} frequent windows skipped",
            searched.files,
            blocks.len(),
            searched.skipped_windows
        )
    }

    /// An incomplete comparison is said so: absent families are then no
    /// proof that there are none.
    fn similar_routines(&mut self) -> String {
        let level = self.rules.similar_family;
        if !level.enabled {
            return "similar routines: off (configuration)".to_string();
        }
        let facts = &self.scan.families;
        let mut ignored = IGNORED_NAMES.to_vec();
        ignored.extend(self.rules.ignored_routines.iter().map(String::as_str));
        let found = families(facts, &ignored);
        self.findings
            .extend(level.apply(family_findings(facts, &found, self.selected)));
        let coverage = &facts.coverage;
        let incomplete = if coverage.complete {
            ""
        } else {
            "; the pair budget ran out, families may be missing"
        };
        format!(
            "similar routines: {} of {} routines compared, {} pairs scored, {} refused before scoring, {} families{incomplete}",
            coverage.eligible,
            coverage.routines,
            coverage.scored_pairs,
            coverage.pruned_pairs,
            found.len()
        )
    }

    /// The locality facts without the bindings the configuration ignores.
    fn locality_facts(&self) -> LocalityFacts {
        let mut facts = self.scan.locality.clone();
        let ignored = &self.rules.ignored_bindings;
        facts
            .bindings
            .retain(|binding| !ignored.contains(&binding.name));
        facts
    }

    fn locality(&mut self) -> String {
        let (definitions, states) = (self.rules.scattered_definition, self.rules.scattered_state);
        let facts = self.locality_facts();
        if definitions.enabled {
            let max_files = self.rules.definition_max_files;
            let found = definition_findings(&facts, self.selected, max_files);
            self.findings.extend(definitions.apply(found));
        }
        if states.enabled {
            let (max_files, min_states) = (self.rules.state_max_files, self.rules.min_states);
            let found = state_findings(&facts, self.selected, max_files, min_states);
            self.findings.extend(states.apply(found));
        }
        let off = match (definitions.enabled, states.enabled) {
            (true, true) => "",
            (false, true) => "; scattered definitions: off (configuration)",
            (true, false) => "; scattered state: off (configuration)",
            (false, false) => "; scattered definitions and state: off (configuration)",
        };
        format!(
            "locality: {} files searched, {} module-level definitions, {} literal comparisons{off}",
            facts.files,
            facts.bindings.len(),
            facts.comparisons.len()
        )
    }

    fn data_access(&mut self) -> String {
        let level = self.rules.positional_access;
        if !level.enabled {
            return "data access: off (configuration)".to_string();
        }
        let data = &self.scan.data;
        self.findings
            .extend(level.apply(positional_findings(data, self.selected)));
        format!(
            "data access: {} files searched, {} positional accesses; column names were not grounded (no data topic is configured)",
            data.files,
            data.accesses.len()
        )
    }
}

/// A local check: no rule that reads the whole project ran, and the
/// coverage line says so.
pub fn not_run() -> CrossFile {
    CrossFile {
        findings: Vec::new(),
        notes: vec![
            "rules across files: not run (--local); run the check without --local before a push or in CI"
                .to_string(),
        ],
    }
}

/// Judges the rules that count across files for the selected files.
pub fn judge(
    root: &ProjectRoot,
    selected: &[String],
    rules: &RuleSettings,
) -> Result<CrossFile, Failure> {
    let project = kiro_code::read_bundle(root.path(), &measurable)
        .map_err(|error| Failure::new(error.to_string()))?;
    let scan = scan_with(
        &project,
        &ScanSettings {
            min_statements: rules.min_statements,
            threshold: rules.threshold,
        },
    );
    let is_selected = |path: &str| selected.iter().any(|selected_path| selected_path == path);
    let mut judge = Judge {
        scan: &scan,
        rules,
        selected: &is_selected,
        findings: Vec::new(),
    };
    let duplicates = judge.duplicate_blocks();
    let similar = judge.similar_routines();
    let locality = judge.locality();
    let data = judge.data_access();
    Ok(CrossFile {
        findings: judge.findings,
        notes: vec![duplicates, locality, similar, data],
    })
}
