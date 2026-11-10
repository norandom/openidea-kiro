//! Metric profiles over a library-produced syntax tree (code-constraints 1.1, 3.1, 4.1, 4.2).
//!
//! Parsing is done by `ast-grep-core` and its tree-sitter grammars. This
//! module holds, per language, tables of node kinds and the counting rules
//! over them. It contains no lexer and no parser.
//!
//! Definitions (definition version [`DEFINITION_VERSION`]):
//!
//! - **parameters**: declared parameters, excluding the receiver.
//! - **cyclomatic** (strict): 1, plus one per branching construct, per
//!   match arm beyond the first, and per short-circuit boolean operator.
//! - **nesting**: deepest chain of control structures inside the body; the
//!   body itself is level 0 and an else-if chain stays on one level.
//! - **code_lines**: lines of the routine that hold at least one token
//!   that is not a comment (Python: nor a docstring).
//! - **statements**: Rust and Python: the named, non-comment items of every
//!   block (a Python docstring is a comment); TypeScript and JavaScript:
//!   the statement nodes, and declarations directly in a block.
//!
//! A nested routine is measured as its own entity and contributes nothing
//! to the routine around it. In TypeScript and JavaScript an arrow function
//! or function expression that is bound to a name (the value of a
//! declaration, a class field or an object property) is a routine of that
//! name. Every other closure, lambda or inline callback belongs to the
//! routine that contains it.

use std::collections::{BTreeMap, BTreeSet};

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{AstGrep, Node};
use ast_grep_language::SupportLang;
use kiro_core::code::{
    CodeFacts, Completeness, CoverageCell, EntityKey, Fingerprint, MetricId, Observation,
    ProducerId, Scope, metrics,
};

/// Version of the metric definitions above. A change of meaning changes it.
pub const DEFINITION_VERSION: &str = "1";
/// Producer name written into every observation.
pub const PRODUCER: &str = "kiro-code";
/// Parse stack identity written into every observation.
pub const GRAMMAR: &str = "ast-grep 0.51.1, tree-sitter 0.27";

pub(crate) type SgNode<'r> = Node<'r, StrDoc<SupportLang>>;

enum StatementRule {
    /// Named, non-comment children of these container kinds.
    ChildrenOf(&'static [&'static str]),
    /// Nodes whose kind ends with a suffix, minus the excluded kinds;
    /// declarations count only directly under the listed parents.
    KindSuffix {
        suffix: &'static str,
        excluded: &'static [&'static str],
        declarations: &'static [&'static str],
        declaration_parents: &'static [&'static str],
    },
}

/// The node-kind tables of one language.
pub struct Profile {
    pub name: &'static str,
    lang: SupportLang,
    extensions: &'static [&'static str],
    functions: &'static [&'static str],
    receiver_kinds: &'static [&'static str],
    receiver_names: &'static [&'static str],
    decisions: &'static [&'static str],
    /// Kind whose arms each add a decision beyond the first.
    match_kind: Option<(&'static str, &'static str)>,
    boolean_exprs: &'static [&'static str],
    boolean_ops: &'static [&'static str],
    nesting: &'static [&'static str],
    chain_parents: &'static [&'static str],
    comments: &'static [&'static str],
    statements: StatementRule,
    /// Type declarations counted for a file (aliases are not types).
    types: &'static [&'static str],
    /// Containers whose name qualifies the routines inside them.
    containers: &'static [&'static str],
    /// Field declarations of a type.
    fields: &'static [&'static str],
    /// Method declarations of a type that are not routines with a body.
    method_signatures: &'static [&'static str],
    /// Name separator in qualified names.
    separator: &'static str,
    docstrings: bool,
    /// Function-valued expressions that are routines when they are the value
    /// of a declaration, a class field or an object property; elsewhere they
    /// belong to the routine around them.
    value_functions: &'static [&'static str],
    value_parents: &'static [&'static str],
}

