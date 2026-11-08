//! SKOS Turtle reader.

use std::collections::{BTreeMap, HashSet};

use kiro_core::finding::{Finding, codes};
use kiro_core::vocab::model::{Kind, is_id};
use oxrdf::NamedOrBlankNodeRef;

use super::{Defaults, Imported, ImportedTerm, Input, derive_id};

pub const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";
pub const OWL: &str = "http://www.w3.org/2002/07/owl#";
pub const DCT: &str = "http://purl.org/dc/terms/";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const KIRO: &str = "urn:openidea-kiro:vocab:";

#[derive(Default)]
struct LiteralPref {
    value: Option<String>,
    rank: u8, // 0 = untagged, 1 = @en, 2 = other
}

impl LiteralPref {
    fn update(&mut self, lit: &oxrdf::LiteralRef<'_>) {
        let (rank, text) = if lit.language().is_none() {
            (0, lit.value().to_string())
        } else if lit
            .language()
            .is_some_and(|lang| lang.eq_ignore_ascii_case("en"))
        {
            (1, lit.value().to_string())
        } else {
            (2, lit.value().to_string())
        };

        if self.value.is_none() || rank < self.rank {
            self.value = Some(text);
            self.rank = rank;
        }
    }
}

fn local_name(iri: &str) -> &str {
    if let Some((_, after_hash)) = iri.rsplit_once('#') {
        after_hash
    } else if let Some((_, after_slash)) = iri.rsplit_once('/') {
        after_slash
    } else {
        iri
    }
}

