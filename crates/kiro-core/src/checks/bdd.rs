//! Decision-diagram engine over `biodivine-lib-bdd` (design component `BddEngine`,
//! task 3.2, requirements 5.2, 5.6, 5.7, 5.8).
//!
//! Provides one-hot domain compilation, bounded BDD operations, and witness decoding.

use std::collections::BTreeMap;

use biodivine_lib_bdd::{Bdd, BddVariable, BddVariableSet, op_function};

use crate::checks::finitize::{Atom, Domain, Typed, Value};

/// Library variable ceiling below `u16::MAX`.
pub const VARIABLE_CEILING: usize = 65_000;

/// Refusal returned when variable or node capacity is exceeded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    Variables { variables: usize, cap: usize },
    Nodes { cap: usize, stage: String },
}

/// A BDD variable encoding and compiled domain constraint for a set of terms.
pub struct Encoding {
    set: BddVariableSet,
    blocks: Vec<(String, Vec<BddVariable>)>,
    index: BTreeMap<String, usize>,
    domain: Bdd,
    pub variables: usize,
    pub cap: usize,
}

impl Encoding {
    /// Compiles one-hot domain blocks and the domain constraint.
    ///
    /// # Errors
    ///
    /// Returns `Refusal::Variables` if total values exceed `cap`,
    /// or `Refusal::Nodes` if any domain conjunction exceeds `max_nodes`.
    pub fn new(
        domains: &[Domain],
        max_variables: usize,
        max_nodes: usize,
    ) -> Result<Encoding, Refusal> {
        let cap = max_variables.min(VARIABLE_CEILING);
        let total_vars: usize = domains.iter().map(|d| d.values.len()).sum();

        if total_vars > cap {
            return Err(Refusal::Variables {
                variables: total_vars,
                cap,
            });
        }

        let var_count_u16 = match u16::try_from(total_vars) {
            Ok(count) => count,
            Err(_) => {
                return Err(Refusal::Variables {
                    variables: total_vars,
                    cap,
                });
            }
        };

        let set = BddVariableSet::new_anonymous(var_count_u16);

        let mut current_idx: usize = 0;
        let mut blocks = Vec::with_capacity(domains.len());
        let mut index = BTreeMap::new();

        for (block_idx, d) in domains.iter().enumerate() {
            let mut vars = Vec::with_capacity(d.values.len());
            for _ in 0..d.values.len() {
                vars.push(BddVariable::from_index(current_idx));
                current_idx = current_idx.saturating_add(1);
            }
            blocks.push((d.id.clone(), vars));
            index.insert(d.id.clone(), block_idx);
        }

        // Domain constraint: exactly one variable per block is true
        let mut domain_bdd = set.mk_true();
        for (_, vars) in &blocks {
            let block_bdd = build_exactly_one(&set, vars, max_nodes)?;
            domain_bdd =
                Bdd::binary_op_with_limit(max_nodes, &domain_bdd, &block_bdd, op_function::and)
                    .ok_or_else(|| Refusal::Nodes {
                        cap: max_nodes,
                        stage: "domain".to_string(),
                    })?;
        }

        Ok(Encoding {
            set,
            blocks,
            index,
            domain: domain_bdd,
            variables: total_vars,
            cap,
        })
    }