const RUST: Profile = Profile {
    name: "rust",
    lang: SupportLang::Rust,
    extensions: &["rs"],
    functions: &["function_item"],
    receiver_kinds: &["self_parameter"],
    receiver_names: &[],
    decisions: &["if_expression", "while_expression", "for_expression"],
    match_kind: Some(("match_expression", "match_arm")),
    boolean_exprs: &["binary_expression"],
    boolean_ops: &["&&", "||"],
    nesting: &[
        "if_expression",
        "while_expression",
        "for_expression",
        "loop_expression",
        "match_expression",
    ],
    chain_parents: &["else_clause"],
    comments: &["line_comment", "block_comment"],
    statements: StatementRule::ChildrenOf(&["block"]),
    types: &["struct_item", "enum_item", "trait_item", "union_item"],
    containers: &["impl_item", "trait_item", "mod_item"],
    fields: &["field_declaration"],
    method_signatures: &["function_signature_item"],
    separator: "::",
    docstrings: false,
    value_functions: &[],
    value_parents: &[],
};

const PYTHON: Profile = Profile {
    name: "python",
    lang: SupportLang::Python,
    extensions: &["py", "pyi"],
    functions: &["function_definition"],
    // A bare `*` or `/` in a parameter list separates, it declares nothing.
    receiver_kinds: &["keyword_separator", "positional_separator"],
    receiver_names: &["self", "cls"],
    decisions: &[
        "if_statement",
        "elif_clause",
        "while_statement",
        "for_statement",
        "conditional_expression",
        "except_clause",
    ],
    match_kind: None,
    boolean_exprs: &["boolean_operator"],
    boolean_ops: &["and", "or"],
    nesting: &[
        "if_statement",
        "while_statement",
        "for_statement",
        "try_statement",
        "with_statement",
        "match_statement",
    ],
    chain_parents: &[],
    comments: &["comment"],
    statements: StatementRule::ChildrenOf(&["block"]),
    types: &["class_definition"],
    containers: &["class_definition"],
    fields: &[],
    method_signatures: &[],
    separator: ".",
    docstrings: true,
    value_functions: &[],
    value_parents: &[],
};

const TS_FUNCTIONS: &[&str] = &["function_declaration", "method_definition"];
const TS_DECISIONS: &[&str] = &[
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "for_in_statement",
    "switch_case",
    "ternary_expression",
    "catch_clause",
];
const TS_NESTING: &[&str] = &[
    "if_statement",
    "while_statement",
    "do_statement",
    "for_statement",
    "for_in_statement",
    "switch_statement",
    "try_statement",
];
const TS_STATEMENTS: StatementRule = StatementRule::KindSuffix {
    suffix: "_statement",
    excluded: &["statement_block"],
    declarations: &["lexical_declaration", "variable_declaration"],
    declaration_parents: &[
        "statement_block",
        "program",
        "switch_case",
        "switch_default",
    ],
};

const TYPESCRIPT: Profile = Profile {
    name: "typescript",
    lang: SupportLang::TypeScript,
    extensions: &["ts", "mts", "cts"],
    functions: TS_FUNCTIONS,
    receiver_kinds: &[],
    receiver_names: &["this"],
    decisions: TS_DECISIONS,
    match_kind: None,
    boolean_exprs: &["binary_expression"],
    boolean_ops: &["&&", "||", "??"],
    nesting: TS_NESTING,
    chain_parents: &["else_clause"],
    comments: &["comment"],
    statements: TS_STATEMENTS,
    types: &[
        "class_declaration",
        "abstract_class_declaration",
        "interface_declaration",
        "enum_declaration",
    ],
    containers: &[
        "class_declaration",
        "abstract_class_declaration",
        "interface_declaration",
    ],
    fields: &["public_field_definition", "property_signature"],
    method_signatures: &["method_signature", "abstract_method_signature"],
    separator: ".",
    docstrings: false,
    value_functions: &["arrow_function", "function_expression"],
    value_parents: &["variable_declarator", "public_field_definition", "pair"],
};

