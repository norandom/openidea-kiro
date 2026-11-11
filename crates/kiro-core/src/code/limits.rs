//! Absolute limits on routines, types and files (code-constraints 3 and 4).
//!
//! A value is judged strictly: equal to its limit passes. Container counts
//! are never ratcheted; they are judged here and nowhere else.

use super::config::{CodeConfig, LimitSource};
use super::facts::{CodeFacts, EntityKey, Observation, Scope, metrics};
use crate::finding::Severity;

/// One value above its limit.
#[derive(Debug, Clone, PartialEq)]
pub struct Judgement {
    pub entity: EntityKey,
    pub metric: String,
    pub value: f64,
    pub limit: f64,
    pub limit_source: LimitSource,
    pub severity: Severity,
    /// 1-based line where the entity starts.
    pub line: u32,
}

/// One value to judge: a measured observation, or a value derived from two.
struct Candidate<'a> {
    entity: &'a EntityKey,
    metric: &'a str,
    value: f64,
    /// 1-based line where the entity starts.
    line: u32,
    /// The count a limit with a minimum population is checked against;
    /// `None` when the value has no such count.
    population: Option<f64>,
}

fn judge_value(config: &CodeConfig, candidate: &Candidate<'_>, out: &mut Vec<Judgement>) {
    let entity = candidate.entity;
    let Some((limit, source)) = config.limit_for(entity.scope, candidate.metric, &entity.path)
    else {
        return;
    };
    if let Some(minimum) = limit.minimum_population {
        match candidate.population {
            Some(count) if count >= f64::from(minimum) => {}
            _ => return,
        }
    }
    if candidate.value > limit.maximum {
        out.push(Judgement {
            entity: entity.clone(),
            metric: candidate.metric.to_string(),
            value: candidate.value,
            limit: limit.maximum,
            limit_source: source,
            severity: limit.severity,
            line: candidate.line,
        });
    }
}

/// Judges every observation of a selected file against the limits.
///
/// Lines per statement is derived from code lines and statements and is
/// judged only for routines at or above its statement floor; a routine
/// without both values is not judged on it.
#[must_use]
pub fn judge(
    facts: &CodeFacts,
    config: &CodeConfig,
    selected: &dyn Fn(&str) -> bool,
) -> Vec<Judgement> {
    let mut out = Vec::new();
    // Statement counts by routine, looked up once per routine below.
    let statements_of: std::collections::BTreeMap<&EntityKey, f64> = facts
        .observations
        .iter()
        .filter(|observation| {
            observation.entity.scope == Scope::Routine
                && observation.metric.name == metrics::STATEMENTS
        })
        .map(|observation| (&observation.entity, observation.value))
        .collect();
    for observation in &facts.observations {
        if !selected(&observation.entity.path) {
            continue;
        }
        let measured = Candidate {
            entity: &observation.entity,
            metric: &observation.metric.name,
            value: observation.value,
            line: observation.line,
            population: None,
        };
        judge_value(config, &measured, &mut out);
        if observation.entity.scope == Scope::Routine
            && observation.metric.name == metrics::CODE_LINES
            && let Some(statements) = statements_of.get(&observation.entity).copied()
            && statements > 0.0
        {
            let derived = Candidate {
                entity: &observation.entity,
                metric: metrics::LINES_PER_STATEMENT,
                value: observation.value / statements,
                line: observation.line,
                population: Some(statements),
            };
            judge_value(config, &derived, &mut out);
        }
    }
    out.sort_by(|a, b| (&a.entity, &a.metric).cmp(&(&b.entity, &b.metric)));
    out
}

/// The highest observation of every scope and metric, for a first look at
/// a grown repository (code-constraints 3.6).
#[must_use]
pub fn highest<'a>(facts: &'a CodeFacts, selected: &dyn Fn(&str) -> bool) -> Vec<&'a Observation> {
    let mut best: Vec<&Observation> = Vec::new();
    for observation in &facts.observations {
        if !selected(&observation.entity.path) {
            continue;
        }
        let slot = best.iter_mut().find(|current| {
            current.entity.scope == observation.entity.scope && current.metric == observation.metric
        });
        match slot {
            Some(current) => {
                if observation.value > current.value {
                    *current = observation;
                }
            }
            None => best.push(observation),
        }
    }
    best.sort_by(|a, b| (a.entity.scope, &a.metric).cmp(&(b.entity.scope, &b.metric)));
    best
}
