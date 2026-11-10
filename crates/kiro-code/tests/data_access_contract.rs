//! Positional access to tabular data (code-locality-data 3.1, 3.6, 4.3).
//! The expected accesses were written by hand from the fixture texts.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used
)]

use kiro_code::scan::scan;
use kiro_code::{SourceBundle, SourceText};

fn accesses(path: &str, text: &str) -> Vec<(u32, String, String, &'static str)> {
    let bundle = SourceBundle {
        files: vec![SourceText {
            path: path.to_string(),
            text: text.to_string(),
        }],
        unreadable: Vec::new(),
    };
    let data = scan(&bundle).data;
    data.accesses
        .into_iter()
        .map(|access| (access.line, access.expression, access.position, access.kind))
        .collect()
}

fn expected(list: &[(u32, &str, &str, &'static str)]) -> Vec<(u32, String, String, &'static str)> {
    list.iter()
        .map(|(line, expression, position, kind)| {
            (
                *line,
                (*expression).to_string(),
                (*position).to_string(),
                *kind,
            )
        })
        .collect()
}

const PYTHON: &str = "def summarize(frame, grid, rows):
    price = frame.iloc[:, 3]
    cell = frame.iat[0, 2]
    pair = frame.iloc[:, [0, 2]]
    last = frame.iloc[:, -1]
    name = frame.columns[4]
    axis = grid[:, 1]
    for row in rows:
        total = row[2]
    for order_row in rows:
        other = order_row[0]
    return price, cell, pair, last, name, axis, total, other
";

#[test]
fn python_positions_are_found_by_shape() {
    assert_eq!(
        accesses("report.py", PYTHON),
        expected(&[
            (2, "frame.iloc[:, 3]", "3", "a dataframe column"),
            (3, "frame.iat[0, 2]", "2", "a dataframe column"),
            (4, "frame.iloc[:, [0, 2]]", "[0, 2]", "a dataframe column"),
            (5, "frame.iloc[:, -1]", "-1", "a dataframe column"),
            (6, "frame.columns[4]", "4", "a dataframe column"),
            (7, "grid[:, 1]", "1", "an array column"),
            (9, "row[2]", "2", "a row record field"),
            (11, "order_row[0]", "0", "a row record field"),
        ])
    );
}

const PYTHON_LEGITIMATE: &str = "def legitimate(frame, grid, items, args, row, index):
    by_name = frame[\"price\"]
    by_label = frame.loc[:, \"price\"]
    first_rows = frame.iloc[3]
    window = frame.iloc[1:3]
    head = items[0]
    second = args[1]
    element = grid[0][1]
    corner = grid[0, 0]
    computed = row[index]
    part = row[1:3]
    named = row[\"total\"]
    column = frame.iloc[:, index]
    return by_name, by_label, first_rows, window, head, second, element, corner, computed, part, named, column
";

#[test]
fn legitimate_python_indexing_and_computed_positions_are_not_reported() {
    // By name, by row position, on plain lists, element by element, and
    // with a position held in a variable (a documented blind spot).
    assert_eq!(accesses("ok.py", PYTHON_LEGITIMATE), expected(&[]));
}

const RUST: &str = "fn summarize(frame: &DataFrame, grid: &Array2<f64>) -> f64 {
    let price = frame.select_at_idx(3);
    let axis = grid.column(1);
    let cell = grid[[0, 2]];
    let view = grid.slice(s![.., 4]);
    let by_name = frame.column(\"price\");
    let items = vec![1, 2, 3];
    let head = items[0];
    let nested = table[1][2];
    let computed = grid.column(index);
    let rows = grid.slice(s![1..3, ..]);
    cell
}
";

#[test]
fn rust_positions_are_found_by_shape_and_plain_indexing_is_not() {
    assert_eq!(
        accesses("report.rs", RUST),
        expected(&[
            (2, "frame.select_at_idx(3)", "3", "a dataframe column"),
            (3, "grid.column(1)", "1", "an array column"),
            (4, "grid[[0, 2]]", "2", "an array column"),
            (5, "s![.., 4]", "4", "an array column"),
        ])
    );
}

#[test]
fn a_language_without_patterns_is_not_searched() {
    let bundle = SourceBundle {
        files: vec![
            SourceText {
                path: "a.ts".to_string(),
                text: "export const first = (row: number[]) => row[2];\n".to_string(),
            },
            SourceText {
                path: "b.py".to_string(),
                text: "x = 1\n".to_string(),
            },
        ],
        unreadable: Vec::new(),
    };
    let data = scan(&bundle).data;
    assert!(data.accesses.is_empty());
    assert_eq!(data.files, 1, "only the Python file has patterns");
}