const TSX: Profile = Profile {
    name: "tsx",
    lang: SupportLang::Tsx,
    extensions: &["tsx"],
    ..TYPESCRIPT
};

const JAVASCRIPT: Profile = Profile {
    name: "javascript",
    lang: SupportLang::JavaScript,
    extensions: &["js", "mjs", "cjs", "jsx"],
    types: &["class_declaration"],
    containers: &["class_declaration"],
    fields: &["field_definition"],
    method_signatures: &[],
    value_parents: &["variable_declarator", "field_definition", "pair"],
    ..TYPESCRIPT
};

/// Every profile of this build.
pub const PROFILES: &[&Profile] = &[&RUST, &PYTHON, &TYPESCRIPT, &TSX, &JAVASCRIPT];

/// The profile that measures a path, chosen by its extension.
#[must_use]
pub fn profile_for(path: &str) -> Option<&'static Profile> {
    let extension = path.rsplit_once('.').map(|(_, extension)| extension)?;
    PROFILES
        .iter()
        .copied()
        .find(|profile| profile.extensions.contains(&extension))
}

/// Whether a node is measured as a routine of its own.
pub(crate) fn is_routine(node: &SgNode<'_>, profile: &Profile) -> bool {
    let kind = node.kind();
    has(profile.functions, &kind)
        || (has(profile.value_functions, &kind)
            && node
                .parent()
                .is_some_and(|parent| has(profile.value_parents, &parent.kind())))
}

/// The name of a routine: its own, or the name it is bound to.
fn routine_name(node: &SgNode<'_>) -> Option<String> {
    if let Some(name) = node.field("name") {
        return Some(name.text().to_string());
    }
    let parent = node.parent()?;
    parent
        .field("name")
        .or_else(|| parent.field("key"))
        .map(|name| name.text().to_string())
}

pub(crate) fn has(list: &[&str], kind: &str) -> bool {
    list.contains(&kind)
}

fn line(index: usize) -> u32 {
    u32::try_from(index.saturating_add(1)).unwrap_or(u32::MAX)
}

fn count(value: usize) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let value = value as f64;
    value
}

/// Whether a node of this kind can hold statements as children.
fn holds_statements(kind: &str, profile: &Profile) -> bool {
    match &profile.statements {
        StatementRule::ChildrenOf(containers) => has(containers, kind),
        StatementRule::KindSuffix {
            declaration_parents,
            ..
        } => has(declaration_parents, kind),
    }
}

/// A node of a routine with the two facts about its place that the
/// measures need. Asking a node for its parent or its previous sibling is
/// slow in the syntax tree library (it walks from the root), so the walk
/// carries both facts down.
struct Walked<'r> {
    node: SgNode<'r>,
    /// The parent can hold statements.
    parent_holds: bool,
    /// The node is the first child of its parent.
    first: bool,
}

/// The children of a node as walked nodes.
fn walked_children<'r>(node: &SgNode<'r>, profile: &Profile) -> Vec<Walked<'r>> {
    let parent_holds = holds_statements(&node.kind(), profile);
    node.children()
        .enumerate()
        .map(|(index, child)| Walked {
            node: child,
            parent_holds,
            first: index == 0,
        })
        .collect()
}

/// Every node below `root` in pre-order, without descending into routines.
fn routine_nodes<'r>(root: &SgNode<'r>, profile: &Profile) -> Vec<Walked<'r>> {
    let mut out = Vec::new();
    let mut stack = walked_children(root, profile);
    while let Some(walked) = stack.pop() {
        if is_routine(&walked.node, profile) {
            continue;
        }
        stack.extend(walked_children(&walked.node, profile));
        out.push(walked);
    }
    out
}