    /// Encodes a typed guard expression into a BDD.
    ///
    /// # Errors
    ///
    /// Returns `Refusal::Nodes` if any boolean operation exceeds `max_nodes`.
    pub fn encode(
        &self,
        guard: &Typed,
        domains: &[Domain],
        max_nodes: usize,
        stage: &str,
    ) -> Result<Bdd, Refusal> {
        match guard {
            Typed::True => Ok(self.set.mk_true()),
            Typed::Atom(atom) => {
                let atom_id = match atom {
                    Atom::Bool { id, .. } | Atom::Enum { id, .. } | Atom::Int { id, .. } => id,
                };

                let block_idx = match self.index.get(atom_id) {
                    Some(&idx) => idx,
                    None => return Ok(self.set.mk_true()),
                };

                let vars = match self.blocks.get(block_idx) {
                    Some((_, v)) => v,
                    None => return Ok(self.set.mk_true()),
                };

                let domain = match domains.get(block_idx) {
                    Some(d) => d,
                    None => return Ok(self.set.mk_true()),
                };

                let mut satisfying_vars = Vec::new();
                for (v, val) in vars.iter().zip(domain.values.iter()) {
                    let matches = match atom {
                        Atom::Bool { value, .. } => val == &Value::Bool(*value),
                        Atom::Enum { equal, value, .. } => {
                            if *equal {
                                val == &Value::Enum(value.clone())
                            } else {
                                val != &Value::Enum(value.clone())
                            }
                        }
                        Atom::Int { op, value, .. } => match val {
                            Value::Region { low, high } => {
                                satisfies_int_region(*low, *high, *op, *value)
                            }
                            _ => false,
                        },
                    };

                    if matches {
                        satisfying_vars.push(*v);
                    }
                }

                if satisfying_vars.is_empty() {
                    return Ok(self.set.mk_false());
                }

                let mut acc = self.set.mk_false();
                for v in satisfying_vars {
                    let bit_bdd = self.set.mk_var(v);
                    acc = Bdd::binary_op_with_limit(max_nodes, &acc, &bit_bdd, op_function::or)
                        .ok_or_else(|| Refusal::Nodes {
                            cap: max_nodes,
                            stage: stage.to_string(),
                        })?;
                }
                Ok(acc)
            }
            Typed::Not(inner) => {
                let b = self.encode(inner, domains, max_nodes, stage)?;
                Ok(b.not())
            }
            Typed::And(items) => {
                let mut acc = self.set.mk_true();
                for item in items {
                    let b = self.encode(item, domains, max_nodes, stage)?;
                    acc = Bdd::binary_op_with_limit(max_nodes, &acc, &b, op_function::and)
                        .ok_or_else(|| Refusal::Nodes {
                            cap: max_nodes,
                            stage: stage.to_string(),
                        })?;
                }
                Ok(acc)
            }
            Typed::Or(items) => {
                let mut acc = self.set.mk_false();
                for item in items {
                    let b = self.encode(item, domains, max_nodes, stage)?;
                    acc = Bdd::binary_op_with_limit(max_nodes, &acc, &b, op_function::or)
                        .ok_or_else(|| Refusal::Nodes {
                            cap: max_nodes,
                            stage: stage.to_string(),
                        })?;
                }
                Ok(acc)
            }
        }
    }

    /// Conjoins two BDDs bounded by `max_nodes`.
    ///
    /// # Errors
    ///
    /// Returns `Refusal::Nodes` if the result exceeds `max_nodes`.
    pub fn conjoin(&self, a: &Bdd, b: &Bdd, max_nodes: usize, stage: &str) -> Result<Bdd, Refusal> {
        Bdd::binary_op_with_limit(max_nodes, a, b, op_function::and).ok_or_else(|| Refusal::Nodes {
            cap: max_nodes,
            stage: stage.to_string(),
        })
    }

    /// Returns whether `f` is satisfiable under the domain constraint.
    ///
    /// # Errors
    ///
    /// Returns `Refusal::Nodes` if the conjunction exceeds `max_nodes`.
    pub fn satisfiable(&self, f: &Bdd, max_nodes: usize, stage: &str) -> Result<bool, Refusal> {
        let constrained = self.conjoin(f, &self.domain, max_nodes, stage)?;
        Ok(!constrained.is_false())
    }

    /// Decodes a satisfying witness assignment for `f ∧ domain`.
    ///
    /// # Errors
    ///
    /// Returns `Refusal::Nodes` if the conjunction exceeds `max_nodes`.
    pub fn witness(
        &self,
        f: &Bdd,
        domains: &[Domain],
        max_nodes: usize,
        stage: &str,
    ) -> Result<Option<BTreeMap<String, Value>>, Refusal> {
        let constrained = self.conjoin(f, &self.domain, max_nodes, stage)?;
        let Some(valuation) = constrained.sat_witness() else {
            return Ok(None);
        };

        let mut result = BTreeMap::new();
        for (d_idx, (id, vars)) in self.blocks.iter().enumerate() {
            let Some(domain) = domains.get(d_idx) else {
                continue;
            };
            for (v_idx, &var) in vars.iter().enumerate() {
                if valuation.value(var)
                    && let Some(val) = domain.values.get(v_idx)
                {
                    result.insert(id.clone(), val.clone());
                    break;
                }
            }
        }
        Ok(Some(result))
    }

    /// Returns the `n` largest blocks by value count descending, then by term id.
    #[must_use]
    pub fn largest_blocks(&self, n: usize) -> Vec<(String, usize)> {
        let mut list: Vec<(String, usize)> = self
            .blocks
            .iter()
            .map(|(id, vars)| (id.clone(), vars.len()))
            .collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        list.truncate(n);
        list
    }

