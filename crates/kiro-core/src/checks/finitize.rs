//! Finitization: typed guards, domains and conflicts (design component `Finitization`,
//! task 3.1, requirements 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 4.9, 4.10, 5.2, 5.6).
//!
//! Provides conversion from raw annotated criteria into typed expressions,
//! integer regions, one-hot domains, and prepared search candidates.

use std::collections::{BTreeMap, BTreeSet};

use crate::checks::annotations::{Annotated, KEY_CONFLICTS, KEY_EFFECT, KEY_GUARD};
use crate::checks::guard::{self, Assignment, Expr, Literal, Op, ParseError};
use crate::checks::terms::{Resolved, TermScope};
use crate::finding::{Finding, codes};
use crate::requirements::RequirementsDoc;
use crate::vocab::model::Kind;

/// A concrete or regional value of a vocabulary term.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Bool(bool),
    Enum(String),
    Region { low: Option<i64>, high: Option<i64> },
}

impl Value {
    /// Formats the value for human display in witnesses and error messages.
    #[must_use]
    pub fn display(&self, term: &str) -> String {
        match self {
            Value::Bool(b) => format!("{term} = {b}"),
            Value::Enum(e) => format!("{term} = {e}"),
            Value::Region { low, high } => match (low, high) {
                (Some(l), Some(h)) if l == h => format!("{term} = {l}"),
                (Some(l), Some(h)) => format!("{term} in {l}..={h}"),
                (Some(l), None) => format!("{term} >= {l}"),
                (None, Some(h)) => format!("{term} <= {h}"),
                (None, None) => format!("{term} unconstrained"),
            },
        }
    }
}

/// A one-hot domain for a vocabulary term used in guards or effects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Domain {
    pub id: String,
    pub topic: String,
    pub values: Vec<Value>,
}

/// A typed atomic proposition over a term.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    Bool {
        id: String,
        value: bool,
    },
    Enum {
        id: String,
        equal: bool,
        value: String,
    },
    Int {
        id: String,
        op: Op,
        value: i64,
    },
}

/// A typed boolean guard expression.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Typed {
    True,
    Atom(Atom),
    Not(Box<Typed>),
    And(Vec<Typed>),
    Or(Vec<Typed>),
}

impl Typed {
    /// Returns all term IDs referenced in this expression.
    #[must_use]
    pub fn ids(&self) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        self.collect_ids(&mut set);
        set
    }

    fn collect_ids(&self, set: &mut BTreeSet<String>) {
        match self {
            Typed::True => {}
            Typed::Atom(atom) => match atom {
                Atom::Bool { id, .. } | Atom::Enum { id, .. } | Atom::Int { id, .. } => {
                    set.insert(id.clone());
                }
            },
            Typed::Not(inner) => inner.collect_ids(set),
            Typed::And(items) | Typed::Or(items) => {
                for item in items {
                    item.collect_ids(set);
                }
            }
        }
    }

    /// Evaluates the expression over an assignment of term values (oracle for tests).
    #[must_use]
    pub fn eval(&self, state: &BTreeMap<String, Value>) -> bool {
        match self {
            Typed::True => true,
            Typed::Atom(atom) => match atom {
                Atom::Bool { id, value } => match state.get(id) {
                    Some(Value::Bool(b)) => *b == *value,
                    _ => false,
                },
                Atom::Enum { id, equal, value } => match state.get(id) {
                    Some(Value::Enum(s)) => {
                        if *equal {
                            s == value
                        } else {
                            s != value
                        }
                    }
                    _ => false,
                },
                Atom::Int { id, op, value } => match state.get(id) {
                    Some(Value::Region { low, high }) => {
                        satisfies_int_region(*low, *high, *op, *value)
                    }
                    _ => false,
                },
            },
            Typed::Not(inner) => !inner.eval(state),
            Typed::And(items) => items.iter().all(|item| item.eval(state)),
            Typed::Or(items) => items.iter().any(|item| item.eval(state)),
        }
    }
}

fn satisfies_int_region(low: Option<i64>, high: Option<i64>, op: Op, c: i64) -> bool {
    match op {
        Op::Eq => low == Some(c) && high == Some(c),
        Op::Ne => high.is_some_and(|h| h < c) || low.is_some_and(|l| l > c),
        Op::Lt => high.is_some_and(|h| h < c),
        Op::Le => high.is_some_and(|h| h <= c),
        Op::Gt => low.is_some_and(|l| l > c),
        Op::Ge => low.is_some_and(|l| l >= c),
    }
}

