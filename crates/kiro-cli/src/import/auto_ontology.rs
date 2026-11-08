//! Auto Ontology JSON reader.
//!
//! Parses Auto Ontology's `/api/terms` page object or a bare JSON array of terms
//! into `Imported`. Never invents hierarchy or topics.

use std::collections::{BTreeMap, HashSet};

use kiro_core::finding::{Finding, codes};
use kiro_core::vocab::model::is_id;
use kiro_core::vocab::normalize_label;

use super::{Defaults, Imported, ImportedTerm, Input, derive_id};

/// Reads one or more Auto Ontology JSON input sources into `Imported`.
pub fn read(inputs: &[Input], defaults: &Defaults<'_>) -> Result<Imported, Finding> {
    let mut terms = Vec::new();
    let mut findings = Vec::new();

    for input in inputs {
        let val: serde_json::Value = match serde_json::from_str(&input.text) {
            Ok(v) => v,
            Err(err) => {
                let line = err.line();
                return Err(Finding::new(
                    codes::IMPORT_INPUT_INVALID,
                    &input.display,
                    format!("{} is not auto-ontology-json: {}", input.display, err),
                )
                .at(line));
            }
        };

        let raw_items = if let Some(obj) = val.as_object() {
            if let Some(terms_val) = obj.get("terms") {
                if let Some(arr) = terms_val.as_array() {
                    arr
                } else {
                    return Err(Finding::new(
                        codes::IMPORT_INPUT_INVALID,
                        &input.display,
                        format!(
                            "{} is not auto-ontology-json: expected the /api/terms page object or an array of terms",
                            input.display
                        ),
                    ));
                }
            } else {
                return Err(Finding::new(
                    codes::IMPORT_INPUT_INVALID,
                    &input.display,
                    format!(
                        "{} is not auto-ontology-json: expected the /api/terms page object or an array of terms",
                        input.display
                    ),
                ));
            }
        } else if let Some(arr) = val.as_array() {
            arr
        } else {
            return Err(Finding::new(
                codes::IMPORT_INPUT_INVALID,
                &input.display,
                format!(
                    "{} is not auto-ontology-json: expected the /api/terms page object or an array of terms",
                    input.display
                ),
            ));
        };

        for (idx, item) in raw_items.iter().enumerate() {
            let ordinal = idx.saturating_add(1);

            let Some(obj) = item.as_object() else {
                findings.push(Finding::new(
                    codes::IMPORT_TERM_SKIPPED,
                    &input.display,
                    format!("term #{ordinal} is not an object"),
                ));
                continue;
            };

            let item_id = obj.get("id").and_then(|v| v.as_str());
            let id_or_pos = item_id
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("#{ordinal}"));

            let name = obj.get("name").and_then(|v| v.as_str()).map(str::trim);
            let Some(name) = name.filter(|s| !s.is_empty()) else {
                findings.push(Finding::new(
                    codes::IMPORT_TERM_SKIPPED,
                    &input.display,
                    format!("term {id_or_pos} has no name"),
                ));
                continue;
            };

            let id = derive_id(name);
            if id.is_empty() || !is_id(&id) {
                findings.push(Finding::new(
                    codes::IMPORT_TERM_SKIPPED,
                    &input.display,
                    format!("name `{name}` yields no id"),
                ));
                continue;
            }

            let definition = obj
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let norm_label = normalize_label(name);
            let mut alt_labels = Vec::new();
            let mut seen_norm = HashSet::new();

            if let Some(synonyms) = obj.get("synonyms").and_then(|v| v.as_array()) {
                for syn_val in synonyms {
                    if let Some(syn) = syn_val.as_str() {
                        let trimmed = syn.trim();
                        if !trimmed.is_empty() {
                            let norm = normalize_label(trimmed);
                            if norm != norm_label && seen_norm.insert(norm) {
                                alt_labels.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
            alt_labels.sort();

            let mut comments = Vec::new();

            // Extract zones (strings or objects with name/id)
            let mut zone_names = Vec::new();
            if let Some(zones) = obj.get("zones").and_then(|v| v.as_array()) {
                for z in zones {
                    if let Some(s) = z.as_str() {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            zone_names.push(trimmed.to_string());
                        }
                    } else if let Some(z_obj) = z.as_object() {
                        if let Some(n) = z_obj.get("name").and_then(|v| v.as_str()) {
                            let trimmed = n.trim();
                            if !trimmed.is_empty() {
                                zone_names.push(trimmed.to_string());
                            }
                        } else if let Some(id_val) = z_obj.get("id").and_then(|v| v.as_str()) {
                            let trimmed = id_val.trim();
                            if !trimmed.is_empty() {
                                zone_names.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }

            if let Some(id_str) = item_id {
                let mut prov = format!("auto-ontology: id={id_str}");
                if !zone_names.is_empty() {
                    prov.push_str(&format!("; zones={}", zone_names.join(", ")));
                }
                comments.push(prov);
            } else if !zone_names.is_empty() {
                comments.push(format!("auto-ontology: zones={}", zone_names.join(", ")));
            }

            // Extract related_terms (never broader)
            let mut related_names = Vec::new();
            if let Some(related) = obj.get("related_terms").and_then(|v| v.as_array()) {
                for r in related {
                    if let Some(s) = r.as_str() {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() {
                            related_names.push(trimmed.to_string());
                        }
                    } else if let Some(r_obj) = r.as_object() {
                        if let Some(n) = r_obj.get("name").and_then(|v| v.as_str()) {
                            let trimmed = n.trim();
                            if !trimmed.is_empty() {
                                related_names.push(trimmed.to_string());
                            }
                        } else if let Some(id_val) = r_obj.get("id").and_then(|v| v.as_str()) {
                            let trimmed = id_val.trim();
                            if !trimmed.is_empty() {
                                related_names.push(trimmed.to_string());
                            }
                        }
                    }
                }
            }
            if !related_names.is_empty() {
                comments.push(format!("related: {}", related_names.join(", ")));
            }

            terms.push(ImportedTerm {
                id,
                topic: defaults.topic.to_string(),
                label: name.to_string(),
                definition,
                kind: defaults.kind.clone(),
                broader: Vec::new(),
                alt_labels,
                deprecated: false,
                replaced_by: None,
                comments,
                source: format!("{}:{}", input.display, ordinal),
            });
        }
    }

    let topics = BTreeMap::from([(defaults.topic.to_string(), None)]);

    Ok(Imported {
        terms,
        topics,
        findings,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;
    use kiro_core::vocab::model::Kind;

    #[test]
    fn parse_terms_page_object() {
        let json = r#"{
            "total": 2,
            "terms": [
                {
                    "id": "t-1",
                    "name": "Order Processing",
                    "description": "Processing of incoming orders.",
                    "synonyms": ["OP", "order fulfillment"],
                    "zones": ["Operations", "Fulfillment"],
                    "related_terms": [{"id": "t-2", "name": "Payment"}]
                },
                {
                    "id": "t-2",
                    "name": "Payment",
                    "description": "Settlement of invoice.",
                    "synonyms": [],
                    "zones": [{"id": "z-1", "name": "Finance"}]
                }
            ]
        }"#;

        let input = Input {
            display: "terms-page.json".to_string(),
            text: json.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        let imported = read(&[input], &defaults).unwrap();
        assert_eq!(imported.terms.len(), 2);
        assert!(imported.findings.is_empty());

        let t1 = &imported.terms[0];
        assert_eq!(t1.id, "order-processing");
        assert_eq!(t1.label, "Order Processing");
        assert_eq!(t1.definition, "Processing of incoming orders.");
        assert_eq!(t1.alt_labels, vec!["OP", "order fulfillment"]);
        assert_eq!(
            t1.comments,
            vec![
                "auto-ontology: id=t-1; zones=Operations, Fulfillment",
                "related: Payment"
            ]
        );
        assert_eq!(t1.source, "terms-page.json:1");

        let t2 = &imported.terms[1];
        assert_eq!(t2.id, "payment");
        assert_eq!(t2.comments, vec!["auto-ontology: id=t-2; zones=Finance"]);
    }

    #[test]
    fn parse_bare_array() {
        let json = r#"[
            {
                "id": "t-1",
                "name": "Order",
                "description": "An order."
            }
        ]"#;

        let input = Input {
            display: "terms.json".to_string(),
            text: json.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        let imported = read(&[input], &defaults).unwrap();
        assert_eq!(imported.terms.len(), 1);
        assert_eq!(imported.terms[0].id, "order");
    }

    #[test]
    fn parse_invalid_shapes_and_syntax() {
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        // Syntax error with line
        let invalid_json = "{\n  \"invalid\":\n    ,\n}";
        let input = Input {
            display: "invalid.json".to_string(),
            text: invalid_json.to_string(),
        };
        let err = read(&[input], &defaults).unwrap_err();
        assert_eq!(err.code, codes::IMPORT_INPUT_INVALID);
        assert_eq!(err.line, Some(3));
        assert!(err.message.contains("is not auto-ontology-json"));

        // Unexpected top-level type (string)
        let input_str = Input {
            display: "str.json".to_string(),
            text: "\"not an object or array\"".to_string(),
        };
        let err = read(&[input_str], &defaults).unwrap_err();
        assert_eq!(err.code, codes::IMPORT_INPUT_INVALID);
        assert!(err.message.contains("expected the /api/terms page object"));

        // Object without terms array
        let input_obj = Input {
            display: "wrong.json".to_string(),
            text: "{\"items\": []}".to_string(),
        };
        let err = read(&[input_obj], &defaults).unwrap_err();
        assert_eq!(err.code, codes::IMPORT_INPUT_INVALID);
        assert!(err.message.contains("expected the /api/terms page object"));
    }

    #[test]
    fn skipped_null_blank_and_invalid_names() {
        let json = r#"[
            { "id": "t-1", "name": null, "description": "no name" },
            { "id": "t-2", "name": "   ", "description": "blank name" },
            { "name": null, "description": "no id no name" },
            { "id": "t-3", "name": "---", "description": "no valid id" },
            "not an object"
        ]"#;

        let input = Input {
            display: "skip.json".to_string(),
            text: json.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        let imported = read(&[input], &defaults).unwrap();
        assert!(imported.terms.is_empty());
        assert_eq!(imported.findings.len(), 5);

        assert_eq!(imported.findings[0].code, codes::IMPORT_TERM_SKIPPED);
        assert_eq!(imported.findings[0].message, "term t-1 has no name");

        assert_eq!(imported.findings[1].code, codes::IMPORT_TERM_SKIPPED);
        assert_eq!(imported.findings[1].message, "term t-2 has no name");

        assert_eq!(imported.findings[2].code, codes::IMPORT_TERM_SKIPPED);
        assert_eq!(imported.findings[2].message, "term #3 has no name");

        assert_eq!(imported.findings[3].code, codes::IMPORT_TERM_SKIPPED);
        assert_eq!(imported.findings[3].message, "name `---` yields no id");

        assert_eq!(imported.findings[4].code, codes::IMPORT_TERM_SKIPPED);
        assert_eq!(imported.findings[4].message, "term #5 is not an object");
    }

    #[test]
    fn synonym_filtering_and_ignored_properties() {
        let json = r#"[
            {
                "id": "t-1",
                "name": "Order",
                "description": "An order.",
                "synonyms": ["Order", "order", "PO", "purchase order", "  "],
                "certification": "gold",
                "owner_id": 42,
                "schema_names": ["public.orders"],
                "tags": ["finance"]
            }
        ]"#;

        let input = Input {
            display: "syn.json".to_string(),
            text: json.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        let imported = read(&[input], &defaults).unwrap();
        assert_eq!(imported.terms.len(), 1);
        let t = &imported.terms[0];
        // "Order" and "order" equal the normalized label, so omitted
        assert_eq!(t.alt_labels, vec!["PO", "purchase order"]);
    }
}