    /// Returns the node count of the compiled domain constraint.
    #[must_use]
    pub fn domain_size(&self) -> usize {
        self.domain.size()
    }
}

fn build_exactly_one(
    set: &BddVariableSet,
    vars: &[BddVariable],
    max_nodes: usize,
) -> Result<Bdd, Refusal> {
    if vars.is_empty() {
        return Ok(set.mk_false());
    }
    if vars.len() == 1
        && let Some(&v) = vars.first()
    {
        return Ok(set.mk_var(v));
    }

    let mut exactly_one = set.mk_false();
    for (i, _) in vars.iter().enumerate() {
        let mut minterm = set.mk_true();
        for (j, &v) in vars.iter().enumerate() {
            let bit = if i == j {
                set.mk_var(v)
            } else {
                set.mk_not_var(v)
            };
            minterm = Bdd::binary_op_with_limit(max_nodes, &minterm, &bit, op_function::and)
                .ok_or_else(|| Refusal::Nodes {
                    cap: max_nodes,
                    stage: "domain".to_string(),
                })?;
        }
        exactly_one = Bdd::binary_op_with_limit(max_nodes, &exactly_one, &minterm, op_function::or)
            .ok_or_else(|| Refusal::Nodes {
                cap: max_nodes,
                stage: "domain".to_string(),
            })?;
    }
    Ok(exactly_one)
}

