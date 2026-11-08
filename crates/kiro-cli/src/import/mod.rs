//! Importer model, id derivation, deduplication, TOML rendering,
//! append-only merge, output validation, and reader dispatch.

pub mod auto_ontology;
pub mod skos;

use std::collections::{BTreeMap, HashMap};

use kiro_core::finding::{Finding, codes};
use kiro_core::vocab::model::Kind;
use kiro_core::vocab::scope::Lookup;
use kiro_core::vocab::{Vocabulary, parse as vocab_parse};

use crate::cli::InputFormat;

/// One operator-provided input source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    pub display: String,
    pub text: String,
}

/// Fallback topic and kind when the source provides neither.
#[derive(Debug, Clone)]
pub struct Defaults<'a> {
    pub topic: &'a str,
    pub kind: &'a Kind,
}

/// A term extracted from an external vocabulary source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTerm {
    pub id: String,
    pub topic: String,
    pub label: String,
    pub definition: String,
    pub kind: Kind,
    pub broader: Vec<String>,
    pub alt_labels: Vec<String>,
    pub deprecated: bool,
    pub replaced_by: Option<String>,
    pub comments: Vec<String>,
    pub source: String,
}

/// The result of reading one or more input sources.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Imported {
    pub terms: Vec<ImportedTerm>,
    pub topics: BTreeMap<String, Option<String>>,
    pub findings: Vec<Finding>,
}

/// Header metadata for generated or merged TOML output.
#[derive(Debug, Clone)]
pub struct Header<'a> {
    pub format: InputFormat,
    pub inputs: &'a [String],
    pub topic: &'a str,
}

/// Dispatches reading to the reader implementation for `format`.
pub fn read(
    format: InputFormat,
    inputs: &[Input],
    defaults: &Defaults<'_>,
) -> Result<Imported, Finding> {
    match format {
        InputFormat::AutoOntologyJson => auto_ontology::read(inputs, defaults),
        InputFormat::SkosTurtle => skos::read(inputs, defaults),
    }
}

/// Derives a vocabulary term identifier from a source name.
///
/// Unicode-aware:
/// 1. Inserts `-` before uppercase letters preceded by lowercase or digit,
///    or preceded by uppercase and followed by lowercase.
/// 2. Lowercases every character.
/// 3. Converts every character outside `[a-z0-9]` to `-`.
/// 4. Collapses runs of `-`.
/// 5. Trims leading and trailing `-`.
#[must_use]
pub fn derive_id(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut with_splits = String::with_capacity(name.len() + 8);

    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = chars.get(i.saturating_sub(1)).copied();
            let next = chars.get(i.saturating_add(1)).copied();

            let is_camel = prev.is_some_and(|p| p.is_lowercase() || p.is_ascii_digit());
            let is_acronym_end =
                prev.is_some_and(|p| p.is_uppercase()) && next.is_some_and(|n| n.is_lowercase());

            if is_camel || is_acronym_end {
                with_splits.push('-');
            }
        }
        with_splits.push(c);
    }

    let mut result = String::with_capacity(with_splits.len());
    let mut last_was_hyphen = false;

    for c in with_splits.chars() {
        for lc in c.to_lowercase() {
            if lc.is_ascii_lowercase() || lc.is_ascii_digit() {
                result.push(lc);
                last_was_hyphen = false;
            } else if !last_was_hyphen {
                result.push('-');
                last_was_hyphen = true;
            }
        }
    }

    result.trim_matches('-').to_string()
}

/// Keeps the first term per id in input order; reports later duplicates.
pub fn dedupe(terms: Vec<ImportedTerm>, findings: &mut Vec<Finding>) -> Vec<ImportedTerm> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut result = Vec::with_capacity(terms.len());

    for term in terms {
        if let Some(first_source) = seen.get(&term.id) {
            let msg = format!(
                "`{}` derived twice: {} and {}; first wins",
                term.id, first_source, term.source
            );
            let file = match term.source.rsplit_once(':') {
                Some((f, ord)) if ord.parse::<usize>().is_ok() => f,
                _ => term.source.as_str(),
            };
            findings.push(Finding::new(codes::IMPORT_DUPLICATE_ID, file, msg));
        } else {
            seen.insert(term.id.clone(), term.source.clone());
            result.push(term);
        }
    }

    result
}

/// Formats a string as a TOML basic string literal with standard escaping.
#[must_use]
pub fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let code = c as u32;
                out.push_str(&format!("\\u{code:04x}"));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Renders a fresh vocabulary TOML file from imported terms and topics.
