//! Strict TOML parser for vocabulary files (design component `VocabParser`).
//!
//! Deserialises `.kiro/vocabulary.toml` into raw types with spans,
//! or produces a structural `VOCAB_INVALID` finding at the offending line.

use std::collections::BTreeMap;

use crate::finding::{Finding, codes};
use crate::vocab::model::is_id;

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawFile {
    pub(crate) schema_version: Option<toml::Spanned<String>>,
    #[serde(default)]
    pub(crate) topics: BTreeMap<String, toml::Spanned<RawTopic>>,
    pub(crate) terms: BTreeMap<String, toml::Spanned<RawTerm>>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawTopic {
    pub(crate) label: Option<String>,
    pub(crate) description: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, serde::Deserialize)]
pub(crate) struct RawTerm {
    pub(crate) topic: String,
    pub(crate) label: String,
    #[serde(default)]
    pub(crate) definition: String,
    pub(crate) kind: Option<String>,
    #[serde(default)]
    pub(crate) values: Vec<String>,
    pub(crate) min: Option<i64>,
    pub(crate) max: Option<i64>,
    #[serde(default)]
    pub(crate) broader: Vec<String>,
    #[serde(default)]
    pub(crate) alt_labels: Vec<String>,
    #[serde(default)]
    pub(crate) deprecated: bool,
    pub(crate) replaced_by: Option<String>,
}

/// Precomputed newline index for O(log N) line lookups.
pub(crate) struct LineIndex {
    newlines: Vec<usize>,
}

impl LineIndex {
    #[must_use]
    pub(crate) fn new(text: &str) -> Self {
        Self {
            newlines: text.match_indices('\n').map(|(i, _)| i).collect(),
        }
    }

    #[must_use]
    pub(crate) fn line_of(&self, byte: usize) -> usize {
        self.newlines.partition_point(|&nl| nl < byte) + 1
    }
}

/// 1-based line containing byte offset `byte` (count of '\n' before it + 1); clamps past the end.
#[allow(dead_code)]
#[must_use]
pub(crate) fn line_of(text: &str, byte: usize) -> usize {
    LineIndex::new(text).line_of(byte)
}

/// Parses the text of a vocabulary file into `RawFile`, or returns a `VOCAB_INVALID` finding.
/// Unknown keys are accepted silently. A leading UTF-8 BOM is stripped.
#[allow(dead_code)]
pub(crate) fn parse_raw(text: &str, file: &str) -> Result<RawFile, Finding> {
    let clean_text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let raw: RawFile = match toml::from_str(clean_text) {
        Ok(r) => r,
        Err(err) => {
            let line = err.span().map_or(1, |span| line_of(clean_text, span.start));
            return Err(Finding::new(codes::VOCAB_INVALID, file, err.message()).at(line));
        }
    };

    let mut invalid_ids = Vec::new();
    for (id, spanned) in &raw.topics {
        if !is_id(id) {
            invalid_ids.push((spanned.span().start, id.clone()));
        }
    }
    for (id, spanned) in &raw.terms {
        if !is_id(id) {
            invalid_ids.push((spanned.span().start, id.clone()));
        }
    }
    invalid_ids.sort_by_key(|(start, _)| *start);
    if let Some((start, id)) = invalid_ids.first() {
        let line = line_of(clean_text, *start);
        return Err(Finding::new(
            codes::VOCAB_INVALID,
            file,
            format!("invalid id `{id}`: use [a-z0-9][a-z0-9-]*"),
        )
        .at(line));
    }

    Ok(raw)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn line_of_clamping_and_counting() {
        let text = "line 1\nline 2\nline 3";
        assert_eq!(line_of(text, 0), 1);
        assert_eq!(line_of(text, 6), 1); // at the '\n'
        assert_eq!(line_of(text, 7), 2); // 'l' in line 2
        assert_eq!(line_of(text, 14), 3); // 'l' in line 3
        assert_eq!(line_of(text, 9999), 3); // clamps to end of text
    }

    #[test]
    fn parse_valid_with_bom_and_crlf() {
        let content = "\u{feff}schema_version = \"2\"\r\n\r\n[topics.sdd]\r\nlabel = \"SDD\"\r\n\r\n[terms.task]\r\ntopic = \"sdd\"\r\nlabel = \"Task\"\r\ndefinition = \"A task.\"\r\n";
        let res = parse_raw(content, ".kiro/vocabulary.toml");
        assert!(res.is_ok());
        let raw = res.unwrap();
        assert_eq!(raw.schema_version.unwrap().get_ref(), "2");
        assert_eq!(raw.topics.len(), 1);
        assert_eq!(raw.terms.len(), 1);
    }

    #[test]
    fn parse_syntax_error_finding_line() {
        let content = "schema_version = \"2\"\n\n[terms.task\n";
        let res = parse_raw(content, ".kiro/vocabulary.toml");
        assert!(res.is_err());
        let f = res.unwrap_err();
        assert_eq!(f.code, codes::VOCAB_INVALID);
        assert_eq!(f.line, Some(3));
    }

    #[test]
    fn parse_wrong_typed_field_finding_line() {
        let content = "schema_version = \"2\"\n\n[terms.task]\ntopic = \"sdd\"\nlabel = 3\ndefinition = \"A task.\"\n";
        let res = parse_raw(content, ".kiro/vocabulary.toml");
        assert!(res.is_err());
        let f = res.unwrap_err();
        assert_eq!(f.code, codes::VOCAB_INVALID);
        assert_eq!(f.line, Some(5)); // label = 3 is at line 5
    }

    #[test]
    fn parse_missing_terms_table_finding_line() {
        let content = "schema_version = \"2\"\n\n[topics.sdd]\nlabel = \"SDD\"\n";
        let res = parse_raw(content, ".kiro/vocabulary.toml");
        assert!(res.is_err());
        let f = res.unwrap_err();
        assert_eq!(f.code, codes::VOCAB_INVALID);
        assert!(f.message.contains("terms"));
    }

    #[test]
    fn parse_invalid_id_finding_line() {
        let content = "schema_version = \"2\"\n\n[topics.sdd]\nlabel = \"SDD\"\n\n[terms.Bad_Id]\ntopic = \"sdd\"\nlabel = \"Bad\"\ndefinition = \"bad\"\n";
        let res = parse_raw(content, ".kiro/vocabulary.toml");
        assert!(res.is_err());
        let f = res.unwrap_err();
        assert_eq!(f.code, codes::VOCAB_INVALID);
        assert_eq!(f.line, Some(6));
        assert!(f.message.contains("invalid id `Bad_Id`"));
    }

    #[test]
    fn parse_arbitrary_garbage_never_panics() {
        let garbage = [
            "",
            "\0\0\0",
            "[[[",
            "=====",
            "{\"json\": true}",
            "\n\n\n\n\n",
        ];
        for g in garbage {
            let _ = parse_raw(g, ".kiro/vocabulary.toml");
        }
    }
}