/// First and last line of a node that holds no code. A doc comment node
/// includes its line break, so its end is taken from its text, not from
/// the position after the break.
fn silent_span(node: &SgNode<'_>) -> (usize, usize) {
    let start = node.start_pos().line();
    let text = node.text();
    let breaks = text.trim_end_matches(['\n', '\r']).matches('\n').count();
    (start, start.saturating_add(breaks))
}

/// A Python docstring: a string expression that opens a body. `first`
/// says whether the node is the first child of its parent.
fn is_docstring(node: &SgNode<'_>, profile: &Profile, first: bool) -> bool {
    if !profile.docstrings || !first || node.kind() != "expression_statement" {
        return false;
    }
    let mut named = node.children().filter(|child| child.is_named());
    matches!((named.next(), named.next()), (Some(only), None) if only.kind() == "string")
}

fn parameters(function: &SgNode<'_>, profile: &Profile) -> (usize, String) {
    let Some(list) = function.field("parameters") else {
        // An arrow function may name its single parameter without a list.
        return match function.field("parameter") {
            Some(single) => (1, single.text().to_string()),
            None => (0, String::new()),
        };
    };
    let counted = list
        .children()
        .filter(|parameter| parameter.is_named() && !has(profile.comments, &parameter.kind()))
        .filter(|parameter| !has(profile.receiver_kinds, &parameter.kind()))
        .filter(|parameter| !has(profile.receiver_names, parameter.text().as_ref()))
        .count();
    let text: String = list.text().split_whitespace().collect::<Vec<_>>().join(" ");
    (counted, text)
}

fn nesting_depth(node: &SgNode<'_>, profile: &Profile, depth: usize) -> usize {
    let mut deepest = depth;
    // An else-if chain stays on one level: the chain node opens nothing.
    let chained = has(profile.chain_parents, &node.kind());
    for child in node.children() {
        if is_routine(&child, profile) {
            continue;
        }
        let opens = has(profile.nesting, &child.kind()) && !chained;
        let next = if opens {
            depth.saturating_add(1)
        } else {
            depth
        };
        deepest = deepest.max(nesting_depth(&child, profile, next));
    }
    deepest
}

struct RoutineValues {
    parameters: usize,
    parameter_text: String,
    cyclomatic: usize,
    nesting: usize,
    code_lines: usize,
    statements: usize,
    /// FNV-1a over the code tokens: the routine's normalized body.
    body_hash: u64,
}

/// Whether a walked node counts as a statement.
fn is_statement(walked: &Walked<'_>, profile: &Profile) -> bool {
    let node = &walked.node;
    let kind = node.kind();
    match &profile.statements {
        StatementRule::ChildrenOf(_) => {
            walked.parent_holds
                && node.is_named()
                && !has(profile.comments, &kind)
                && !is_docstring(node, profile, walked.first)
        }
        StatementRule::KindSuffix {
            suffix,
            excluded,
            declarations,
            ..
        } => {
            (kind.ends_with(suffix) && !has(excluded, &kind))
                || (walked.parent_holds && has(declarations, &kind))
        }
    }
}

/// The arms of a match beyond the first; zero for every other node.
fn extra_match_arms(node: &SgNode<'_>, profile: &Profile) -> usize {
    let Some((match_kind, arm_kind)) = profile.match_kind else {
        return 0;
    };
    if node.kind() != match_kind {
        return 0;
    }
    let arms = node
        .children()
        .flat_map(|body| body.children().collect::<Vec<_>>())
        .filter(|arm| arm.kind() == arm_kind)
        .count();
    arms.saturating_sub(1)
}

/// Whether a node is a boolean expression with a short-circuit operator.
fn is_short_circuit(node: &SgNode<'_>, profile: &Profile) -> bool {
    has(profile.boolean_exprs, &node.kind())
        && node
            .children()
            .any(|child| !child.is_named() && has(profile.boolean_ops, &child.kind()))
}