#[must_use]
pub fn render(imported: &Imported, terms: &[ImportedTerm], header: &Header<'_>) -> String {
    let mut out = String::new();
    let format_str = match header.format {
        InputFormat::AutoOntologyJson => "auto-ontology-json",
        InputFormat::SkosTurtle => "skos-turtle",
    };
    let inputs_str = header.inputs.join(", ");
    out.push_str(&format!(
        "# Generated by kiro-cli vocab import --format {} --topic {}\n",
        format_str, header.topic
    ));
    out.push_str(&format!("# Inputs: {}\n", inputs_str));
    out.push_str(
        "# Topics, hierarchy (broader) and kinds are curated by maintainers; see docs/vocabulary.md.\n",
    );
    out.push_str("schema_version = \"2\"\n");

    // Gather all topics occurring in terms or imported.topics, sorted by id
    let mut topics: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    for (t, l) in &imported.topics {
        topics.insert(t.as_str(), l.as_deref());
    }
    for term in terms {
        topics.entry(term.topic.as_str()).or_insert(None);
    }

    for (topic_id, label) in topics {
        out.push('\n');
        out.push_str(&format!("[topics.{}]\n", topic_id));
        if let Some(lbl) = label {
            out.push_str(&format!("label = {}\n", toml_string(lbl)));
        }
    }

    let mut sorted_terms: Vec<&ImportedTerm> = terms.iter().collect();
    sorted_terms.sort_by_key(|t| &t.id);

    for term in sorted_terms {
        out.push('\n');
        out.push_str(&format!("[terms.{}]\n", term.id));
        for comment in &term.comments {
            out.push_str(&format!("# {}\n", comment));
        }
        out.push_str(&format!("topic = {}\n", toml_string(&term.topic)));
        out.push_str(&format!("label = {}\n", toml_string(&term.label)));
        out.push_str(&format!("kind = {}\n", toml_string(term.kind.name())));
        match &term.kind {
            Kind::Enum { values } => {
                if !values.is_empty() {
                    let vals: Vec<String> = values.iter().map(|v| toml_string(v)).collect();
                    out.push_str(&format!("values = [{}]\n", vals.join(", ")));
                }
            }
            Kind::Int { min, max } => {
                if let Some(m) = min {
                    out.push_str(&format!("min = {}\n", m));
                }
                if let Some(m) = max {
                    out.push_str(&format!("max = {}\n", m));
                }
            }
            _ => {}
        }
        out.push_str(&format!("definition = {}\n", toml_string(&term.definition)));
        if !term.broader.is_empty() {
            let b: Vec<String> = term.broader.iter().map(|s| toml_string(s)).collect();
            out.push_str(&format!("broader = [{}]\n", b.join(", ")));
        }
        if !term.alt_labels.is_empty() {
            let a: Vec<String> = term.alt_labels.iter().map(|s| toml_string(s)).collect();
            out.push_str(&format!("alt_labels = [{}]\n", a.join(", ")));
        }
        if term.deprecated {
            out.push_str("deprecated = true\n");
        }
        if let Some(replaced) = &term.replaced_by {
            out.push_str(&format!("replaced_by = {}\n", toml_string(replaced)));
        }
    }

    out
}