/// A typed effect assignment on a criterion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedEffect {
    pub id: String,
    pub value: Value,
}

/// A prepared criterion accepted for contradiction search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prepared {
    pub id: String,
    pub line: usize,
    pub guard_line: Option<usize>,
    pub guard: Typed,
    pub effects: Vec<TypedEffect>,
    pub conflicts: Vec<String>,
}

/// Result of finitizing a requirements document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Finitized {
    pub criteria: Vec<Prepared>,
    pub domains: Vec<Domain>,
    pub findings: Vec<Finding>,
    pub excluded: usize,
}

/// Ascending regions of an integer term cut at `constants` inside optional bounds.
#[must_use]
pub fn regions(constants: &[i64], min: Option<i64>, max: Option<i64>) -> Vec<Value> {
    if let (Some(m), Some(m_max)) = (min, max)
        && m > m_max
    {
        return Vec::new();
    }

    let mut consts: Vec<i64> = constants
        .iter()
        .copied()
        .filter(|&c| min.is_none_or(|m| c >= m) && max.is_none_or(|m| c <= m))
        .collect();
    consts.sort_unstable();
    consts.dedup();

    if consts.is_empty() {
        return vec![Value::Region {
            low: min,
            high: max,
        }];
    }

    let mut result = Vec::new();

    // Below the first constant
    if let Some(&first) = consts.first() {
        let low = min;
        let high = Some(first.saturating_sub(1));
        let valid = low.is_none_or(|l| l <= first.saturating_sub(1));
        if valid {
            result.push(Value::Region { low, high });
        }
    }

    // Each constant as a point, and gaps between them
    for i in 0..consts.len() {
        if let Some(&c) = consts.get(i) {
            result.push(Value::Region {
                low: Some(c),
                high: Some(c),
            });

            if let Some(&next) = consts.get(i.saturating_add(1)) {
                let gap_low = c.saturating_add(1);
                let gap_high = next.saturating_sub(1);
                if gap_low <= gap_high {
                    result.push(Value::Region {
                        low: Some(gap_low),
                        high: Some(gap_high),
                    });
                }
            }
        }
    }

    // Above the last constant
    if let Some(&last) = consts.last() {
        let low = Some(last.saturating_add(1));
        let high = max;
        let valid = high.is_none_or(|h| h >= last.saturating_add(1));
        if valid {
            result.push(Value::Region { low, high });
        }
    }

    result
}

fn key_display(key: &str) -> &'static str {
    if key.eq_ignore_ascii_case(KEY_GUARD) {
        "_Guard:_"
    } else if key.eq_ignore_ascii_case(KEY_EFFECT) {
        "_Effect:_"
    } else if key.eq_ignore_ascii_case(KEY_CONFLICTS) {
        "_Conflicts:_"
    } else {
        "_Terms:_"
    }
}

fn literal_kind_phrase(lit: &Literal) -> &'static str {
    match lit {
        Literal::Bool(_) => "a boolean",
        Literal::Int(_) => "an integer",
        Literal::Word(_) => "a word",
    }
}

enum TypeCheckResult<T> {
    Ok(T),
    Finding(Finding),
    SilentExclusion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TermInfo {
    id: String,
    topic: String,
    kind: Kind,
}

struct Context<'a> {
    scope: &'a TermScope,
    req_id: &'a str,
    crit_id: &'a str,
    file: &'a str,
    feature: &'a str,
    active_terms: &'a mut BTreeMap<String, TermInfo>,
}

fn check_term_ref(
    term_name: &str,
    line: usize,
    ctx: &mut Context<'_>,
) -> TypeCheckResult<TermInfo> {
    match ctx.scope.reference(ctx.req_id, ctx.crit_id, term_name) {
        None => TypeCheckResult::Finding(
            Finding::new(codes::GUARD_TERM_UNDECLARED, ctx.file, term_name)
                .at(line)
                .feature(ctx.feature),
        ),
        Some(decl) => match &decl.resolved {
            Resolved::Found {
                id, topic, kind, ..
            } => {
                let info = TermInfo {
                    id: id.clone(),
                    topic: topic.clone(),
                    kind: kind.clone(),
                };
                ctx.active_terms.insert(id.clone(), info.clone());
                TypeCheckResult::Ok(info)
            }
            _ => TypeCheckResult::SilentExclusion,
        },
    }
}