/// What one node adds to the cyclomatic complexity of its routine.
fn decisions_of(node: &SgNode<'_>, profile: &Profile) -> usize {
    let branch = usize::from(has(profile.decisions, &node.kind()));
    let short_circuit = usize::from(is_short_circuit(node, profile));
    branch
        .saturating_add(extra_match_arms(node, profile))
        .saturating_add(short_circuit)
}

/// Whether a node is a code token: a leaf that is no comment and does not
/// lie inside one of the `silent` line ranges.
fn is_code_token(node: &SgNode<'_>, profile: &Profile, silent: &[(usize, usize)]) -> bool {
    if node.children().next().is_some() || has(profile.comments, &node.kind()) {
        return false;
    }
    let (start, end) = (node.start_pos().line(), node.end_pos().line());
    !silent.iter().any(|(from, to)| start >= *from && end <= *to)
}

/// The code tokens of one routine: the lines they stand on and their
/// FNV-1a hash.
struct CodeTokens {
    lines: BTreeSet<usize>,
    hash: u64,
}

impl CodeTokens {
    fn new() -> Self {
        Self {
            lines: BTreeSet::new(),
            hash: 0xcbf2_9ce4_8422_2325,
        }
    }

    fn add(&mut self, token: &SgNode<'_>) {
        self.lines
            .extend(token.start_pos().line()..=token.end_pos().line());
        // The fingerprint covers every code token of the routine,
        // so layout and comments do not change it.
        for byte in token.text().bytes().chain(std::iter::once(0)) {
            self.hash ^= u64::from(byte);
            self.hash = self.hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

fn measure_routine(function: &SgNode<'_>, profile: &Profile) -> RoutineValues {
    let nodes = routine_nodes(function, profile);
    // Line ranges that hold no code: comments and docstrings.
    let silent: Vec<(usize, usize)> = nodes
        .iter()
        .filter(|walked| {
            has(profile.comments, &walked.node.kind())
                || is_docstring(&walked.node, profile, walked.first)
        })
        .map(|walked| silent_span(&walked.node))
        .collect();
    let mut cyclomatic: usize = 1;
    let mut statements: usize = 0;
    let mut tokens = CodeTokens::new();
    for walked in &nodes {
        let node = &walked.node;
        cyclomatic = cyclomatic.saturating_add(decisions_of(node, profile));
        if is_statement(walked, profile) {
            statements = statements.saturating_add(1);
        }
        if is_code_token(node, profile, &silent) {
            tokens.add(node);
        }
    }
    let (parameters, parameter_text) = parameters(function, profile);
    RoutineValues {
        parameters,
        parameter_text,
        cyclomatic,
        nesting: nesting_depth(function, profile, 0),
        code_lines: tokens.lines.len(),
        statements,
        body_hash: tokens.hash,
    }
}

/// The name a container contributes to qualified names.
fn container_name(node: &SgNode<'_>) -> Option<String> {
    // Rust `impl Type` and `impl Trait for Type` name the type.
    let named = node.field("type").or_else(|| node.field("name"))?;
    let text = named.text();
    // `Box<T>` qualifies as `Box`.
    let base = text.split(['<', ' ']).next().unwrap_or("");
    if base.is_empty() {
        None
    } else {
        Some(base.to_string())
    }
}

pub(crate) fn qualified(node: &SgNode<'_>, profile: &Profile) -> String {
    let own = if is_routine(node, profile) {
        routine_name(node)
    } else {
        node.field("name").map(|name| name.text().to_string())
    }
    .unwrap_or_default();
    qualified_as(node, own, profile)
}

/// `own` qualified by the names of the containers and routines around `node`.
fn qualified_as(node: &SgNode<'_>, own: String, profile: &Profile) -> String {
    let mut parts = vec![own];
    let mut current = node.parent();
    while let Some(parent) = current {
        if (has(profile.containers, &parent.kind()) || is_routine(&parent, profile))
            && let Some(name) = if is_routine(&parent, profile) {
                routine_name(&parent)
            } else {
                container_name(&parent)
            }
        {
            parts.push(name);
        }
        current = parent.parent();
    }
    parts.reverse();
    parts.join(profile.separator)
}

/// Lines of a file that hold at least one token that is not a comment.
fn file_code_lines(root: &SgNode<'_>, profile: &Profile) -> usize {
    let mut lines: BTreeSet<usize> = BTreeSet::new();
    let mut silent: Vec<(usize, usize)> = Vec::new();
    let mut leaves: Vec<(usize, usize)> = Vec::new();
    // Each node with whether it is the first child of its parent.
    let mut stack = vec![(root.clone(), true)];
    while let Some((node, first)) = stack.pop() {
        let kind = node.kind();
        if has(profile.comments, &kind) || is_docstring(&node, profile, first) {
            silent.push(silent_span(&node));
            continue;
        }
        let mut children = node.children().enumerate().peekable();
        if children.peek().is_none() {
            leaves.push((node.start_pos().line(), node.end_pos().line()));
        }
        stack.extend(children.map(|(index, child)| (child, index == 0)));
    }
    for (start, end) in leaves {
        // A string spanning comment-free lines is code on each of them.
        if !silent.iter().any(|(from, to)| start >= *from && end <= *to) {
            lines.extend(start..=end);
        }
    }
    lines.len()
}

struct Emitter<'a> {
    path: &'a str,
    producer: ProducerId,
    ordinals: BTreeMap<(Scope, String, String), u32>,
    observations: Vec<Observation>,
    fingerprints: Vec<Fingerprint>,
}

impl<'a> Emitter<'a> {
    fn new(path: &'a str, profile: &Profile) -> Self {
        Self {
            path,
            producer: ProducerId {
                name: PRODUCER.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                grammar: format!("{GRAMMAR}, {}", profile.name),
            },
            ordinals: BTreeMap::new(),
            observations: Vec::new(),
            fingerprints: Vec::new(),
        }
    }

    fn key(&mut self, scope: Scope, name: &str, parameters: &str) -> EntityKey {
        let slot = self
            .ordinals
            .entry((scope, name.to_string(), parameters.to_string()))
            .or_insert(0);
        let ordinal = *slot;
        *slot = slot.saturating_add(1);
        EntityKey {
            scope,
            path: self.path.to_string(),
            qualified_name: name.to_string(),
            parameters: parameters.to_string(),
            ordinal,
        }
    }

    fn emit(&mut self, entity: &EntityKey, metric: &str, value: f64, at: u32) {
        self.observations.push(Observation {
            entity: entity.clone(),
            metric: MetricId {
                name: metric.to_string(),
                definition_version: DEFINITION_VERSION.to_string(),
            },
            value,
            producer: self.producer.clone(),
            line: at,
        });
    }

    /// Emits the observations and the fingerprint of one routine and
    /// returns its cyclomatic complexity.
    fn routine(&mut self, node: &SgNode<'_>, profile: &Profile) -> usize {
        let values = measure_routine(node, profile);
        let name = qualified(node, profile);
        let at = line(node.start_pos().line());
        let entity = self.key(Scope::Routine, &name, &values.parameter_text);
        self.emit(&entity, metrics::PARAMETERS, count(values.parameters), at);
        self.emit(&entity, metrics::CYCLOMATIC, count(values.cyclomatic), at);
        self.emit(&entity, metrics::NESTING, count(values.nesting), at);
        self.emit(&entity, metrics::CODE_LINES, count(values.code_lines), at);
        self.emit(&entity, metrics::STATEMENTS, count(values.statements), at);
        self.fingerprints.push(Fingerprint {
            entity: entity.clone(),
            hash: format!("{:016x}", values.body_hash),
        });
        values.cyclomatic
    }

    /// Emits the member counts of every type declared in this file.
    fn types(&mut self, members: &BTreeMap<String, Members>, profile: &Profile) {
        for (name, counted) in members {
            // Members of a type declared elsewhere (an impl for a foreign type)
            // are not attributed to a type entity of this file.
            if counted.declared_at == 0 {
                continue;
            }
            let at = counted.declared_at;
            let entity = self.key(Scope::Type, name, "");
            self.emit(&entity, metrics::METHODS, count(counted.methods), at);
            if !profile.fields.is_empty() {
                self.emit(&entity, metrics::FIELDS, count(counted.fields), at);
            }
        }
    }

    /// Emits the observations of the file itself.
    fn file(&mut self, tally: &FileTally, code_lines: usize) {
        let path = self.path;
        let file = self.key(Scope::File, path, "");
        self.emit(&file, metrics::CODE_LINES, count(code_lines), 1);
        self.emit(&file, metrics::FUNCTIONS, count(tally.functions), 1);
        self.emit(&file, metrics::TYPES, count(tally.types), 1);
        if tally.functions > 0 {
            self.emit(
                &file,
                metrics::MAX_CYCLOMATIC,
                count(tally.max_cyclomatic),
                1,
            );
        }
    }
}

/// Methods and fields of one type name. `declared_at` is the line of the
/// type's declaration, or 0 while only members were seen.
#[derive(Default)]
struct Members {
    methods: usize,
    fields: usize,
    declared_at: u32,
}

/// What the walk over a file adds up.
#[derive(Default)]
struct FileTally {
    functions: usize,
    types: usize,
    max_cyclomatic: usize,
    /// Members per type name; Rust inherent impls in the same file add
    /// their methods to the type they implement.
    members: BTreeMap<String, Members>,
}

/// Every node of a tree in document order.
pub(crate) fn document_order<'r>(root: &SgNode<'r>) -> Vec<SgNode<'r>> {
    let mut all: Vec<SgNode<'r>> = Vec::new();
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let mut children: Vec<_> = node.children().collect();
        children.reverse();
        stack.extend(children);
        all.push(node);
    }
    all
}

