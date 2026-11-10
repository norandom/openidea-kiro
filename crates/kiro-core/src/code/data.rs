//! Positional data access (code-locality-data 3.1, 3.3).
//!
//! A column, a field or an axis of tabular data that is addressed by a
//! number says nothing about what it holds: `frame.iloc[:, 3]` is correct
//! only as long as nobody reorders the columns, and a reader cannot tell
//! what column 3 means. The rule reports such accesses and asks for a
//! name.
//!
//! The patterns are textual. The native crate recognizes a fixed set of
//! shapes per language (see its `data_access` module); nothing is known
//! about the type of the receiver. This module is pure: it words the
//! findings.

use super::report::{CodeFinding, Tag};
use crate::finding::{Finding, Severity, codes};

/// Longest expression text quoted in a finding.
const MAX_EXPRESSION_CHARS: usize = 60;

/// One access by position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PositionalAccess {
    pub path: String,
    pub line: u32,
    /// The access as written, whitespace collapsed.
    pub expression: String,
    /// The integer literal or literals in the column position.
    pub position: String,
    /// What the shape is: `a dataframe column`, `an array column` or
    /// `a row record field`.
    pub kind: &'static str,
}

/// The data-access facts of a project.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataFacts {
    pub accesses: Vec<PositionalAccess>,
    /// Files searched: those of a language with shipped patterns.
    pub files: usize,
}

fn quoted(expression: &str) -> String {
    if expression.chars().count() <= MAX_EXPRESSION_CHARS {
        return expression.to_string();
    }
    let head: String = expression.chars().take(MAX_EXPRESSION_CHARS).collect();
    format!("{head}...")
}

/// One finding per positional access in a selected file.
#[must_use]
pub fn positional_findings(facts: &DataFacts, selected: &dyn Fn(&str) -> bool) -> Vec<CodeFinding> {
    facts
        .accesses
        .iter()
        .filter(|access| selected(&access.path))
        .map(|access| {
            let mut finding = Finding::new(
                codes::CODE_POSITIONAL_ACCESS,
                access.path.clone(),
                format!(
                    "`{}` addresses {} by position {}",
                    quoted(&access.expression),
                    access.kind,
                    access.position
                ),
            )
            .severity(Severity::Warning);
            finding.line = usize::try_from(access.line).ok();
            CodeFinding::across_files(
                finding,
                Tag::Name,
                "select the column by its role name, or give the row a typed record".to_string(),
            )
        })
        .collect()
}