pub fn read(inputs: &[Input], defaults: &Defaults<'_>) -> Result<Imported, Finding> {
    let mut terms = Vec::new();
    let mut findings = Vec::new();
    let mut imported_topics: BTreeMap<String, Option<String>> = BTreeMap::new();
    imported_topics.insert(defaults.topic.to_string(), None);

    for input in inputs {
        let parser = oxttl::TurtleParser::new().for_reader(input.text.as_bytes());
        let mut triples = Vec::new();

        for res in parser {
            match res {
                Ok(triple) => triples.push(triple),
                Err(err) => {
                    let line = match err {
                        oxttl::TurtleParseError::Syntax(ref s) => {
                            let l = s.location().start.line;
                            usize::try_from(l).ok().and_then(|val| val.checked_add(1))
                        }
                        _ => None,
                    };
                    let mut finding = Finding::new(
                        codes::IMPORT_INPUT_INVALID,
                        &input.display,
                        format!("{} is not skos-turtle: {}", input.display, err),
                    );
                    if let Some(ln) = line {
                        finding = finding.at(ln);
                    }
                    return Err(finding);
                }
            }
        }

        // 1. Identify concept subjects and concept scheme subjects in first-occurrence order
        let mut concept_order = Vec::new();
        let mut concept_set = HashSet::new();
        let mut schemes = Vec::new();

        let skos_concept = format!("{SKOS}Concept");
        let skos_concept_scheme = format!("{SKOS}ConceptScheme");

        for triple in &triples {
            if let (NamedOrBlankNodeRef::NamedNode(subj), oxrdf::TermRef::NamedNode(obj)) =
                (triple.subject.as_ref(), triple.object.as_ref())
                && triple.predicate.as_ref().as_str() == RDF_TYPE
            {
                if obj.as_str() == skos_concept {
                    let iri = subj.as_str().to_string();
                    if concept_set.insert(iri.clone()) {
                        concept_order.push(iri);
                    }
                } else if obj.as_str() == skos_concept_scheme {
                    let iri = subj.as_str().to_string();
                    if !schemes.contains(&iri) {
                        schemes.push(iri);
                    }
                }
            }
        }

        // Check single concept scheme for default topic label
        if schemes.len() == 1
            && let Some(scheme_iri) = schemes.first()
        {
            let skos_pref_label = format!("{SKOS}prefLabel");
            let dct_title = format!("{DCT}title");
            let mut scheme_title = LiteralPref::default();

            for triple in &triples {
                if let NamedOrBlankNodeRef::NamedNode(subj) = triple.subject.as_ref()
                    && subj.as_str() == scheme_iri
                {
                    let pred = triple.predicate.as_ref().as_str();
                    if (pred == skos_pref_label || pred == dct_title)
                        && let oxrdf::TermRef::Literal(lit) = triple.object.as_ref()
                    {
                        scheme_title.update(&lit);
                    }
                }
            }

            if let Some(title) = scheme_title.value {
                imported_topics.insert(defaults.topic.to_string(), Some(title));
            }
        }

        // Predicate constants for matching
        let skos_pref_label = format!("{SKOS}prefLabel");
        let skos_definition = format!("{SKOS}definition");
        let skos_alt_label = format!("{SKOS}altLabel");
        let skos_broader = format!("{SKOS}broader");
        let skos_narrower = format!("{SKOS}narrower");
        let owl_deprecated = format!("{OWL}deprecated");
        let dct_is_replaced_by = format!("{DCT}isReplacedBy");
        let kiro_topic_pred = format!("{KIRO}topic");
        let kiro_kind_pred = format!("{KIRO}kind");
        let kiro_value_pred = format!("{KIRO}value");
        let kiro_min_pred = format!("{KIRO}min");
        let kiro_max_pred = format!("{KIRO}max");

        // 2. Process each concept in first-occurrence order
        for concept_iri in concept_order {
            let loc_name = local_name(&concept_iri);
            let derived = derive_id(loc_name);

            if derived.is_empty() || !is_id(&derived) {
                findings.push(Finding::new(
                    codes::IMPORT_TERM_SKIPPED,
                    &input.display,
                    format!("concept `{concept_iri}` yields no valid id"),
                ));
                continue;
            }

            let mut pref_label_pref = LiteralPref::default();
            let mut definition_pref = LiteralPref::default();
            let mut alt_labels = Vec::new();
            let mut broader_set = HashSet::new();
            let mut deprecated = false;
            let mut replaced_by = None;
            let mut topic_override = None;
            let mut kind_override = None;
            let mut enum_values = Vec::new();
            let mut int_min = None;
            let mut int_max = None;
            let mut comments = Vec::new();

            for triple in &triples {
                let subj = match triple.subject.as_ref() {
                    NamedOrBlankNodeRef::NamedNode(n) => n.as_str(),
                    _ => continue,
                };
                let pred = triple.predicate.as_ref().as_str();

                if subj == concept_iri {
                    if pred == skos_pref_label {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            pref_label_pref.update(&lit);
                        }
                    } else if pred == skos_definition {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            definition_pref.update(&lit);
                        }
                    } else if pred == skos_alt_label {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            let trimmed = lit.value().trim();
                            if !trimmed.is_empty() && !alt_labels.contains(&trimmed.to_string()) {
                                alt_labels.push(trimmed.to_string());
                            }
                        }
                    } else if pred == skos_broader {
                        if let oxrdf::TermRef::NamedNode(target) = triple.object.as_ref() {
                            let target_id = derive_id(local_name(target.as_str()));
                            if !target_id.is_empty() {
                                broader_set.insert(target_id);
                            }
                        }
                    } else if pred == owl_deprecated {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref()
                            && lit.value().eq_ignore_ascii_case("true")
                        {
                            deprecated = true;
                        }
                    } else if pred == dct_is_replaced_by {
                        if let oxrdf::TermRef::NamedNode(target) = triple.object.as_ref() {
                            let rep_id = derive_id(local_name(target.as_str()));
                            if !rep_id.is_empty() {
                                replaced_by = Some(rep_id);
                            }
                        }
                    } else if pred == kiro_topic_pred {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            let t = lit.value().trim().to_string();
                            if !t.is_empty() {
                                imported_topics.entry(t.clone()).or_insert(None);
                                topic_override = Some(t);
                            }
                        }
                    } else if pred == kiro_kind_pred {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            let k = lit.value().trim().to_ascii_lowercase();
                            match k.as_str() {
                                "entity" => kind_override = Some("entity"),
                                "bool" => kind_override = Some("bool"),
                                "enum" => kind_override = Some("enum"),
                                "int" => kind_override = Some("int"),
                                other => {
                                    comments.push(format!("kiro:kind `{other}` not recognised"));
                                }
                            }
                        }
                    } else if pred == kiro_value_pred {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref() {
                            let val = lit.value().trim().to_string();
                            if !val.is_empty() && !enum_values.contains(&val) {
                                enum_values.push(val);
                            }
                        }
                    } else if pred == kiro_min_pred {
                        if let oxrdf::TermRef::Literal(lit) = triple.object.as_ref()
                            && let Ok(v) = lit.value().trim().parse::<i64>()
                        {
                            int_min = Some(v);
                        }
                    } else if pred == kiro_max_pred
                        && let oxrdf::TermRef::Literal(lit) = triple.object.as_ref()
                        && let Ok(v) = lit.value().trim().parse::<i64>()
                    {
                        int_max = Some(v);
                    }
                } else if pred == skos_narrower
                    && let oxrdf::TermRef::NamedNode(obj_node) = triple.object.as_ref()
                    && obj_node.as_str() == concept_iri
                {
                    let source_id = derive_id(local_name(subj));
                    if !source_id.is_empty() {
                        broader_set.insert(source_id);
                    }
                }
            }

            let label = pref_label_pref.value.unwrap_or_else(|| derived.clone());
            let definition = definition_pref.value.unwrap_or_default();
            alt_labels.sort();
            let mut broader: Vec<String> = broader_set.into_iter().collect();
            broader.sort();

            let topic = topic_override.unwrap_or_else(|| defaults.topic.to_string());
            let kind = match kind_override {
                Some("entity") => Kind::Entity,
                Some("bool") => Kind::Bool,
                Some("enum") => Kind::Enum {
                    values: enum_values,
                },
                Some("int") => Kind::Int {
                    min: int_min,
                    max: int_max,
                },
                _ => defaults.kind.clone(),
            };

            terms.push(ImportedTerm {
                id: derived,
                topic,
                label,
                definition,
                kind,
                broader,
                alt_labels,
                deprecated,
                replaced_by,
                comments,
                source: concept_iri,
            });
        }
    }

    Ok(Imported {
        terms,
        topics: imported_topics,
        findings,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn parse_concepts_ttl_fixture() {
        let ttl = include_str!("../../fixtures/import/concepts.ttl");
        let input = Input {
            display: "concepts.ttl".to_string(),
            text: ttl.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "default_topic",
            kind: &kind,
        };

        let imported = read(&[input], &defaults).unwrap();
        assert!(imported.findings.is_empty());
        // Concepts in order: InsiderThreat, HTTPSProxy, Level2Cache, Threat
        // NonConcept (Collection) and _:b0 (BlankNode) ignored!
        assert_eq!(imported.terms.len(), 4);

        // 1. InsiderThreat
        let t1 = &imported.terms[0];
        assert_eq!(t1.id, "insider-threat");
        assert_eq!(t1.label, "Insider Threat");
        assert_eq!(
            t1.definition,
            "A malicious threat from within the organisation."
        );
        assert_eq!(t1.alt_labels, vec!["internal threat", "rogue employee"]);
        assert_eq!(t1.topic, "sec");
        assert_eq!(
            t1.kind,
            Kind::Enum {
                values: vec!["low".to_string(), "high".to_string()]
            }
        );
        assert_eq!(t1.broader, vec!["threat"]);

        // 2. HTTPSProxy
        let t2 = &imported.terms[1];
        assert_eq!(t2.id, "https-proxy");
        assert_eq!(t2.label, "HTTPS Proxy");
        assert_eq!(t2.definition, "An intermediary proxy server.");
        assert_eq!(t2.alt_labels, vec!["SSL Proxy"]);
        assert_eq!(t2.topic, "net");
        assert!(t2.deprecated);
        assert_eq!(t2.replaced_by.as_deref(), Some("level2-cache"));

        // 3. Level2Cache (has @fr label and skos:narrower ex:L2SubCache)
        let t3 = &imported.terms[2];
        assert_eq!(t3.id, "level2-cache");
        assert_eq!(t3.label, "Level 2 Cache");
        assert_eq!(t3.definition, "CPU cache memory.");
        assert_eq!(t3.topic, "arch");
        assert_eq!(
            t3.kind,
            Kind::Int {
                min: Some(1),
                max: Some(100)
            }
        );

        // 4. Threat
        let t4 = &imported.terms[3];
        assert_eq!(t4.id, "threat");
        assert_eq!(t4.topic, "default_topic");

        // Scheme title attached to default_topic
        assert_eq!(
            imported
                .topics
                .get("default_topic")
                .and_then(|o| o.as_deref()),
            Some("Security & Architecture Concepts")
        );
        assert!(imported.topics.contains_key("sec"));
        assert!(imported.topics.contains_key("net"));
        assert!(imported.topics.contains_key("arch"));
    }

    #[test]
    fn parse_invalid_ttl() {
        let ttl = include_str!("../../fixtures/import/invalid.ttl");
        let input = Input {
            display: "invalid.ttl".to_string(),
            text: ttl.to_string(),
        };
        let kind = Kind::Entity;
        let defaults = Defaults {
            topic: "sdd",
            kind: &kind,
        };

        let err = read(&[input], &defaults).unwrap_err();
        assert_eq!(err.code, codes::IMPORT_INPUT_INVALID);
        assert_eq!(err.line, Some(4));
        assert!(err.message.contains("is not skos-turtle"));
    }
}
