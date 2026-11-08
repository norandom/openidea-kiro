//! Pair selection and contradiction search (design component `ContradictionSearch`,
//! task 3.3, requirements 4.10, 5.1, 5.3, 5.4, 5.5, 5.7).
//!
//! Evaluates pairs of incompatible criteria to find satisfiable states (contradictions)
//! or report bounded search refusals.

use std::collections::{BTreeMap, BTreeSet};

use biodivine_lib_bdd::Bdd;

use crate::checks::bdd::{Encoding, Refusal, VARIABLE_CEILING};
use crate::checks::finitize::{Domain, Finitized, Typed, Value};
use crate::finding::{Finding, codes};

/// Reason two criteria are incompatible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    Declared,
    Effect {
        id: String,
        left: Value,
        right: Value,
    },
}

/// A pair of incompatible criteria to check for joint satisfiability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub left: String,
    pub right: String,
    pub reason: Reason,
}

/// Summary of contradiction search results.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchSummary {
    pub guarded: usize,
    pub pairs_considered: usize,
    pub contradictions: usize,
    pub refused_pairs: usize,
    pub variables: usize,
    pub max_variables: usize,
    pub max_nodes: usize,
}

/// Computes incompatible criterion pairs in document order.
#[must_use]
pub fn pairs(fin: &Finitized) -> Vec<Pair> {
    let mut result = Vec::new();
    let n = fin.criteria.len();

    for i in 0..n {
        let Some(left) = fin.criteria.get(i) else {
            continue;
        };

        for j in (i.saturating_add(1))..n {
            let Some(right) = fin.criteria.get(j) else {
                continue;
            };

            // Declared conflict (symmetric)
            let is_declared =
                left.conflicts.contains(&right.id) || right.conflicts.contains(&left.id);

            if is_declared {
                result.push(Pair {
                    left: left.id.clone(),
                    right: right.id.clone(),
                    reason: Reason::Declared,
                });
                continue;
            }

            // Effect conflict (first clashing effect wins)
            let mut clash = None;
            for eff_l in &left.effects {
                for eff_r in &right.effects {
                    if eff_l.id == eff_r.id && eff_l.value != eff_r.value {
                        clash = Some(Reason::Effect {
                            id: eff_l.id.clone(),
                            left: eff_l.value.clone(),
                            right: eff_r.value.clone(),
                        });
                        break;
                    }
                }
                if clash.is_some() {
                    break;
                }
            }

            if let Some(reason) = clash {
                result.push(Pair {
                    left: left.id.clone(),
                    right: right.id.clone(),
                    reason,
                });
            }
        }
    }

    result
}

fn value_short(val: &Value) -> String {
    match val {
        Value::Bool(b) => b.to_string(),
        Value::Enum(s) => s.clone(),
        Value::Region {
            low: Some(l),
            high: Some(h),
        } if l == h => l.to_string(),
        Value::Region { low, high } => match (low, high) {
            (Some(l), Some(h)) => format!("{l}..={h}"),
            (Some(l), None) => format!(">={l}"),
            (None, Some(h)) => format!("<={h}"),
            (None, None) => "unconstrained".to_string(),
        },
    }
}