fn check_word_literal(w: &str, line: usize, ctx: &mut Context<'_>) -> Result<(), Finding> {
    if ctx.scope.reference(ctx.req_id, ctx.crit_id, w).is_some() {
        return Err(Finding::new(
            codes::GUARD_UNPARSED,
            ctx.file,
            format!("{w} is a term; comparisons are against literal constants only"),
        )
        .at(line)
        .feature(ctx.feature));
    }
    Ok(())
}

fn type_expr(expr: &Expr, line: usize, ctx: &mut Context<'_>) -> TypeCheckResult<Typed> {
    match expr {
        Expr::True => TypeCheckResult::Ok(Typed::True),
        Expr::Term(name) => {
            let term = match check_term_ref(name, line, ctx) {
                TypeCheckResult::Ok(t) => t,
                TypeCheckResult::Finding(f) => return TypeCheckResult::Finding(f),
                TypeCheckResult::SilentExclusion => return TypeCheckResult::SilentExclusion,
            };

            if term.kind != Kind::Bool {
                return TypeCheckResult::Finding(
                    Finding::new(
                        codes::GUARD_KIND_MISMATCH,
                        ctx.file,
                        format!(
                            "{} ({}) cannot appear bare; only boolean terms can appear bare",
                            term.id,
                            term.kind.name()
                        ),
                    )
                    .at(line)
                    .feature(ctx.feature),
                );
            }

            TypeCheckResult::Ok(Typed::Atom(Atom::Bool {
                id: term.id.clone(),
                value: true,
            }))
        }
        Expr::Compare {
            term: name,
            op,
            value,
        } => {
            if let Literal::Word(w) = value
                && let Err(f) = check_word_literal(w, line, ctx)
            {
                return TypeCheckResult::Finding(f);
            }

            let term = match check_term_ref(name, line, ctx) {
                TypeCheckResult::Ok(t) => t,
                TypeCheckResult::Finding(f) => return TypeCheckResult::Finding(f),
                TypeCheckResult::SilentExclusion => return TypeCheckResult::SilentExclusion,
            };

            match &term.kind {
                Kind::Entity => TypeCheckResult::Finding(
                    Finding::new(
                        codes::GUARD_KIND_MISMATCH,
                        ctx.file,
                        format!(
                            "{} (entity) cannot be compared with {}",
                            term.id,
                            literal_kind_phrase(value)
                        ),
                    )
                    .at(line)
                    .feature(ctx.feature),
                ),
                Kind::Bool => {
                    if *op != Op::Eq && *op != Op::Ne {
                        return TypeCheckResult::Finding(
                            Finding::new(
                                codes::GUARD_KIND_MISMATCH,
                                ctx.file,
                                format!(
                                    "{} (bool) cannot use ordering operator {}",
                                    term.id,
                                    op.as_str()
                                ),
                            )
                            .at(line)
                            .feature(ctx.feature),
                        );
                    }
                    match value {
                        Literal::Bool(b) => {
                            let expected_val = if *op == Op::Eq { *b } else { !b };
                            TypeCheckResult::Ok(Typed::Atom(Atom::Bool {
                                id: term.id.clone(),
                                value: expected_val,
                            }))
                        }
                        _ => TypeCheckResult::Finding(
                            Finding::new(
                                codes::GUARD_KIND_MISMATCH,
                                ctx.file,
                                format!(
                                    "{} (bool) cannot be compared with {}",
                                    term.id,
                                    literal_kind_phrase(value)
                                ),
                            )
                            .at(line)
                            .feature(ctx.feature),
                        ),
                    }
                }
                Kind::Enum { values } => {
                    if *op != Op::Eq && *op != Op::Ne {
                        return TypeCheckResult::Finding(
                            Finding::new(
                                codes::GUARD_KIND_MISMATCH,
                                ctx.file,
                                format!(
                                    "{} (enum) cannot use ordering operator {}",
                                    term.id,
                                    op.as_str()
                                ),
                            )
                            .at(line)
                            .feature(ctx.feature),
                        );
                    }
                    match value {
                        Literal::Word(w) => {
                            if !values.contains(w) {
                                TypeCheckResult::Finding(
                                    Finding::new(
                                        codes::GUARD_KIND_MISMATCH,
                                        ctx.file,
                                        format!(
                                            "{} (enum) value '{}' is not in declared values: {}",
                                            term.id,
                                            w,
                                            values.join(", ")
                                        ),
                                    )
                                    .at(line)
                                    .feature(ctx.feature),
                                )
                            } else {
                                TypeCheckResult::Ok(Typed::Atom(Atom::Enum {
                                    id: term.id.clone(),
                                    equal: *op == Op::Eq,
                                    value: w.clone(),
                                }))
                            }
                        }
                        _ => TypeCheckResult::Finding(
                            Finding::new(
                                codes::GUARD_KIND_MISMATCH,
                                ctx.file,
                                format!(
                                    "{} (enum) cannot be compared with {}",
                                    term.id,
                                    literal_kind_phrase(value)
                                ),
                            )
                            .at(line)
                            .feature(ctx.feature),
                        ),
                    }
                }
                Kind::Int { .. } => match value {
                    Literal::Int(n) => TypeCheckResult::Ok(Typed::Atom(Atom::Int {
                        id: term.id.clone(),
                        op: *op,
                        value: *n,
                    })),
                    _ => TypeCheckResult::Finding(
                        Finding::new(
                            codes::GUARD_KIND_MISMATCH,
                            ctx.file,
                            format!(
                                "{} (int) cannot be compared with {}",
                                term.id,
                                literal_kind_phrase(value)
                            ),
                        )
                        .at(line)
                        .feature(ctx.feature),
                    ),
                },
            }
        }
        Expr::Not(inner) => match type_expr(inner, line, ctx) {
            TypeCheckResult::Ok(t) => TypeCheckResult::Ok(Typed::Not(Box::new(t))),
            other => other,
        },
        Expr::And(items) => {
            let mut typed_items = Vec::new();
            for item in items {
                match type_expr(item, line, ctx) {
                    TypeCheckResult::Ok(t) => typed_items.push(t),
                    other => return other,
                }
            }
            TypeCheckResult::Ok(Typed::And(typed_items))
        }
        Expr::Or(items) => {
            let mut typed_items = Vec::new();
            for item in items {
                match type_expr(item, line, ctx) {
                    TypeCheckResult::Ok(t) => typed_items.push(t),
                    other => return other,
                }
            }
            TypeCheckResult::Ok(Typed::Or(typed_items))
        }
    }
}