/// Appends non-conflicting terms and missing topic tables to an existing vocabulary.
pub fn merge(
    existing_text: &str,
    existing: &Vocabulary,
    imported: &Imported,
    terms: &[ImportedTerm],
    header: &Header<'_>,
    findings: &mut Vec<Finding>,
    out_display: &str,
) -> String {
    let all_topics: Vec<String> = existing
        .topics()
        .into_iter()
        .map(|t| t.id.clone())
        .collect();
    let mut non_conflicting_terms = Vec::new();

    for term in terms {
        let mut has_conflict = false;

        // 1. Check if term id already exists in existing vocabulary
        if matches!(
            existing.preferred_label(&term.id, &all_topics),
            Lookup::Found { .. }
        ) {
            findings.push(Finding::new(
                codes::IMPORT_MERGE_CONFLICT,
                out_display,
                format!(
                    "`{}` already exists in {}; existing kept",
                    term.id, out_display
                ),
            ));
            has_conflict = true;
        }

        // 2. Check if primary label collides with any term in the same topic
        let topic_scope = [term.topic.clone()];
        let label_res = existing.resolve(&term.label, &topic_scope);
        if let Lookup::Found { term: hit_term, .. } = label_res {
            if hit_term.id != term.id {
                findings.push(Finding::new(
                    codes::IMPORT_MERGE_CONFLICT,
                    out_display,
                    format!(
                        "label `{}` of `{}` already names `{}` in topic {}; not added",
                        term.label, term.id, hit_term.id, term.topic
                    ),
                ));
                has_conflict = true;
            }
        } else if let Lookup::Ambiguous { hits } = label_res {
            let hit_id = hits.first().map_or("existing", |h| &h.id);
            if hit_id != term.id {
                findings.push(Finding::new(
                    codes::IMPORT_MERGE_CONFLICT,
                    out_display,
                    format!(
                        "label `{}` of `{}` already names `{}` in topic {}; not added",
                        term.label, term.id, hit_id, term.topic
                    ),
                ));
                has_conflict = true;
            }
        }

        // 3. Check if any alternative label collides with any term in the same topic
        for alt in &term.alt_labels {
            let alt_res = existing.resolve(alt, &topic_scope);
            if let Lookup::Found { term: hit_term, .. } = alt_res {
                if hit_term.id != term.id {
                    findings.push(Finding::new(
                        codes::IMPORT_MERGE_CONFLICT,
                        out_display,
                        format!(
                            "label `{}` of `{}` already names `{}` in topic {}; not added",
                            alt, term.id, hit_term.id, term.topic
                        ),
                    ));
                    has_conflict = true;
                    break;
                }
            } else if let Lookup::Ambiguous { hits } = alt_res {
                let hit_id = hits.first().map_or("existing", |h| &h.id);
                if hit_id != term.id {
                    findings.push(Finding::new(
                        codes::IMPORT_MERGE_CONFLICT,
                        out_display,
                        format!(
                            "label `{}` of `{}` already names `{}` in topic {}; not added",
                            alt, term.id, hit_id, term.topic
                        ),
                    ));
                    has_conflict = true;
                    break;
                }
            }
        }

        if !has_conflict {
            non_conflicting_terms.push(term);
        }
    }

    // Determine topics that lack explicit [topics.<id>] declarations in existing vocabulary
    let mut missing_topics: BTreeMap<&str, Option<&str>> = BTreeMap::new();
    for (t, l) in &imported.topics {
        if existing.topic(t).is_none_or(|topic| topic.line.is_none()) {
            missing_topics.insert(t.as_str(), l.as_deref());
        }
    }
    for term in &non_conflicting_terms {
        if existing
            .topic(&term.topic)
            .is_none_or(|topic| topic.line.is_none())
        {
            missing_topics.entry(term.topic.as_str()).or_insert(None);
        }
    }

    if non_conflicting_terms.is_empty() && missing_topics.is_empty() {
        return existing_text.to_string();
    }

    let mut out = existing_text.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');

    let format_str = match header.format {
        InputFormat::AutoOntologyJson => "auto-ontology-json",
        InputFormat::SkosTurtle => "skos-turtle",
    };
    let inputs_str = header.inputs.join(", ");
    out.push_str(&format!(
        "# Merged by kiro-cli vocab import --format {} --topic {} (inputs: {})\n",
        format_str, header.topic, inputs_str
    ));

    for (topic_id, label) in missing_topics {
        out.push('\n');
        out.push_str(&format!("[topics.{}]\n", topic_id));
        if let Some(lbl) = label {
            out.push_str(&format!("label = {}\n", toml_string(lbl)));
        }
    }

    let mut sorted_terms = non_conflicting_terms;
    sorted_terms.sort_by_key(|t| &t.id);

    for term in sorted_terms {
        out.push('\n');
        out.push_str(&format!("[terms.{}]\n", term.id));
        for comment in &term.comments {
            out.push_str(&format!("# {}\n", comment));
        }
        out.push_str(&format!("topic = {}\n", toml_string(&term.topic)));
        out.push_str(&format!("label = {}\n", toml_string(&term.label)));
        out.push_str(&format!("kind = {}\n", toml_string(term.kind.name())));
        match &term.kind {
            Kind::Enum { values } => {
                if !values.is_empty() {
                    let vals: Vec<String> = values.iter().map(|v| toml_string(v)).collect();
                    out.push_str(&format!("values = [{}]\n", vals.join(", ")));
                }
            }
            Kind::Int { min, max } => {
                if let Some(m) = min {
                    out.push_str(&format!("min = {}\n", m));
                }
                if let Some(m) = max {
                    out.push_str(&format!("max = {}\n", m));
                }
            }
            _ => {}
        }
        out.push_str(&format!("definition = {}\n", toml_string(&term.definition)));
        if !term.broader.is_empty() {
            let b: Vec<String> = term.broader.iter().map(|s| toml_string(s)).collect();
            out.push_str(&format!("broader = [{}]\n", b.join(", ")));
        }
        if !term.alt_labels.is_empty() {
            let a: Vec<String> = term.alt_labels.iter().map(|s| toml_string(s)).collect();
            out.push_str(&format!("alt_labels = [{}]\n", a.join(", ")));
        }
        if term.deprecated {
            out.push_str("deprecated = true\n");
        }
        if let Some(replaced) = &term.replaced_by {
            out.push_str(&format!("replaced_by = {}\n", toml_string(replaced)));
        }
    }

    out
}