/// Executes the bounded contradiction search over finitised criteria.
#[must_use]
pub fn search(
    fin: &Finitized,
    feature: &str,
    file: &str,
    max_variables: usize,
    max_nodes: usize,
) -> (Vec<Finding>, SearchSummary) {
    let candidate_pairs = pairs(fin);
    let effective_cap = max_variables.min(VARIABLE_CEILING);
    let guarded_count = fin
        .criteria
        .iter()
        .filter(|c| c.guard != Typed::True)
        .count();

    let mut findings = Vec::new();

    // 1. Compile encoding and domain
    let enc = match Encoding::new(&fin.domains, max_variables, max_nodes) {
        Ok(e) => e,
        Err(refusal) => {
            let prefix = match refusal {
                Refusal::Variables { variables, cap } => {
                    format!("variable cap {cap} exceeded: {variables} values")
                }
                Refusal::Nodes { cap, stage } => {
                    format!("node cap exceeded (> {cap} nodes) at {stage}")
                }
            };

            let pairs_str = candidate_pairs
                .iter()
                .map(|p| format!("{}/{}", p.left, p.right))
                .collect::<Vec<_>>()
                .join(", ");

            let mut terms_counts: Vec<(String, usize)> = fin
                .domains
                .iter()
                .map(|d| (d.id.clone(), d.values.len()))
                .collect();
            terms_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
            terms_counts.truncate(2);

            let terms_str = terms_counts
                .iter()
                .map(|(id, count)| format!("{id} ({count} values)"))
                .collect::<Vec<_>>()
                .join(", ");

            let msg = format!(
                "{prefix}; {} pairs not checked: {pairs_str}; largest terms: {terms_str}",
                candidate_pairs.len()
            );

            findings.push(Finding::new(codes::SEARCH_REFUSED, file, msg).feature(feature));

            return (
                findings,
                SearchSummary {
                    guarded: guarded_count,
                    pairs_considered: candidate_pairs.len(),
                    contradictions: 0,
                    refused_pairs: candidate_pairs.len(),
                    variables: fin.domains.iter().map(|d| d.values.len()).sum(),
                    max_variables: effective_cap,
                    max_nodes,
                },
            );
        }
    };

    // 2. Encode guards and detect unsatisfiable ones
    let mut guard_bdds: BTreeMap<String, Bdd> = BTreeMap::new();
    let mut unsatisfiable_criteria = BTreeSet::new();
    let mut refused_criteria = BTreeMap::new();

    for crit in &fin.criteria {
        let stage = format!("guard {}", crit.id);
        match enc.encode(&crit.guard, &fin.domains, max_nodes, &stage) {
            Ok(bdd) => match enc.satisfiable(&bdd, max_nodes, &stage) {
                Ok(true) => {
                    guard_bdds.insert(crit.id.clone(), bdd);
                }
                Ok(false) => {
                    unsatisfiable_criteria.insert(crit.id.clone());
                    if let Some(line) = crit.guard_line {
                        findings.push(
                            Finding::new(
                                codes::GUARD_UNSATISFIABLE,
                                file,
                                "guard holds in no state",
                            )
                            .at(line)
                            .feature(feature),
                        );
                    }
                }
                Err(r) => {
                    refused_criteria.insert(crit.id.clone(), r);
                }
            },
            Err(r) => {
                refused_criteria.insert(crit.id.clone(), r);
            }
        }
    }

    // 3. Evaluate each incompatible pair
    let mut refused_pairs = Vec::new();
    let mut first_refusal = None;
    let mut contradictions_count: usize = 0;

    for pair in &candidate_pairs {
        // If either criterion's guard was refused:
        if let Some(r) = refused_criteria
            .get(&pair.left)
            .or_else(|| refused_criteria.get(&pair.right))
        {
            refused_pairs.push(pair);
            if first_refusal.is_none() {
                first_refusal = Some(r.clone());
            }
            continue;
        }

        // Unsatisfiable guard produces no pair finding
        if unsatisfiable_criteria.contains(&pair.left)
            || unsatisfiable_criteria.contains(&pair.right)
        {
            continue;
        }

        let bdd_left = match guard_bdds.get(&pair.left) {
            Some(b) => b,
            None => continue,
        };
        let bdd_right = match guard_bdds.get(&pair.right) {
            Some(b) => b,
            None => continue,
        };

        let stage = format!("pair {}/{}", pair.left, pair.right);
        match enc.conjoin(bdd_left, bdd_right, max_nodes, &stage) {
            Err(refusal) => {
                refused_pairs.push(pair);
                if first_refusal.is_none() {
                    first_refusal = Some(refusal);
                }
            }
            Ok(joint) => match enc.witness(&joint, &fin.domains, max_nodes, &stage) {
                Err(refusal) => {
                    refused_pairs.push(pair);
                    if first_refusal.is_none() {
                        first_refusal = Some(refusal);
                    }
                }
                Ok(Some(state)) => {
                    contradictions_count = contradictions_count.saturating_add(1);
                    let left_crit = fin.criteria.iter().find(|c| c.id == pair.left);
                    let left_line = left_crit.map_or(1, |c| c.line);

                    let mut mentioned = left_crit.map(|c| c.guard.ids()).unwrap_or_default();
                    if let Some(right_crit) = fin.criteria.iter().find(|c| c.id == pair.right) {
                        mentioned.extend(right_crit.guard.ids());
                    }

                    let witness_str = if mentioned.is_empty() {
                        "in every state".to_string()
                    } else {
                        let mut term_entries: Vec<(&Domain, &str)> = mentioned
                            .iter()
                            .filter_map(|id| {
                                fin.domains
                                    .iter()
                                    .find(|d| d.id == *id)
                                    .map(|d| (d, id.as_str()))
                            })
                            .collect();
                        term_entries
                            .sort_by(|a, b| (&a.0.topic, &a.0.id).cmp(&(&b.0.topic, &b.0.id)));

                        let mut formatted = Vec::new();
                        for (_, id) in term_entries {
                            if let Some(val) = state.get(id) {
                                formatted.push(val.display(id));
                            }
                        }
                        formatted.join(", ")
                    };

                    let reason_str = match &pair.reason {
                        Reason::Declared => "declared conflict".to_string(),
                        Reason::Effect { id, left, right } => {
                            format!(
                                "effect {id}: {} vs {}",
                                value_short(left),
                                value_short(right)
                            )
                        }
                    };

                    findings.push(
                        Finding::new(
                            codes::CONTRADICTION,
                            file,
                            format!(
                                "criteria {} and {} can both apply ({reason_str}); witness: {witness_str}",
                                pair.left, pair.right
                            ),
                        )
                        .at(left_line)
                        .feature(feature),
                    );
                }
                Ok(None) => {
                    // Jointly unsatisfiable: no contradiction
                }
            },
        }
    }

    // 4. Report refused pairs if any
    if !refused_pairs.is_empty() {
        let (cap_str, stage_str) = match &first_refusal {
            Some(Refusal::Nodes { cap, stage }) => (cap.to_string(), stage.clone()),
            _ => (max_nodes.to_string(), "search".to_string()),
        };

        let prefix = format!("node cap exceeded (> {cap_str} nodes) at {stage_str}");
        let pairs_str = refused_pairs
            .iter()
            .map(|p| format!("{}/{}", p.left, p.right))
            .collect::<Vec<_>>()
            .join(", ");

        let largest = enc.largest_blocks(2);
        let largest_str = largest
            .iter()
            .map(|(id, count)| format!("{id} ({count} values)"))
            .collect::<Vec<_>>()
            .join(", ");

        let msg = format!(
            "{prefix}; {} pairs not checked: {pairs_str}; largest terms: {largest_str}",
            refused_pairs.len()
        );

        findings.push(Finding::new(codes::SEARCH_REFUSED, file, msg).feature(feature));
    }

    (
        findings,
        SearchSummary {
            guarded: guarded_count,
            pairs_considered: candidate_pairs.len(),
            contradictions: contradictions_count,
            refused_pairs: refused_pairs.len(),
            variables: enc.variables,
            max_variables: enc.cap,
            max_nodes,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::annotations::collect;
    use crate::checks::finitize::finitize;
    use crate::checks::terms::resolve_scope;
    use crate::requirements::parse;
    use crate::vocab::parse as parse_vocab;

    fn setup_broken() -> (Finitized, String, String) {
        let vocab_toml = r#"
schema = "2"
[topics.navigation]
label = "Navigation"

[topics.telemetry]
label = "Telemetry"

[terms.mode]
topic = "navigation"
label = "Operating Mode"
kind = "enum"
values = ["safe", "normal"]
definition = "Mode."

[terms.depth]
topic = "navigation"
label = "Submersible depth"
kind = "int"
min = 0
definition = "Depth."

[terms.armed]
topic = "telemetry"
label = "Armed state"
kind = "bool"
definition = "Armed."
"#;
        let vocab = parse_vocab(vocab_toml, "vocabulary.toml").vocabulary;
        let topics = vec!["navigation".to_string(), "telemetry".to_string()];

        let doc_text = r#"
# Requirements Document

## Requirements

### Requirement 1: Contradictions
_Terms: mode, depth, armed_

#### Acceptance Criteria

1. While mode is safe and depth exceeds 1000, the controller shall arm.
   _Guard: mode = safe and depth > 1000_
   _Effect: armed = true_

2. While depth is at least 500, the controller shall disarm.
   _Guard: depth >= 500_
   _Effect: armed = false_

3. While mode is safe, the controller shall process.
   _Guard: mode = safe_
   _Conflicts: 1.4_

4. The controller shall be partner.

5. While mode is safe and mode is normal, the controller shall report.
   _Guard: mode = safe and mode = normal_
   _Conflicts: 1.4_

6. While mode is safe, the controller shall be safe.
   _Guard: mode = safe_
   _Conflicts: 1.7_

7. While mode is normal, the controller shall be normal.
   _Guard: mode = normal_
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);
        let fin = finitize(&doc, &ann, &scope, "test", "test.md");

        (fin, "test".to_string(), "test.md".to_string())
    }

    #[test]
    fn pairs_selection_declared_and_effect() {
        let (fin, _, _) = setup_broken();
        let pair_list = pairs(&fin);

        // 1.1 & 1.2: clashing effect on armed (true vs false)
        let p_1_2 = pair_list
            .iter()
            .find(|p| p.left == "1.1" && p.right == "1.2");
        assert!(matches!(
            p_1_2.map(|p| &p.reason),
            Some(Reason::Effect { id, .. }) if id == "armed"
        ));

        // 1.3 & 1.4: declared conflict
        let p_3_4 = pair_list
            .iter()
            .find(|p| p.left == "1.3" && p.right == "1.4");
        assert_eq!(p_3_4.map(|p| &p.reason), Some(&Reason::Declared));

        // 1.4 & 1.5: declared conflict
        let p_4_5 = pair_list
            .iter()
            .find(|p| p.left == "1.4" && p.right == "1.5");
        assert_eq!(p_4_5.map(|p| &p.reason), Some(&Reason::Declared));

        // 1.6 & 1.7: declared conflict
        let p_6_7 = pair_list
            .iter()
            .find(|p| p.left == "1.6" && p.right == "1.7");
        assert_eq!(p_6_7.map(|p| &p.reason), Some(&Reason::Declared));
    }

    #[test]
    fn search_finds_contradictions_and_handles_disjoint_and_unsatisfiable() {
        let (fin, feat, file) = setup_broken();
        let (findings, summary) = search(&fin, &feat, &file, 10_000, 10_000);

        // 1. Unsatisfiable guard on 1.5
        assert!(
            findings
                .iter()
                .any(|f| { f.code == codes::GUARD_UNSATISFIABLE && f.line == Some(26) })
        );

        // 2. Contradiction on 1.1 / 1.2 with effect armed and witness depth >= 1001, mode = safe
        let f_1_2 = findings.iter().find(|f| {
            f.code == codes::CONTRADICTION
                && f.message.contains("criteria 1.1 and 1.2 can both apply (effect armed: true vs false); witness: depth >= 1001, mode = safe")
        });
        assert!(f_1_2.is_some());

        // 3. Contradiction on 1.3 / 1.4 (1.4 is unannotated partner) with witness "mode = safe"
        let f_3_4 = findings.iter().find(|f| {
            f.code == codes::CONTRADICTION
                && f.message.contains(
                    "criteria 1.3 and 1.4 can both apply (declared conflict); witness: mode = safe",
                )
        });
        assert!(f_3_4.is_some());

        // 4. Disjoint guards on 1.6 & 1.7 (mode = safe vs mode = normal) produces NO contradiction
        assert!(!findings.iter().any(|f| {
            f.code == codes::CONTRADICTION && f.message.contains("criteria 1.6 and 1.7")
        }));

        // 5. 1.5 has an unsatisfiable guard, so pair 1.4 / 1.5 produces NO contradiction
        assert!(!findings.iter().any(|f| {
            f.code == codes::CONTRADICTION && f.message.contains("criteria 1.4 and 1.5")
        }));

        assert_eq!(summary.contradictions, 2);
        assert_eq!(summary.refused_pairs, 0);
    }

    #[test]
    fn unconditional_pair_witness_in_every_state() {
        let vocab_toml = r#"
schema = "2"
[topics.navigation]
label = "Navigation"
"#;
        let vocab = parse_vocab(vocab_toml, "vocabulary.toml").vocabulary;
        let topics = vec!["navigation".to_string()];

        let doc_text = r#"
# Requirements Document

## Requirements

### Requirement 1: Declared only

#### Acceptance Criteria

1. Criterion one.
   _Conflicts: 1.2_

2. Criterion two.
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);
        let fin = finitize(&doc, &ann, &scope, "test", "test.md");

        let (findings, summary) = search(&fin, "test", "test.md", 10_000, 10_000);
        assert_eq!(summary.contradictions, 1);
        let f = findings.first();
        assert_eq!(f.map(|x| x.code), Some(codes::CONTRADICTION));
        assert!(
            f.map(|x| x.message.as_str())
                == Some(
                    "criteria 1.1 and 1.2 can both apply (declared conflict); witness: in every state"
                )
        );
    }

    #[test]
    fn variable_and_node_refusals_generate_search_refused_finding() {
        let (fin, feat, file) = setup_broken();

        // Variable cap exceeded
        let (findings_v, summary_v) = search(&fin, &feat, &file, 2, 10_000);
        assert_eq!(summary_v.refused_pairs, 4);
        let f_v = findings_v.first();
        assert_eq!(f_v.map(|x| x.code), Some(codes::SEARCH_REFUSED));
        assert!(
            f_v.map(|x| x.message.as_str())
                .unwrap_or("")
                .starts_with("variable cap 2 exceeded:")
        );
        assert!(
            f_v.map(|x| x.message.as_str()).unwrap_or("").contains(
                "4 pairs not checked: 1.1/1.2, 1.3/1.4, 1.4/1.5, 1.6/1.7; largest terms:"
            )
        );

        // Node cap exceeded during search
        let (findings_n, summary_n) = search(&fin, &feat, &file, 10_000, 2);
        assert_eq!(summary_n.refused_pairs, 4);
        let f_n = findings_n.first();
        assert_eq!(f_n.map(|x| x.code), Some(codes::SEARCH_REFUSED));
        assert!(
            f_n.map(|x| x.message.as_str())
                .unwrap_or("")
                .starts_with("node cap exceeded (> 2 nodes)")
        );
    }
}