/// The type a container stands for, qualified. A module is no type, and a
/// Rust trait implementation (`impl Trait for Type`) is not counted: only
/// the type's own declaration and its inherent impls are.
fn owning_type(container: &SgNode<'_>, profile: &Profile) -> Option<String> {
    if container.kind() == "mod_item" || container.field("trait").is_some() {
        return None;
    }
    let name = container_name(container)?;
    Some(qualified_as(container, name, profile))
}

/// The type a member belongs to: the nearest container decides, and a
/// routine in between means the node is no member of a type.
fn member_owner(node: &SgNode<'_>, profile: &Profile) -> Option<String> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if is_routine(&parent, profile) {
            return None;
        }
        if has(profile.containers, &parent.kind()) || has(profile.types, &parent.kind()) {
            return owning_type(&parent, profile);
        }
        current = parent.parent();
    }
    None
}

/// Counts a method or a field for the type it belongs to.
fn count_member(node: &SgNode<'_>, profile: &Profile, members: &mut BTreeMap<String, Members>) {
    let kind = node.kind();
    let is_method = is_routine(node, profile) || has(profile.method_signatures, &kind);
    let is_field = has(profile.fields, &kind)
        && !node
            .children()
            .any(|child| has(profile.value_functions, &child.kind()));
    if !is_method && !is_field {
        return;
    }
    let Some(owner) = member_owner(node, profile) else {
        return;
    };
    let entry = members.entry(owner).or_default();
    if is_method {
        entry.methods = entry.methods.saturating_add(1);
    } else {
        entry.fields = entry.fields.saturating_add(1);
    }
}