/// Validates the output text through the engine's vocabulary parser.
#[must_use]
pub fn validate_output(text: &str, out_display: &str) -> Vec<Finding> {
    vocab_parse(text, out_display).findings
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::vocab::model::is_id;

    #[test]
    fn derive_id_table() {
        assert_eq!(derive_id("InsiderThreat"), "insider-threat");
        assert_eq!(derive_id("HTTPSProxy"), "https-proxy");
        assert_eq!(derive_id("Level2Cache"), "level2-cache");
        assert_eq!(
            derive_id("Customer Lifetime Value"),
            "customer-lifetime-value"
        );
        assert_eq!(derive_id("order_id"), "order-id");
        assert_eq!(derive_id("Ärger"), "rger");
        assert_eq!(derive_id("   "), "");
        assert_eq!(derive_id("---"), "");
        assert_eq!(derive_id("Order"), "order");

        // Verify is_id conformance
        assert!(is_id(&derive_id("InsiderThreat")));
        assert!(is_id(&derive_id("HTTPSProxy")));
        assert!(is_id(&derive_id("Level2Cache")));
        assert!(is_id(&derive_id("Customer Lifetime Value")));
        assert!(is_id(&derive_id("order_id")));
        assert!(is_id(&derive_id("Ärger")));
    }

    #[test]
    fn toml_string_escaping() {
        assert_eq!(toml_string("hello"), "\"hello\"");
        assert_eq!(toml_string("hello \"world\""), "\"hello \\\"world\\\"\"");
        assert_eq!(toml_string("line 1\nline 2"), "\"line 1\\nline 2\"");
        assert_eq!(toml_string("tab\there"), "\"tab\\there\"");
        assert_eq!(toml_string(""), "\"\"");
    }

    #[test]
    fn dedupe_first_wins() {
        let mut findings = Vec::new();
        let terms = vec![
            ImportedTerm {
                id: "order".to_string(),
                topic: "sdd".to_string(),
                label: "Order".to_string(),
                definition: "First order".to_string(),
                kind: Kind::Entity,
                broader: Vec::new(),
                alt_labels: Vec::new(),
                deprecated: false,
                replaced_by: None,
                comments: Vec::new(),
                source: "file.json:1".to_string(),
            },
            ImportedTerm {
                id: "order".to_string(),
                topic: "sdd".to_string(),
                label: "order".to_string(),
                definition: "Second order".to_string(),
                kind: Kind::Entity,
                broader: Vec::new(),
                alt_labels: Vec::new(),
                deprecated: false,
                replaced_by: None,
                comments: Vec::new(),
                source: "file.json:2".to_string(),
            },
        ];

        let deduped = dedupe(terms, &mut findings);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].definition, "First order");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, codes::IMPORT_DUPLICATE_ID);
        assert!(findings[0].message.contains("derived twice"));
        assert!(findings[0].message.contains("file.json:1"));
        assert!(findings[0].message.contains("file.json:2"));
        assert_eq!(findings[0].file, "file.json");
    }

    #[test]
    fn render_order_and_omission() {
        let imported = Imported {
            terms: Vec::new(),
            topics: BTreeMap::from([("sdd".to_string(), Some("Software Design".to_string()))]),
            findings: Vec::new(),
        };

        let terms = vec![ImportedTerm {
            id: "customer-lifetime-value".to_string(),
            topic: "sdd".to_string(),
            label: "Customer Lifetime Value".to_string(),
            definition: "Total expected revenue.".to_string(),
            kind: Kind::Entity,
            broader: Vec::new(),
            alt_labels: vec!["CLV".to_string()],
            deprecated: false,
            replaced_by: None,
            comments: vec!["auto-ontology: id=123".to_string()],
            source: "src.json:1".to_string(),
        }];

        let header = Header {
            format: InputFormat::AutoOntologyJson,
            inputs: &["terms.json".to_string()],
            topic: "sdd",
        };

        let rendered = render(&imported, &terms, &header);
        assert!(rendered.contains(
            "# Generated by kiro-cli vocab import --format auto-ontology-json --topic sdd\n"
        ));
        assert!(rendered.contains("# Inputs: terms.json\n"));
        assert!(rendered.contains("schema_version = \"2\"\n"));
        assert!(rendered.contains("[topics.sdd]\nlabel = \"Software Design\"\n"));
        assert!(rendered.contains("[terms.customer-lifetime-value]\n# auto-ontology: id=123\ntopic = \"sdd\"\nlabel = \"Customer Lifetime Value\"\nkind = \"entity\"\ndefinition = \"Total expected revenue.\"\nalt_labels = [\"CLV\"]\n"));

        let findings = validate_output(&rendered, ".kiro/vocabulary.toml");
        assert!(
            findings.is_empty(),
            "rendered TOML must validate cleanly: {:?}",
            findings
        );
    }

    #[test]
    fn merge_conflict_and_topic_addition() {
        let existing_text = r#"schema_version = "2"

[topics.sdd]
label = "SDD"

[terms.order]
topic = "sdd"
label = "Order"
definition = "Existing order."
kind = "entity"
alt_labels = ["PO"]
"#;
        let parsed = vocab_parse(existing_text, "existing.toml");
        assert!(parsed.findings.is_empty());
        let existing = parsed.vocabulary;

        let mut findings = Vec::new();
        let imported = Imported {
            terms: Vec::new(),
            topics: BTreeMap::from([
                ("sdd".to_string(), None),
                ("ops".to_string(), Some("Operations".to_string())),
            ]),
            findings: Vec::new(),
        };

        let terms = vec![
            // 1. Collides by id
            ImportedTerm {
                id: "order".to_string(),
                topic: "sdd".to_string(),
                label: "Different Label".to_string(),
                definition: "new order".to_string(),
                kind: Kind::Entity,
                broader: Vec::new(),
                alt_labels: Vec::new(),
                deprecated: false,
                replaced_by: None,
                comments: Vec::new(),
                source: "test.json:1".to_string(),
            },
            // 2. Collides by alt_label matching existing alt_label "PO"
            ImportedTerm {
                id: "purchase-order".to_string(),
                topic: "sdd".to_string(),
                label: "Purchase Order".to_string(),
                definition: "new po".to_string(),
                kind: Kind::Entity,
                broader: Vec::new(),
                alt_labels: vec!["PO".to_string()],
                deprecated: false,
                replaced_by: None,
                comments: Vec::new(),
                source: "test.json:2".to_string(),
            },
            // 3. Clean new term in new topic ops
            ImportedTerm {
                id: "deployment".to_string(),
                topic: "ops".to_string(),
                label: "Deployment".to_string(),
                definition: "Release of code.".to_string(),
                kind: Kind::Entity,
                broader: Vec::new(),
                alt_labels: Vec::new(),
                deprecated: false,
                replaced_by: None,
                comments: Vec::new(),
                source: "test.json:3".to_string(),
            },
        ];

        let header = Header {
            format: InputFormat::AutoOntologyJson,
            inputs: &["input.json".to_string()],
            topic: "sdd",
        };

        let merged = merge(
            existing_text,
            &existing,
            &imported,
            &terms,
            &header,
            &mut findings,
            "existing.toml",
        );

        assert!(merged.starts_with(existing_text));
        assert!(merged.contains("# Merged by kiro-cli vocab import --format auto-ontology-json --topic sdd (inputs: input.json)\n"));
        // sdd was already declared with [topics.sdd], so no new [topics.sdd]
        assert_eq!(merged.matches("[topics.sdd]").count(), 1);
        // ops was not declared, so [topics.ops] added
        assert!(merged.contains("[topics.ops]\nlabel = \"Operations\"\n"));
        assert!(merged.contains("[terms.deployment]\n"));
        assert!(!merged.contains("[terms.purchase-order]\n"));

        // Two conflict findings
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].code, codes::IMPORT_MERGE_CONFLICT);
        assert!(
            findings[0]
                .message
                .contains("`order` already exists in existing.toml")
        );
        assert_eq!(findings[1].code, codes::IMPORT_MERGE_CONFLICT);
        assert!(
            findings[1]
                .message
                .contains("already names `order` in topic sdd")
        );

        let val_findings = validate_output(&merged, "existing.toml");
        assert!(
            val_findings.is_empty(),
            "merged TOML must validate cleanly: {:?}",
            val_findings
        );
    }
}