fn type_assignment(
    assignment: &Assignment,
    line: usize,
    ctx: &mut Context<'_>,
) -> TypeCheckResult<TypedEffect> {
    if let Literal::Word(w) = &assignment.value
        && let Err(f) = check_word_literal(w, line, ctx)
    {
        return TypeCheckResult::Finding(f);
    }

    let term = match check_term_ref(&assignment.term, line, ctx) {
        TypeCheckResult::Ok(t) => t,
        TypeCheckResult::Finding(f) => return TypeCheckResult::Finding(f),
        TypeCheckResult::SilentExclusion => return TypeCheckResult::SilentExclusion,
    };

    match &term.kind {
        Kind::Entity => TypeCheckResult::Finding(
            Finding::new(
                codes::GUARD_KIND_MISMATCH,
                ctx.file,
                format!(
                    "{} (entity) cannot be assigned {}",
                    term.id,
                    literal_kind_phrase(&assignment.value)
                ),
            )
            .at(line)
            .feature(ctx.feature),
        ),
        Kind::Bool => match &assignment.value {
            Literal::Bool(b) => TypeCheckResult::Ok(TypedEffect {
                id: term.id.clone(),
                value: Value::Bool(*b),
            }),
            _ => TypeCheckResult::Finding(
                Finding::new(
                    codes::GUARD_KIND_MISMATCH,
                    ctx.file,
                    format!(
                        "{} (bool) cannot be compared with {}",
                        term.id,
                        literal_kind_phrase(&assignment.value)
                    ),
                )
                .at(line)
                .feature(ctx.feature),
            ),
        },
        Kind::Enum { values } => match &assignment.value {
            Literal::Word(w) => {
                if !values.contains(w) {
                    TypeCheckResult::Finding(
                        Finding::new(
                            codes::GUARD_KIND_MISMATCH,
                            ctx.file,
                            format!(
                                "{} (enum) value '{}' is not in declared values: {}",
                                term.id,
                                w,
                                values.join(", ")
                            ),
                        )
                        .at(line)
                        .feature(ctx.feature),
                    )
                } else {
                    TypeCheckResult::Ok(TypedEffect {
                        id: term.id.clone(),
                        value: Value::Enum(w.clone()),
                    })
                }
            }
            _ => TypeCheckResult::Finding(
                Finding::new(
                    codes::GUARD_KIND_MISMATCH,
                    ctx.file,
                    format!(
                        "{} (enum) cannot be compared with {}",
                        term.id,
                        literal_kind_phrase(&assignment.value)
                    ),
                )
                .at(line)
                .feature(ctx.feature),
            ),
        },
        Kind::Int { .. } => match &assignment.value {
            Literal::Int(n) => TypeCheckResult::Ok(TypedEffect {
                id: term.id.clone(),
                value: Value::Region {
                    low: Some(*n),
                    high: Some(*n),
                },
            }),
            _ => TypeCheckResult::Finding(
                Finding::new(
                    codes::GUARD_KIND_MISMATCH,
                    ctx.file,
                    format!(
                        "{} (int) cannot be compared with {}",
                        term.id,
                        literal_kind_phrase(&assignment.value)
                    ),
                )
                .at(line)
                .feature(ctx.feature),
            ),
        },
    }
}

