//! Positional access to tabular data (code-locality-data 3.1, 3.5), found
//! in the shared syntax tree by shape.
//!
//! Python:
//! - `x.iloc[rows, 3]`, `x.iat[row, 3]`, `x.iloc[:, [0, 2]]`: a dataframe
//!   column selected by position;
//! - `x.columns[3]`: a dataframe column named by position;
//! - `x[:, 3]`: the column axis of a two-dimensional array;
//! - `row[2]`, `record[2]`, `order_row[2]`: a field of a row record.
//!
//! Rust:
//! - `x.select_at_idx(3)`: a dataframe column selected by position;
//! - `x.column(3)`, `x[[i, 3]]`, `s![.., 3]`: the column axis of a
//!   two-dimensional array.
//!
//! Nothing is known about the type of the receiver; the shape decides. A
//! position held in a variable, an access through a helper and every
//! other receiver are not seen.

use kiro_core::code::data::PositionalAccess;

use crate::measure::{SgNode, document_order};

const DATAFRAME_COLUMN: &str = "a dataframe column";
const ARRAY_COLUMN: &str = "an array column";
const ROW_FIELD: &str = "a row record field";

fn line_of(node: &SgNode<'_>) -> u32 {
    u32::try_from(node.start_pos().line().saturating_add(1)).unwrap_or(u32::MAX)
}

fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text of a node when it is an integer literal, with its sign.
fn integer(node: &SgNode<'_>) -> Option<String> {
    let kind = node.kind();
    if kind == "integer" || kind == "integer_literal" {
        return Some(node.text().to_string());
    }
    if kind != "unary_operator" && kind != "unary_expression" {
        return None;
    }
    let operand = node.children().find(|child| child.is_named())?;
    let text: String = node.text().split_whitespace().collect();
    (text.starts_with('-') && integer(&operand).is_some()).then_some(text)
}

/// An integer literal, or a list of nothing but integer literals.
fn integers(node: &SgNode<'_>) -> Option<String> {
    if let Some(single) = integer(node) {
        return Some(single);
    }
    if node.kind() != "list" {
        return None;
    }
    let items: Vec<SgNode<'_>> = node.children().filter(|child| child.is_named()).collect();
    let all: Option<Vec<String>> = items.iter().map(integer).collect();
    all.filter(|found| !found.is_empty())
        .map(|found| format!("[{}]", found.join(", ")))
}

fn is_row_name(name: &str) -> bool {
    ["row", "record"]
        .iter()
        .any(|word| name == *word || name.ends_with(&format!("_{word}")))
}

/// What a Python subscript addresses by position, if anything.
fn python_subscript(node: &SgNode<'_>) -> Option<(&'static str, String)> {
    let mut named = node.children().filter(|child| child.is_named());
    let value = named.next()?;
    let indices: Vec<SgNode<'_>> = named.collect();
    let attribute = (value.kind() == "attribute")
        .then(|| value.field("attribute"))
        .flatten()
        .map(|name| name.text().to_string());
    match (attribute.as_deref(), indices.as_slice()) {
        (Some("iloc" | "iat"), [_, column]) => integers(column).map(|at| (DATAFRAME_COLUMN, at)),
        (Some("columns"), [only]) => integer(only).map(|at| (DATAFRAME_COLUMN, at)),
        (_, [rows, column]) if rows.kind() == "slice" => {
            integer(column).map(|at| (ARRAY_COLUMN, at))
        }
        (None, [only]) if value.kind() == "identifier" && is_row_name(&value.text()) => {
            integer(only).map(|at| (ROW_FIELD, at))
        }
        _ => None,
    }
}

/// The single integer argument of a Rust method call named `method`.
fn rust_call(node: &SgNode<'_>) -> Option<(&'static str, String)> {
    let function = node.field("function")?;
    let method = function.field("field")?;
    let kind = match method.text().as_ref() {
        "select_at_idx" => DATAFRAME_COLUMN,
        "column" => ARRAY_COLUMN,
        _ => return None,
    };
    let arguments = node.field("arguments")?;
    let mut named = arguments.children().filter(|child| child.is_named());
    let (only, none) = (named.next()?, named.next());
    none.is_none()
        .then(|| integer(&only))
        .flatten()
        .map(|at| (kind, at))
}

/// `x[[i, 3]]`: an index made of two elements with an integer second.
fn rust_index(node: &SgNode<'_>) -> Option<(&'static str, String)> {
    let index = node.children().filter(|child| child.is_named()).nth(1)?;
    if index.kind() != "array_expression" {
        return None;
    }
    let items: Vec<SgNode<'_>> = index.children().filter(|child| child.is_named()).collect();
    match items.as_slice() {
        [_, column] => integer(column).map(|at| (ARRAY_COLUMN, at)),
        _ => None,
    }
}

/// `s![.., 3]`: the slicing macro of `ndarray` with a whole first axis
/// and an integer second.
fn rust_slice_macro(node: &SgNode<'_>) -> Option<(&'static str, String)> {
    if node.field("macro")?.text() != "s" {
        return None;
    }
    let tree = node.children().find(|child| child.kind() == "token_tree")?;
    let text = tree.text();
    let inner = text.trim().strip_prefix('[')?.strip_suffix(']')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    match parts.as_slice() {
        [rows, column] if *rows == ".." && column.parse::<i64>().is_ok() => {
            Some((ARRAY_COLUMN, (*column).to_string()))
        }
        _ => None,
    }
}

fn found(node: &SgNode<'_>, profile: &str) -> Option<(&'static str, String)> {
    let kind = node.kind();
    match (profile, kind.as_ref()) {
        ("python", "subscript") => python_subscript(node),
        ("rust", "call_expression") => rust_call(node),
        ("rust", "index_expression") => rust_index(node),
        ("rust", "macro_invocation") => rust_slice_macro(node),
        _ => None,
    }
}

/// Whether patterns are shipped for a language.
#[must_use]
pub fn covers(profile: &str) -> bool {
    matches!(profile, "python" | "rust")
}

/// Collects the positional accesses of one parsed file.
pub fn extract(root: &SgNode<'_>, profile: &str, path: &str, out: &mut Vec<PositionalAccess>) {
    if !covers(profile) {
        return;
    }
    for node in document_order(root) {
        if let Some((kind, position)) = found(&node, profile) {
            out.push(PositionalAccess {
                path: path.to_string(),
                line: line_of(&node),
                expression: collapsed(&node.text()),
                position,
                kind,
            });
        }
    }
}