/// Walks a tree that parsed, emits every routine and adds up the file.
fn measure_tree(root: &SgNode<'_>, profile: &Profile, emitter: &mut Emitter<'_>) -> FileTally {
    let mut tally = FileTally::default();
    // Document order, so ordinals are stable.
    for node in &document_order(root) {
        if is_routine(node, profile) {
            tally.functions = tally.functions.saturating_add(1);
            let cyclomatic = emitter.routine(node, profile);
            tally.max_cyclomatic = tally.max_cyclomatic.max(cyclomatic);
        }
        if has(profile.types, &node.kind()) {
            tally.types = tally.types.saturating_add(1);
            let name = qualified(node, profile);
            tally.members.entry(name).or_default().declared_at = line(node.start_pos().line());
        }
        count_member(node, profile, &mut tally.members);
    }
    tally
}

fn coverage(
    path: &str,
    state: Completeness,
    reason: Option<String>,
    at: Option<u32>,
) -> CoverageCell {
    CoverageCell {
        path: path.to_string(),
        metric: None,
        state,
        reason,
        line: at,
    }
}

/// The first place a tree does not parse: 1-based line.
fn first_error(root: &SgNode<'_>) -> Option<u32> {
    let mut stack = vec![root.clone()];
    let mut first: Option<usize> = None;
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            let at = node.start_pos().line();
            first = Some(first.map_or(at, |current| current.min(at)));
        }
        stack.extend(node.children());
    }
    first.map(line)
}