/// Finitizes a requirements document into typed guards, domains, and conflict sets.
#[must_use]
pub fn finitize(
    doc: &RequirementsDoc,
    annotated: &Annotated<'_>,
    scope: &TermScope,
    feature: &str,
    file: &str,
) -> Finitized {
    let mut findings = Vec::new();
    let mut excluded: usize = 0;
    let mut active_terms = BTreeMap::new();
    let mut prepared = Vec::new();

    // 1. Misplaced annotations on requirements
    for req in &annotated.requirements {
        for misplaced in &req.misplaced {
            findings.push(
                Finding::new(
                    codes::GUARD_UNPARSED,
                    file,
                    format!(
                        "misplaced {} belongs indented under a criterion",
                        key_display(&misplaced.key)
                    ),
                )
                .at(misplaced.line)
                .feature(feature),
            );
        }
    }

    // 2. Process criteria with annotations
    for req in &annotated.requirements {
        for crit in &req.criteria {
            let has_annotations = crit.guard.is_some()
                || crit.effect.is_some()
                || crit.conflicts.is_some()
                || !crit.duplicates.is_empty();

            if !has_annotations {
                continue;
            }

            let mut crit_excluded = false;
            let mut guard_findings = Vec::new();

            // Check duplicates
            for dup in &crit.duplicates {
                guard_findings.push(
                    Finding::new(
                        codes::GUARD_UNPARSED,
                        file,
                        format!("second {} on criterion", key_display(&dup.key)),
                    )
                    .at(dup.line)
                    .feature(feature),
                );
                crit_excluded = true;
            }

            let mut typed_guard = Typed::True;
            let mut typed_effects = Vec::new();
            let mut accepted_conflicts = Vec::new();

            let mut ctx = Context {
                scope,
                req_id: &req.requirement.id,
                crit_id: &crit.criterion.id,
                file,
                feature,
                active_terms: &mut active_terms,
            };

            // Process guard
            if let Some(guard_ann) = crit.guard {
                match guard::parse_guard(&guard_ann.value) {
                    Err(ParseError { message, position }) => {
                        guard_findings.push(
                            Finding::new(
                                codes::GUARD_UNPARSED,
                                file,
                                format!("{message} at position {position}"),
                            )
                            .at(guard_ann.line)
                            .feature(feature),
                        );
                        crit_excluded = true;
                    }
                    Ok(expr) => match type_expr(&expr, guard_ann.line, &mut ctx) {
                        TypeCheckResult::Ok(t) => {
                            typed_guard = t;
                        }
                        TypeCheckResult::Finding(f) => {
                            guard_findings.push(f);
                            crit_excluded = true;
                        }
                        TypeCheckResult::SilentExclusion => {
                            crit_excluded = true;
                        }
                    },
                }
            }

            // Process effect
            if let Some(eff_ann) = crit.effect {
                match guard::parse_effect(&eff_ann.value) {
                    Err(ParseError { message, position }) => {
                        guard_findings.push(
                            Finding::new(
                                codes::GUARD_UNPARSED,
                                file,
                                format!("{message} at position {position}"),
                            )
                            .at(eff_ann.line)
                            .feature(feature),
                        );
                        crit_excluded = true;
                    }
                    Ok(assignments) => {
                        for a in &assignments {
                            match type_assignment(a, eff_ann.line, &mut ctx) {
                                TypeCheckResult::Ok(eff) => {
                                    typed_effects.push(eff);
                                }
                                TypeCheckResult::Finding(f) => {
                                    guard_findings.push(f);
                                    crit_excluded = true;
                                }
                                TypeCheckResult::SilentExclusion => {
                                    crit_excluded = true;
                                }
                            }
                        }
                    }
                }
            }

            // Process conflicts (only if criterion itself is not excluded by guard/effects)
            if let Some(conflicts) = &crit.conflicts {
                for cid in &conflicts.ids {
                    if doc.criterion(cid).is_some() {
                        accepted_conflicts.push(cid.clone());
                    } else {
                        findings.push(
                            Finding::new(codes::CONFLICT_REF_UNKNOWN, file, cid)
                                .at(conflicts.line)
                                .feature(feature),
                        );
                    }
                }
            }

            if crit_excluded {
                excluded = excluded.saturating_add(1);
                findings.extend(guard_findings);
            } else {
                prepared.push(Prepared {
                    id: crit.criterion.id.clone(),
                    line: crit.criterion.line,
                    guard_line: crit.guard.map(|g| g.line),
                    guard: typed_guard,
                    effects: typed_effects,
                    conflicts: accepted_conflicts,
                });
            }
        }
    }

    // 3. Criteria with no annotation at all that are named by an accepted _Conflicts:_ list
    let mut conflict_targets = BTreeSet::new();
    for p in &prepared {
        for cid in &p.conflicts {
            conflict_targets.insert(cid.clone());
        }
    }

    let existing_ids: BTreeSet<String> = prepared.iter().map(|p| p.id.clone()).collect();
    for cid in conflict_targets {
        if !existing_ids.contains(&cid)
            && let Some(target) = doc.criterion(&cid)
        {
            // If it had no annotations in Annotated (i.e. truly unannotated), prepare as unconditional
            let is_unannotated = annotated
                .requirements
                .iter()
                .flat_map(|r| r.criteria.iter())
                .find(|c| c.criterion.id == cid)
                .is_none_or(|c| {
                    c.guard.is_none()
                        && c.effect.is_none()
                        && c.conflicts.is_none()
                        && c.duplicates.is_empty()
                });

            if is_unannotated {
                prepared.push(Prepared {
                    id: target.id.clone(),
                    line: target.line,
                    guard_line: None,
                    guard: Typed::True,
                    effects: Vec::new(),
                    conflicts: Vec::new(),
                });
            }
        }
    }

    // Keep prepared criteria in document line order
    prepared.sort_by_key(|p| p.line);

    // 4. Domains for all terms used across accepted criteria
    let mut used_term_ids = BTreeSet::new();
    for p in &prepared {
        for id in p.guard.ids() {
            used_term_ids.insert(id);
        }
        for eff in &p.effects {
            used_term_ids.insert(eff.id.clone());
        }
    }

    let mut term_list: Vec<&TermInfo> = used_term_ids
        .iter()
        .filter_map(|id| active_terms.get(id))
        .collect();

    // Ordered by (topic, id)
    term_list.sort_by(|a, b| (&a.topic, &a.id).cmp(&(&b.topic, &b.id)));

    let mut domains = Vec::new();
    for term in term_list {
        let values = match &term.kind {
            Kind::Entity => Vec::new(),
            Kind::Bool => vec![Value::Bool(false), Value::Bool(true)],
            Kind::Enum { values } => values.iter().cloned().map(Value::Enum).collect(),
            Kind::Int { min, max } => {
                let mut constants = Vec::new();
                for p in &prepared {
                    collect_guard_int_constants(&p.guard, &term.id, &mut constants);
                    for eff in &p.effects {
                        if eff.id == term.id
                            && let Value::Region { low: Some(c), .. } = eff.value
                        {
                            constants.push(c);
                        }
                    }
                }
                regions(&constants, *min, *max)
            }
        };

        domains.push(Domain {
            id: term.id.clone(),
            topic: term.topic.clone(),
            values,
        });
    }

    Finitized {
        criteria: prepared,
        domains,
        findings,
        excluded,
    }
}

