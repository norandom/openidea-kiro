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

fn judge_value(
    config: &CodeConfig,
    entity: &EntityKey,
    metric: &str,
    value: f64,
    line: u32,
    population: Option<f64>,
    out: &mut Vec<Judgement>,
) {
    let Some((limit, source)) = config.limit_for(entity.scope, metric, &entity.path) else {
        return;
    };
    if let Some(minimum) = limit.minimum_population {
        match population {
            Some(count) if count >= f64::from(minimum) => {}
            _ => return,
        }
    }
    if value > limit.maximum {
        out.push(Judgement {
            entity: entity.clone(),
            metric: metric.to_string(),
            value,
            limit: limit.maximum,
            limit_source: source,
            severity: limit.severity,
            line,
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
        judge_value(
            config,
            &observation.entity,
            &observation.metric.name,
            observation.value,
            observation.line,
            None,
            &mut out,
        );
        if observation.entity.scope == Scope::Routine
            && observation.metric.name == metrics::CODE_LINES
            && let Some(statements) = statements_of.get(&observation.entity).copied()
            && statements > 0.0
        {
            judge_value(
                config,
                &observation.entity,
                metrics::LINES_PER_STATEMENT,
                observation.value / statements,
                observation.line,
                Some(statements),
                &mut out,
            );
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
