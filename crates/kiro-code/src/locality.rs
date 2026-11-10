//! Module-level bindings and literal comparisons of a file
//! (code-locality-data 1.1 and 2.1), taken from the shared syntax tree.
//!
//! Per language, a small table says which node kinds are a module-level
//! binding, a comparison, a string literal and a match. Nothing is
//! evaluated and no name is resolved: a binding is its name and the text
//! of its initializer, a comparison is the text of its subject and the
//! text of the literal.

use ast_grep_core::Node;
use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_language::SupportLang;
use kiro_core::code::locality::{Binding, Comparison};

type SgNode<'r> = Node<'r, StrDoc<SupportLang>>;

/// Longest initializer taken as a definition, in lines.
const MAX_INITIALIZER_LINES: usize = 12;
/// Longest subject text taken as a state subject.
const MAX_SUBJECT_CHARS: usize = 40;

struct Table {
    /// Kinds whose children are module level.
    module_parents: &'static [&'static str],
    /// Wrappers between the module and a binding (`export`, an expression statement).
    binding_wrappers: &'static [&'static str],
    /// Binding kinds with a `name` (or `left`) and a `value` (or `right`) field.
    bindings: &'static [&'static str],
    name_fields: &'static [&'static str],
    value_fields: &'static [&'static str],
    comparisons: &'static [&'static str],
    operators: &'static [&'static str],
    strings: &'static [&'static str],
    /// Literal collections on the right of a membership test.
    collections: &'static [&'static str],
    /// (match kind, subject field, arm kind).
    matches: &'static [(&'static str, &'static str, &'static str)],
    /// Whether `A::B` paths with a capitalized last segment are literals.
    enum_paths: bool,
}

const RUST: Table = Table {
    module_parents: &["source_file", "declaration_list"],
    binding_wrappers: &[],
    bindings: &["const_item", "static_item"],
    name_fields: &["name"],
    value_fields: &["value"],
    comparisons: &["binary_expression"],
    operators: &["==", "!="],
    strings: &["string_literal"],
    collections: &[],
    matches: &[("match_expression", "value", "match_arm")],
    enum_paths: true,
};

const PYTHON: Table = Table {
    module_parents: &["module"],
    binding_wrappers: &["expression_statement"],
    bindings: &["assignment"],
    name_fields: &["left"],
    value_fields: &["right"],
    comparisons: &["comparison_operator"],
    operators: &["==", "!=", "in", "not in"],
    strings: &["string"],
    collections: &["tuple", "list", "set"],
    matches: &[("match_statement", "subject", "case_clause")],
    enum_paths: false,
};

const SCRIPT: Table = Table {
    module_parents: &["program"],
    binding_wrappers: &[
        "export_statement",
        "lexical_declaration",
        "variable_declaration",
    ],
    bindings: &["variable_declarator"],
    name_fields: &["name"],
    value_fields: &["value"],
    comparisons: &["binary_expression"],
    operators: &["===", "!==", "==", "!="],
    strings: &["string"],
    collections: &[],
    matches: &[("switch_statement", "value", "switch_case")],
    enum_paths: false,
};

fn table(profile: &str) -> Option<&'static Table> {
    match profile {
        "rust" => Some(&RUST),
        "python" => Some(&PYTHON),
        "typescript" | "tsx" | "javascript" => Some(&SCRIPT),
        _ => None,
    }
}

fn has(list: &[&str], kind: &str) -> bool {
    list.contains(&kind)
}

fn line(node: &SgNode<'_>) -> u32 {
    u32::try_from(node.start_pos().line().saturating_add(1)).unwrap_or(u32::MAX)
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn field<'r>(node: &SgNode<'r>, names: &[&str]) -> Option<SgNode<'r>> {
    names.iter().find_map(|name| node.field(name))
}

/// Whether a node stands at module level, looking through the wrappers.
fn at_module_level(node: &SgNode<'_>, table: &Table) -> bool {
    let mut current = node.parent();
    while let Some(parent) = current {
        let kind = parent.kind();
        if has(table.module_parents, &kind) {
            // A Rust `declaration_list` is module level only inside `mod`.
            return kind != "declaration_list"
                || parent
                    .parent()
                    .is_some_and(|outer| outer.kind() == "mod_item");
        }
        if !has(table.binding_wrappers, &kind) {
            return false;
        }
        current = parent.parent();
    }
    false
}

fn is_enum_path(node: &SgNode<'_>) -> bool {
    node.kind() == "scoped_identifier"
        && node
            .text()
            .rsplit("::")
            .next()
            .and_then(|last| last.chars().next())
            .is_some_and(char::is_uppercase)
}

