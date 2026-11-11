//! Code lines of a file for the duplicate search (clone-constraints 1.1).
//!
//! A code line is the line's tokens with whitespace and comments removed.
//! The tokens are the leaves of the same syntax tree the metrics use; no
//! second tokenizer is involved. A token that spans several lines (a long
//! string) belongs to the line it starts on.

use std::collections::BTreeMap;

use ast_grep_core::tree_sitter::StrDoc;
use ast_grep_core::{AstGrep, Node};
use ast_grep_language::SupportLang;
use kiro_core::code::clones::{FileLines, LineHash};

use crate::measure::{comment_kinds, language_of};

fn fnv(hash: u64, bytes: &[u8]) -> u64 {
    let mut hash = hash;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The code lines of a parsed file, or `None` when the tree has a hole.
#[must_use]
pub fn lines_of(
    root: &Node<'_, StrDoc<SupportLang>>,
    comments: &[&str],
    path: &str,
) -> Option<FileLines> {
    let mut by_line: BTreeMap<usize, u64> = BTreeMap::new();
    // Document order: children are pushed reversed so they pop in order.
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if node.is_error() || node.is_missing() {
            return None;
        }
        let kind = node.kind();
        if comments.contains(&kind.as_ref()) {
            continue;
        }
        let mut children: Vec<_> = node.children().collect();
        if children.is_empty() {
            let text = node.text();
            if !text.trim().is_empty() {
                let entry = by_line
                    .entry(node.start_pos().line())
                    .or_insert(0xcbf2_9ce4_8422_2325);
                // A separator keeps `a b` and `ab` apart.
                *entry = fnv(fnv(*entry, text.as_bytes()), &[0]);
            }
        } else {
            children.reverse();
            stack.extend(children);
        }
    }
    Some(FileLines {
        path: path.to_string(),
        lines: by_line
            .into_iter()
            .map(|(line, hash)| LineHash {
                line: u32::try_from(line.saturating_add(1)).unwrap_or(u32::MAX),
                hash,
            })
            .collect(),
    })
}

/// The code lines of one source text, or `None` when no profile covers the
/// file or it does not parse: such a file takes no part in the search.
#[must_use]
pub fn file_lines(path: &str, text: &str) -> Option<FileLines> {
    let lang = language_of(path)?;
    let comments = comment_kinds(path)?;
    let normalized = text.replace("\r\n", "\n");
    let tree = StrDoc::try_new(&normalized, lang).map(AstGrep::doc).ok()?;
    lines_of(&tree.root(), comments, path)
}