/// Measures one source text into `facts`.
///
/// A file in a language without a profile gets an `Unavailable` coverage
/// cell and no observation. A file that does not parse gets a `Failed`
/// cell with the first error line and no observation: no rule is judged on
/// a tree with holes.
pub fn measure_into(path: &str, text: &str, facts: &mut CodeFacts) {
    let Some(profile) = profile_for(path) else {
        let reason = "no measurement profile for this file type".to_string();
        facts.coverage.push(coverage(
            path,
            Completeness::Unavailable,
            Some(reason),
            None,
        ));
        return;
    };
    // Line endings are normalized so that a CRLF working tree and LF blobs
    // measure alike.
    let normalized = text.replace("\r\n", "\n");
    let failed =
        |reason: String, at: Option<u32>| coverage(path, Completeness::Failed, Some(reason), at);
    let tree = match StrDoc::try_new(&normalized, profile.lang).map(AstGrep::doc) {
        Ok(tree) => tree,
        Err(error) => {
            facts
                .coverage
                .push(failed(format!("cannot parse: {error}"), None));
            return;
        }
    };
    let root = tree.root();
    if let Some(at) = first_error(&root) {
        facts
            .coverage
            .push(failed(format!("syntax error at line {at}"), Some(at)));
        return;
    }

    let mut emitter = Emitter::new(path, profile);
    let tally = measure_tree(&root, profile, &mut emitter);
    emitter.types(&tally.members, profile);
    emitter.file(&tally, file_code_lines(&root, profile));

    facts.observations.append(&mut emitter.observations);
    facts.fingerprints.append(&mut emitter.fingerprints);
    facts
        .coverage
        .push(coverage(path, Completeness::Complete, None, None));
}

/// The parser language of a path, for analyzers that share the parse stack.
#[must_use]
pub fn language_of(path: &str) -> Option<SupportLang> {
    profile_for(path).map(|profile| profile.lang)
}

/// The comment node kinds of a path's language.
#[must_use]
pub fn comment_kinds(path: &str) -> Option<&'static [&'static str]> {
    profile_for(path).map(|profile| profile.comments)
}

/// The profile name of a path's language, such as `rust` or `python`.
#[must_use]
pub fn profile_name(path: &str) -> Option<&'static str> {
    profile_for(path).map(|profile| profile.name)
}

/// The statements of one routine, as the `statements` metric counts them.
pub(crate) fn statement_count(function: &SgNode<'_>, profile: &Profile) -> usize {
    measure_routine(function, profile).statements
}