fn satisfies_int_region(
    low: Option<i64>,
    high: Option<i64>,
    op: crate::checks::guard::Op,
    c: i64,
) -> bool {
    use crate::checks::guard::Op;
    match op {
        Op::Eq => low == Some(c) && high == Some(c),
        Op::Ne => high.is_some_and(|h| h < c) || low.is_some_and(|l| l > c),
        Op::Lt => high.is_some_and(|h| h < c),
        Op::Le => high.is_some_and(|h| h <= c),
        Op::Gt => low.is_some_and(|l| l > c),
        Op::Ge => low.is_some_and(|l| l >= c),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::finitize::regions;
    use crate::checks::guard::Op;

    fn sample_domains() -> Vec<Domain> {
        vec![
            Domain {
                id: "mode".to_string(),
                topic: "nav".to_string(),
                values: vec![
                    Value::Enum("safe".to_string()),
                    Value::Enum("normal".to_string()),
                    Value::Enum("fast".to_string()),
                ],
            },
            Domain {
                id: "armed".to_string(),
                topic: "telemetry".to_string(),
                values: vec![Value::Bool(false), Value::Bool(true)],
            },
            Domain {
                id: "depth".to_string(),
                topic: "nav".to_string(),
                values: regions(&[500, 1000], Some(0), None),
            },
        ]
    }

    #[test]
    fn contiguous_blocks_and_variable_allocation() {
        let domains = sample_domains();
        let enc_res = Encoding::new(&domains, 100, 10_000);
        assert!(enc_res.is_ok());
        let enc = enc_res.unwrap_or_else(|_| panic_free_enc());

        // 3 + 2 + 5 = 10 variables
        assert_eq!(enc.variables, 10);
        let mut expected_idx: usize = 0;
        for (_, vars) in &enc.blocks {
            for v in vars {
                assert_eq!(v.to_index(), expected_idx);
                expected_idx = expected_idx.saturating_add(1);
            }
        }
    }

    #[test]
    fn domain_forbids_two_values_and_zero_values_of_one_term() {
        let domains = sample_domains();
        let enc_res = Encoding::new(&domains, 100, 10_000);
        assert!(enc_res.is_ok());
        let enc = enc_res.unwrap_or_else(|_| panic_free_enc());

        let default_slice = [];
        let mode_vars = enc
            .blocks
            .first()
            .map_or(&default_slice[..], |(_, vars)| vars.as_slice());
        assert!(mode_vars.len() >= 2);
        let v0 = enc
            .set
            .mk_var(*mode_vars.first().unwrap_or(&BddVariable::from_index(0)));
        let v1 = enc
            .set
            .mk_var(*mode_vars.get(1).unwrap_or(&BddVariable::from_index(0)));

        // v0 and v1 both true is forbidden by domain
        let both_true = enc
            .conjoin(&v0, &v1, 10_000, "test")
            .unwrap_or(enc.set.mk_false());
        assert!(!enc.satisfiable(&both_true, 10_000, "test").unwrap_or(true));

        // all mode variables false is forbidden by domain
        let mut all_false = enc.set.mk_true();
        for &v in mode_vars {
            let not_v = enc.set.mk_not_var(v);
            all_false = enc
                .conjoin(&all_false, &not_v, 10_000, "test")
                .unwrap_or(enc.set.mk_false());
        }
        assert!(!enc.satisfiable(&all_false, 10_000, "test").unwrap_or(true));
    }

    #[test]
    fn variable_and_node_refusals() {
        let domains = sample_domains();

        // Variable cap exceeded
        let err_vars = Encoding::new(&domains, 5, 10_000);
        assert_eq!(
            err_vars.err(),
            Some(Refusal::Variables {
                variables: 10,
                cap: 5,
            })
        );

        // Node cap exceeded during domain creation
        let err_nodes = Encoding::new(&domains, 100, 2);
        assert_eq!(
            err_nodes.err(),
            Some(Refusal::Nodes {
                cap: 2,
                stage: "domain".to_string(),
            })
        );
    }

    #[test]
    fn witness_decoding_to_term_values() {
        let domains = sample_domains();
        let enc_res = Encoding::new(&domains, 100, 10_000);
        assert!(enc_res.is_ok());
        let enc = enc_res.unwrap_or_else(|_| panic_free_enc());

        let guard = Typed::And(vec![
            Typed::Atom(Atom::Bool {
                id: "armed".to_string(),
                value: true,
            }),
            Typed::Atom(Atom::Enum {
                id: "mode".to_string(),
                equal: true,
                value: "safe".to_string(),
            }),
            Typed::Atom(Atom::Int {
                id: "depth".to_string(),
                op: Op::Gt,
                value: 1000,
            }),
        ]);

        let f_res = enc.encode(&guard, &domains, 10_000, "test");
        assert!(f_res.is_ok());
        let f = f_res.unwrap_or(enc.set.mk_false());

        let witness_opt = enc.witness(&f, &domains, 10_000, "test").unwrap_or(None);
        assert!(witness_opt.is_some());
        let witness = witness_opt.unwrap_or_default();

        assert_eq!(witness.get("armed"), Some(&Value::Bool(true)));
        assert_eq!(witness.get("mode"), Some(&Value::Enum("safe".to_string())));
        assert_eq!(
            witness.get("depth"),
            Some(&Value::Region {
                low: Some(1001),
                high: None,
            })
        );
    }

    #[test]
    fn domain_size_linear_in_blocks() {
        let d1 = vec![Domain {
            id: "t1".to_string(),
            topic: "top".to_string(),
            values: vec![
                Value::Enum("a".to_string()),
                Value::Enum("b".to_string()),
                Value::Enum("c".to_string()),
            ],
        }];
        let enc1 = Encoding::new(&d1, 100, 10_000).unwrap_or_else(|_| panic_free_enc());

        let mut d4 = Vec::new();
        for i in 0..4 {
            d4.push(Domain {
                id: format!("t{i}"),
                topic: "top".to_string(),
                values: vec![
                    Value::Enum("a".to_string()),
                    Value::Enum("b".to_string()),
                    Value::Enum("c".to_string()),
                ],
            });
        }
        let enc4 = Encoding::new(&d4, 100, 10_000).unwrap_or_else(|_| panic_free_enc());

        // Domain size should grow linearly with the number of blocks
        assert!(enc4.domain_size() <= enc1.domain_size().saturating_mul(5));
    }

    #[test]
    fn largest_blocks_order() {
        let domains = sample_domains();
        let enc = Encoding::new(&domains, 100, 10_000).unwrap_or_else(|_| panic_free_enc());
        let top2 = enc.largest_blocks(2);
        assert_eq!(top2.len(), 2);
        // depth has 5 values, mode has 3 values, armed has 2 values
        assert_eq!(top2.first(), Some(&("depth".to_string(), 5)));
        assert_eq!(top2.get(1), Some(&("mode".to_string(), 3)));
    }

    fn panic_free_enc() -> Encoding {
        let d = vec![Domain {
            id: "dummy".to_string(),
            topic: "top".to_string(),
            values: vec![Value::Bool(true)],
        }];
        Encoding::new(&d, 10, 100).unwrap_or_else(|_| {
            let set = BddVariableSet::new_anonymous(0);
            let domain = set.mk_true();
            Encoding {
                set,
                blocks: Vec::new(),
                index: BTreeMap::new(),
                domain,
                variables: 0,
                cap: 0,
            }
        })
    }
}
