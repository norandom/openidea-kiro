//! Module-level bindings and literal comparisons (code-locality-data 1.1,
//! 2.1), extracted from the shared syntax tree. The expected facts were
//! written by hand from the fixture texts below.

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::expect_used,
    clippy::type_complexity
)]

use kiro_code::scan::scan;
use kiro_code::{SourceBundle, SourceText};

const RUST: &str = "pub const TIMEOUT: u64 = 30;
static NAME: &str = \"kiro\";
mod inner {
    pub const DEPTH: usize = 3;
}
fn run(order: &Order, mode: Mode) -> u8 {
    const LOCAL: u8 = 1;
    if order.status == \"paid\" {
        return 1;
    }
    if \"open\" != order.status {
        return 2;
    }
    let code = match order.kind.as_str() {
        \"a\" | \"b\" => 3,
        \"c\" => 4,
        _ => 5,
    };
    if mode == Mode::Fast { code } else { LOCAL }
}
impl Mode {
    fn label(&self) -> &str {
        match self {
            Mode::Fast => \"fast\",
            Mode::Slow => \"slow\",
        }
    }
}
";

const PYTHON: &str = "TIMEOUT = 30
NAMES = (\"a\", \"b\")
x, y = 1, 2
def f(state, kind):
    LOCAL = 1
    if state == \"open\":
        return 1
    if kind in (\"x\", \"y\"):
        return 2
    if state != \"closed\" and \"done\" == state:
        return 3
    match kind:
        case \"z\":
            return 4
        case _:
            return 5
";

const TYPESCRIPT: &str = "export const TIMEOUT = 30;
const NAME = \"kiro\";
let counter = 0;
function f(state: string, kind: string): number {
  const LOCAL = 1;
  if (state === \"open\") { return 1; }
  if (\"closed\" !== state) { return 2; }
  switch (kind) {
    case \"a\":
      return 3;
    case \"b\":
      return 4;
    default:
      return LOCAL;
  }
}
";

fn facts(path: &str, text: &str) -> (Vec<(u32, String, String)>, Vec<(String, String)>) {
    let bundle = SourceBundle {
        files: vec![SourceText {
            path: path.to_string(),
            text: text.to_string(),
        }],
        unreadable: Vec::new(),
    };
    let scanned = scan(&bundle);
    assert_eq!(scanned.locality.files, 1, "{path} must parse");
    let bindings = scanned
        .locality
        .bindings
        .iter()
        .map(|binding| {
            (
                binding.line,
                binding.name.clone(),
                binding.initializer.clone(),
            )
        })
        .collect();
    let mut comparisons: Vec<(String, String)> = scanned
        .locality
        .comparisons
        .iter()
        .map(|comparison| (comparison.subject.clone(), comparison.literal.clone()))
        .collect();
    comparisons.sort();
    (bindings, comparisons)
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = list
        .iter()
        .map(|(subject, literal)| ((*subject).to_string(), (*literal).to_string()))
        .collect();
    out.sort();
    out
}

#[test]
fn rust_bindings_and_comparisons() {
    let (bindings, comparisons) = facts("a.rs", RUST);
    // Module level and inside `mod`; a constant inside a routine is not one.
    assert_eq!(
        bindings,
        [
            (1, "TIMEOUT".to_string(), "30".to_string()),
            (2, "NAME".to_string(), "\"kiro\"".to_string()),
            (4, "DEPTH".to_string(), "3".to_string()),
        ]
    );
    // Either side may hold the literal; match arms and alternatives count;
    // an enum path is a literal; `match self` is no subject.
    assert_eq!(
        comparisons,
        pairs(&[
            ("order.status", "\"paid\""),
            ("order.status", "\"open\""),
            ("order.kind.as_str()", "\"a\""),
            ("order.kind.as_str()", "\"b\""),
            ("order.kind.as_str()", "\"c\""),
            ("mode", "Mode::Fast"),
        ])
    );
}

#[test]
fn python_bindings_and_comparisons() {
    let (bindings, comparisons) = facts("a.py", PYTHON);
    // A destructuring assignment and a local are no definitions.
    assert_eq!(
        bindings,
        [
            (1, "TIMEOUT".to_string(), "30".to_string()),
            (2, "NAMES".to_string(), "(\"a\", \"b\")".to_string()),
        ]
    );
    // Equality, inequality, membership in a literal tuple, and match cases.
    assert_eq!(
        comparisons,
        pairs(&[
            ("state", "\"open\""),
            ("kind", "\"x\""),
            ("kind", "\"y\""),
            ("state", "\"closed\""),
            ("state", "\"done\""),
            ("kind", "\"z\""),
        ])
    );
}

#[test]
fn typescript_bindings_and_comparisons() {
    let (bindings, comparisons) = facts("a.ts", TYPESCRIPT);
    assert_eq!(
        bindings,
        [
            (1, "TIMEOUT".to_string(), "30".to_string()),
            (2, "NAME".to_string(), "\"kiro\"".to_string()),
            (3, "counter".to_string(), "0".to_string()),
        ]
    );
    assert_eq!(
        comparisons,
        pairs(&[
            ("state", "\"open\""),
            ("state", "\"closed\""),
            ("kind", "\"a\""),
            ("kind", "\"b\""),
        ])
    );
}

#[test]
fn a_long_initializer_is_no_definition_and_whitespace_is_collapsed() {
    let long = format!(
        "pub const TABLE: [u8; 14] = [\n{}];\n",
        "    1,\n".repeat(14)
    );
    let (bindings, _) = facts("a.rs", &long);
    assert!(
        bindings.is_empty(),
        "an initializer of more than 12 lines is skipped"
    );

    let (bindings, _) = facts("a.rs", "pub const PAIR: (u8, u8) = (1,\n        2);\n");
    assert_eq!(bindings[0].2, "(1, 2)");
}

#[test]
fn a_comparison_of_two_names_or_two_literals_is_no_state_comparison() {
    let (_, comparisons) = facts(
        "a.rs",
        "fn f(a: &str, b: &str) -> bool {\n    a == b || \"x\" == \"y\" || a.len() == 3\n}\n",
    );
    assert!(comparisons.is_empty(), "{comparisons:?}");
}