fn collect_guard_int_constants(guard: &Typed, term_id: &str, out: &mut Vec<i64>) {
    match guard {
        Typed::True => {}
        Typed::Atom(Atom::Int { id, value, .. }) if id == term_id => {
            out.push(*value);
        }
        Typed::Atom(_) => {}
        Typed::Not(inner) => collect_guard_int_constants(inner, term_id, out),
        Typed::And(items) | Typed::Or(items) => {
            for item in items {
                collect_guard_int_constants(item, term_id, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::annotations::collect;
    use crate::checks::terms::resolve_scope;
    use crate::requirements::parse;
    use crate::vocab::parse as parse_vocab;

    #[test]
    fn regions_with_and_without_bounds() {
        // Without bounds, two constants
        let r1 = regions(&[500, 1000], None, None);
        assert_eq!(r1.len(), 5);
        assert_eq!(
            r1.first(),
            Some(&Value::Region {
                low: None,
                high: Some(499)
            })
        );
        assert_eq!(
            r1.get(1),
            Some(&Value::Region {
                low: Some(500),
                high: Some(500)
            })
        );
        assert_eq!(
            r1.get(2),
            Some(&Value::Region {
                low: Some(501),
                high: Some(999)
            })
        );
        assert_eq!(
            r1.get(3),
            Some(&Value::Region {
                low: Some(1000),
                high: Some(1000)
            })
        );
        assert_eq!(
            r1.get(4),
            Some(&Value::Region {
                low: Some(1001),
                high: None
            })
        );

        // With min = 0, no max
        let r2 = regions(&[500, 1000], Some(0), None);
        assert_eq!(r2.len(), 5);
        assert_eq!(
            r2.first(),
            Some(&Value::Region {
                low: Some(0),
                high: Some(499)
            })
        );
        assert_eq!(
            r2.first().map(|v| v.display("depth")),
            Some("depth in 0..=499".to_string())
        );
        assert_eq!(
            r2.get(1).map(|v| v.display("depth")),
            Some("depth = 500".to_string())
        );
        assert_eq!(
            r2.get(4).map(|v| v.display("depth")),
            Some("depth >= 1001".to_string())
        );

        // Empty constants with bounds
        let r3 = regions(&[], Some(10), Some(20));
        assert_eq!(
            r3,
            vec![Value::Region {
                low: Some(10),
                high: Some(20)
            }]
        );

        // Constant equal to min
        let r4 = regions(&[0], Some(0), None);
        assert_eq!(r4.len(), 2);
        assert_eq!(
            r4.first(),
            Some(&Value::Region {
                low: Some(0),
                high: Some(0)
            })
        );
        assert_eq!(
            r4.get(1),
            Some(&Value::Region {
                low: Some(1),
                high: None
            })
        );
    }

    #[test]
    fn value_display_formats() {
        assert_eq!(Value::Bool(true).display("armed"), "armed = true");
        assert_eq!(Value::Enum("safe".into()).display("mode"), "mode = safe");
        assert_eq!(
            Value::Region {
                low: Some(1000),
                high: Some(1000)
            }
            .display("depth"),
            "depth = 1000"
        );
        assert_eq!(
            Value::Region {
                low: Some(1001),
                high: Some(4999)
            }
            .display("depth"),
            "depth in 1001..=4999"
        );
        assert_eq!(
            Value::Region {
                low: None,
                high: Some(999)
            }
            .display("depth"),
            "depth <= 999"
        );
        assert_eq!(
            Value::Region {
                low: Some(5001),
                high: None
            }
            .display("depth"),
            "depth >= 5001"
        );
    }

    #[test]
    fn typed_eval_oracle() {
        let mut state = BTreeMap::new();
        state.insert("armed".to_string(), Value::Bool(true));
        state.insert("mode".to_string(), Value::Enum("safe".to_string()));
        state.insert(
            "depth".to_string(),
            Value::Region {
                low: Some(1001),
                high: None,
            },
        );

        let t1 = Typed::Atom(Atom::Bool {
            id: "armed".to_string(),
            value: true,
        });
        assert!(t1.eval(&state));

        let t2 = Typed::Atom(Atom::Enum {
            id: "mode".to_string(),
            equal: true,
            value: "safe".to_string(),
        });
        assert!(t2.eval(&state));

        let t3 = Typed::Atom(Atom::Int {
            id: "depth".to_string(),
            op: Op::Gt,
            value: 1000,
        });
        assert!(t3.eval(&state));

        let t4 = Typed::And(vec![t1, t2, t3]);
        assert!(t4.eval(&state));

        let t5 = Typed::Not(Box::new(t4));
        assert!(!t5.eval(&state));
    }

    #[test]
    fn finitize_exercises_all_finding_codes_and_kind_checks() {
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

[terms.operator]
topic = "navigation"
label = "Operator"
kind = "entity"
definition = "Operator."
"#;
        let vocab = parse_vocab(vocab_toml, "vocabulary.toml").vocabulary;
        let topics = vec!["navigation".to_string(), "telemetry".to_string()];

        let doc_text = r#"
# Requirements Document

## Requirements

### Requirement 1: Kind and syntax checks
_Terms: mode, depth, armed, operator_
_Guard: mode = safe_

#### Acceptance Criteria

1. The controller shall reject syntax error.
   _Guard: mode ==_

2. The controller shall reject term-to-term comparison.
   _Guard: mode = armed_

3. The controller shall reject duplicate guards.
   _Guard: mode = safe_
   _Guard: mode = normal_

4. The controller shall reject undeclared terms.
   _Guard: speed > 10_

5. The controller shall reject entity comparison.
   _Guard: operator = true_

6. The controller shall accept clean criterion.
   _Guard: mode = safe and depth > 1000_
   _Effect: armed = true_
   _Conflicts: 1.7, 99.9_

7. The controller shall serve as partner.
"#;
        let doc = parse(doc_text, "test.md");
        let ann = collect(&doc);
        let scope = resolve_scope(&ann, &vocab, &topics);

        let fin = finitize(&doc, &ann, &scope, "test", "test.md");

        // Misplaced guard on requirement 1
        assert!(fin.findings.iter().any(|f| {
            f.code == codes::GUARD_UNPARSED
                && f.message == "misplaced _Guard:_ belongs indented under a criterion"
        }));

        // 1.1 syntax error
        assert!(
            fin.findings
                .iter()
                .any(|f| { f.code == codes::GUARD_UNPARSED && f.message.contains("at position") })
        );

        // 1.2 term-to-term comparison
        assert!(fin.findings.iter().any(|f| {
            f.code == codes::GUARD_UNPARSED
                && f.message == "armed is a term; comparisons are against literal constants only"
        }));

        // 1.3 duplicate guard
        assert!(fin.findings.iter().any(|f| {
            f.code == codes::GUARD_UNPARSED && f.message == "second _Guard:_ on criterion"
        }));

        // 1.4 undeclared term
        assert!(
            fin.findings
                .iter()
                .any(|f| { f.code == codes::GUARD_TERM_UNDECLARED && f.message == "speed" })
        );

        // 1.5 entity kind mismatch
        assert!(fin.findings.iter().any(|f| {
            f.code == codes::GUARD_KIND_MISMATCH
                && f.message == "operator (entity) cannot be compared with a boolean"
        }));

        // 1.6 unknown conflict id 99.9
        assert!(
            fin.findings
                .iter()
                .any(|f| { f.code == codes::CONFLICT_REF_UNKNOWN && f.message == "99.9" })
        );

        // Exactly 5 criteria excluded (1.1, 1.2, 1.3, 1.4, 1.5)
        assert_eq!(fin.excluded, 5);

        // Accepted criteria: 1.6 and 1.7 (unconditional partner from conflict)
        assert_eq!(fin.criteria.len(), 2);
        assert_eq!(fin.criteria.first().map(|c| c.id.as_str()), Some("1.6"));
        assert_eq!(fin.criteria.get(1).map(|c| c.id.as_str()), Some("1.7"));
        assert_eq!(fin.criteria.get(1).map(|c| &c.guard), Some(&Typed::True));

        // Domains in (topic, id) order: navigation comes before telemetry
        // navigation: depth, mode; telemetry: armed
        assert_eq!(fin.domains.len(), 3);
        assert_eq!(fin.domains.first().map(|d| d.id.as_str()), Some("depth"));
        assert_eq!(fin.domains.get(1).map(|d| d.id.as_str()), Some("mode"));
        assert_eq!(fin.domains.get(2).map(|d| d.id.as_str()), Some("armed"));
    }
}