/// The literals a node consists of: itself, or the members of a literal
/// collection or alternative pattern. Empty when anything else is inside.
fn literals(node: &SgNode<'_>, table: &Table) -> Vec<String> {
    let kind = node.kind();
    if has(table.strings, &kind) || (table.enum_paths && is_enum_path(node)) {
        return vec![node.text().to_string()];
    }
    let wraps = has(table.collections, &kind)
        || matches!(
            kind.as_ref(),
            "parenthesized_expression"
                | "match_pattern"
                | "or_pattern"
                | "case_pattern"
                | "union_pattern"
        );
    if !wraps {
        return Vec::new();
    }
    let mut out = Vec::new();
    for child in node.children().filter(|child| child.is_named()) {
        let inner = literals(&child, table);
        if inner.is_empty() {
            return Vec::new();
        }
        out.extend(inner);
    }
    out
}

fn subject_text(node: &SgNode<'_>) -> Option<String> {
    let text: String = node.text().split_whitespace().collect();
    let text = text
        .strip_prefix('(')
        .and_then(|inner| inner.strip_suffix(')'))
        .map_or(text.clone(), str::to_string);
    // The receiver is a different thing in every type: `match self` in two
    // enums is not one subject.
    let receiver = matches!(text.as_str(), "self" | "*self" | "&self" | "this");
    (!receiver && !text.is_empty() && text.chars().count() <= MAX_SUBJECT_CHARS).then_some(text)
}

fn comparison(node: &SgNode<'_>, table: &Table, path: &str, out: &mut Vec<Comparison>) {
    if !node.children().any(|child| {
        has(table.operators, child.text().as_ref()) || has(table.operators, &child.kind())
    }) {
        return;
    }
    let named: Vec<SgNode<'_>> = node.children().filter(|child| child.is_named()).collect();
    let (Some(left), Some(right)) = (named.first(), named.last()) else {
        return;
    };
    if named.len() != 2 {
        return;
    }
    let (left_literals, right_literals) = (literals(left, table), literals(right, table));
    let (subject, found) = match (left_literals.is_empty(), right_literals.is_empty()) {
        (true, false) => (left, right_literals),
        (false, true) => (right, left_literals),
        _ => return,
    };
    let Some(subject) = subject_text(subject) else {
        return;
    };
    for literal in found {
        out.push(Comparison {
            path: path.to_string(),
            line: line(node),
            subject: subject.clone(),
            literal,
        });
    }
}

fn match_arms(
    node: &SgNode<'_>,
    subject_field: &str,
    arm_kind: &str,
    table: &Table,
    path: &str,
    out: &mut Vec<Comparison>,
) {
    let Some(subject) = node
        .field(subject_field)
        .and_then(|value| subject_text(&value))
    else {
        return;
    };
    let mut stack: Vec<SgNode<'_>> = node.children().collect();
    while let Some(child) = stack.pop() {
        if child.kind() == arm_kind {
            let pattern = child
                .field("pattern")
                .or_else(|| child.field("value"))
                .or_else(|| child.children().find(|inner| inner.is_named()));
            if let Some(pattern) = pattern {
                for literal in literals(&pattern, table) {
                    out.push(Comparison {
                        path: path.to_string(),
                        line: line(&child),
                        subject: subject.clone(),
                        literal,
                    });
                }
            }
        } else if child.kind() != node.kind() {
            // Arms sit one or two levels down; a nested match is its own.
            stack.extend(child.children());
        }
    }
}

/// Collects the bindings and comparisons of one parsed file.
pub fn extract(
    root: &SgNode<'_>,
    profile: &str,
    path: &str,
    bindings: &mut Vec<Binding>,
    comparisons: &mut Vec<Comparison>,
) {
    let Some(table) = table(profile) else {
        return;
    };
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        if has(table.bindings, &kind) && at_module_level(&node, table) {
            let name = field(&node, table.name_fields);
            let value = field(&node, table.value_fields);
            if let (Some(name), Some(value)) = (name, value) {
                let span = value
                    .end_pos()
                    .line()
                    .saturating_sub(value.start_pos().line());
                // Only a plain name is a definition; a destructuring is not.
                let plain = name.children().next().is_none();
                if plain && span < MAX_INITIALIZER_LINES {
                    bindings.push(Binding {
                        path: path.to_string(),
                        line: line(&node),
                        name: name.text().to_string(),
                        initializer: collapsed(&value.text()),
                    });
                }
            }
        }
        if has(table.comparisons, &kind) {
            comparison(&node, table, path, comparisons);
        }
        for (match_kind, subject_field, arm_kind) in table.matches {
            if kind == *match_kind {
                match_arms(&node, subject_field, arm_kind, table, path, comparisons);
            }
        }
        stack.extend(node.children());
    }
}
