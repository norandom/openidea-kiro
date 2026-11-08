//! Criterion text masking and word scanning (design component `CriterionText`,
//! requirements 2.2, 2.10, 3.7).
//!
//! Provides masking of balanced quoted literals, word scanning with
//! word-internal apostrophes, whole-word case-insensitive matching,
//! hyphen adjacency, and whole-word phrase matching.

/// Same byte length as `text`: the content of every balanced quoted literal
/// is replaced by spaces (one per byte of each char).
///
/// Delimiters: `"`, `“`→`”`, `` ` ``, `'` and `‘`→`’` — the single-quote forms
/// only when not between two alphanumerics.
/// The closing delimiter is the matching one at the next position that is
/// itself a delimiter; absent → the opener stays unmasked.
#[must_use]
pub fn mask_literals(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut byte_idx = 0;

    while byte_idx < text.len() {
        let Some(remaining) = text.get(byte_idx..) else {
            break;
        };
        if let Some((closer_kind, opener_len)) = detect_opener(text, byte_idx) {
            let content_start = byte_idx + opener_len;
            if let Some(close_offset) = find_closer(text, content_start, closer_kind) {
                // Balanced literal found!
                // 1. Keep opener as-is
                if let Some(opener_slice) = text.get(byte_idx..content_start) {
                    out.push_str(opener_slice);
                }
                // 2. Replace content with spaces (preserving byte length)
                let content_byte_len = close_offset.saturating_sub(content_start);
                for _ in 0..content_byte_len {
                    out.push(' ');
                }
                // 3. Keep closer as-is
                let closer_len = closer_byte_len(closer_kind);
                let after_closer = close_offset + closer_len;
                if let Some(closer_slice) = text.get(close_offset..after_closer) {
                    out.push_str(closer_slice);
                }
                byte_idx = after_closer;
                continue;
            }
        }

        // Not an opener or unbalanced: copy one character as-is
        let c = remaining.chars().next().unwrap_or(' ');
        out.push(c);
        byte_idx += c.len_utf8();
    }

    debug_assert_eq!(out.len(), text.len());
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloserKind {
    StraightDouble,
    Backtick,
    TypoDouble,
    StraightSingle,
    TypoSingle,
}

fn closer_byte_len(kind: CloserKind) -> usize {
    match kind {
        CloserKind::StraightDouble | CloserKind::Backtick | CloserKind::StraightSingle => 1,
        CloserKind::TypoDouble => '”'.len_utf8(),
        CloserKind::TypoSingle => '’'.len_utf8(),
    }
}

fn is_between_alphanumeric(text: &str, byte_idx: usize, char_len: usize) -> bool {
    let before = text.get(..byte_idx).and_then(|s| s.chars().next_back());
    let after = text
        .get(byte_idx + char_len..)
        .and_then(|s| s.chars().next());
    matches!((before, after), (Some(b), Some(a)) if b.is_alphanumeric() && a.is_alphanumeric())
}

fn detect_opener(text: &str, byte_idx: usize) -> Option<(CloserKind, usize)> {
    let rem = text.get(byte_idx..)?;
    if rem.starts_with('"') {
        Some((CloserKind::StraightDouble, 1))
    } else if rem.starts_with('`') {
        Some((CloserKind::Backtick, 1))
    } else if rem.starts_with('“') {
        Some((CloserKind::TypoDouble, '“'.len_utf8()))
    } else if rem.starts_with('‘') {
        let len = '‘'.len_utf8();
        if !is_between_alphanumeric(text, byte_idx, len) {
            Some((CloserKind::TypoSingle, len))
        } else {
            None
        }
    } else if rem.starts_with('\'') {
        if !is_between_alphanumeric(text, byte_idx, 1) {
            Some((CloserKind::StraightSingle, 1))
        } else {
            None
        }
    } else {
        None
    }
}

fn find_closer(text: &str, start_search: usize, kind: CloserKind) -> Option<usize> {
    let mut idx = start_search;
    while idx < text.len() {
        let rem = text.get(idx..)?;
        let matched = match kind {
            CloserKind::StraightDouble => rem.starts_with('"'),
            CloserKind::Backtick => rem.starts_with('`'),
            CloserKind::TypoDouble => rem.starts_with('”'),
            CloserKind::StraightSingle => {
                rem.starts_with('\'') && !is_between_alphanumeric(text, idx, 1)
            }
            CloserKind::TypoSingle => {
                rem.starts_with('’') && !is_between_alphanumeric(text, idx, '’'.len_utf8())
            }
        };
        if matched {
            return Some(idx);
        }
        let c = rem.chars().next()?;
        idx += c.len_utf8();
    }
    None
}

/// Byte range of a maximal run of alphanumerics, `_` and word-internal apostrophes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word {
    pub start: usize,
    pub end: usize,
}

/// Enumerates words in `masked` text in document order.
#[must_use]
pub fn words(masked: &str) -> Vec<Word> {
    let mut result = Vec::new();
    let mut chars = masked.char_indices().peekable();

    while let Some(&(start, c)) = chars.peek() {
        if c.is_alphanumeric() || c == '_' {
            chars.next();
            let mut end = start + c.len_utf8();
            while let Some(&(idx, next_c)) = chars.peek() {
                if next_c.is_alphanumeric() || next_c == '_' {
                    end = idx + next_c.len_utf8();
                    chars.next();
                } else if (next_c == '\'' || next_c == '’')
                    && let Some(after_c) = masked
                        .get(idx + next_c.len_utf8()..)
                        .and_then(|s| s.chars().next())
                    && (after_c.is_alphanumeric() || after_c == '_')
                {
                    chars.next(); // consume apostrophe
                    chars.next(); // consume after_c
                    end = idx + next_c.len_utf8() + after_c.len_utf8();
                } else {
                    break;
                }
            }
            result.push(Word { start, end });
        } else {
            chars.next();
        }
    }
    result
}

/// Whole-word, ASCII case-insensitive matches of `word` in document order.
#[must_use]
pub fn find_word_ci(masked: &str, word: &str) -> Vec<Word> {
    words(masked)
        .into_iter()
        .filter(|w| {
            masked
                .get(w.start..w.end)
                .is_some_and(|s| s.eq_ignore_ascii_case(word))
        })
        .collect()
}

/// True when the byte before `w.start` or after `w.end` is `-`.
#[must_use]
pub fn hyphen_adjacent(masked: &str, w: Word) -> bool {
    let bytes = masked.as_bytes();
    (w.start > 0 && bytes.get(w.start.saturating_sub(1)) == Some(&b'-'))
        || bytes.get(w.end) == Some(&b'-')
}

/// Whole-word phrase match: `phrase` (already `normalize_label`ed) equals
/// a run of consecutive words of `masked`, lower-cased and joined by one space.
#[must_use]
pub fn contains_phrase(masked: &str, phrase: &str) -> bool {
    if phrase.is_empty() || masked.is_empty() {
        return false;
    }
    let m_words = words(masked);
    if m_words.is_empty() {
        return false;
    }

    let p_words = words(phrase);
    if p_words.is_empty() {
        return false;
    }
    let k = p_words.len();
    if k > m_words.len() {
        return false;
    }

    let phrase_lower = phrase.to_lowercase();
    let p_words_joined = p_words
        .iter()
        .filter_map(|w| phrase.get(w.start..w.end))
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ");

    for i in 0..=(m_words.len() - k) {
        let Some(run) = m_words.get(i..i + k) else {
            continue;
        };
        let Some(first_word) = run.first().copied() else {
            continue;
        };
        let Some(last_word) = run.last().copied() else {
            continue;
        };

        let run_joined = run
            .iter()
            .filter_map(|w| masked.get(w.start..w.end))
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
            .join(" ");

        let matches_phrase = run_joined == phrase_lower
            || run_joined == p_words_joined
            || masked
                .get(first_word.start..last_word.end)
                .is_some_and(|s| s.to_lowercase() == phrase_lower);

        if matches_phrase {
            let hyphen_before = first_word.start > 0
                && masked
                    .as_bytes()
                    .get(first_word.start.saturating_sub(1))
                    .is_some_and(|&b| b == b'-');
            let hyphen_after = masked
                .as_bytes()
                .get(last_word.end)
                .is_some_and(|&b| b == b'-');

            let phrase_starts_hyphen = phrase.starts_with('-');
            let phrase_ends_hyphen = phrase.ends_with('-');

            if (hyphen_before && !phrase_starts_hyphen) || (hyphen_after && !phrase_ends_hyphen) {
                continue;
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_balanced_double_quotes() {
        let text = r#"The system shall display "you must enter a password"."#;
        let masked = mask_literals(text);
        assert_eq!(masked.len(), text.len());
        assert_eq!(
            masked,
            r#"The system shall display "                         "."#
        );
        assert!(find_word_ci(&masked, "must").is_empty());
        assert_eq!(find_word_ci(&masked, "shall").len(), 1);
    }

    #[test]
    fn mask_unbalanced_double_quotes() {
        let text = r#"The system shall display "unclosed quote must remain"#;
        let masked = mask_literals(text);
        assert_eq!(masked, text);
        assert_eq!(find_word_ci(&masked, "must").len(), 1);
    }

    #[test]
    fn mask_typographic_double_quotes() {
        let text = "The system shall display “you must enter”.";
        let masked = mask_literals(text);
        assert_eq!(masked.len(), text.len());
        assert_eq!(masked, "The system shall display “              ”.");
        assert!(find_word_ci(&masked, "must").is_empty());
    }

    #[test]
    fn mask_backticks() {
        let text = "The system shall return `must` always.";
        let masked = mask_literals(text);
        assert_eq!(masked.len(), text.len());
        assert_eq!(masked, "The system shall return `    ` always.");
        assert!(find_word_ci(&masked, "must").is_empty());
    }

    #[test]
    fn mask_single_quotes_and_possessives() {
        let text = "The validator shall verify the tree's root hash and 'must'.";
        let masked = mask_literals(text);
        assert_eq!(masked.len(), text.len());
        assert_eq!(
            masked,
            "The validator shall verify the tree's root hash and '    '."
        );
        assert!(find_word_ci(&masked, "must").is_empty());
        assert_eq!(find_word_ci(&masked, "tree's").len(), 1);
    }

    #[test]
    fn mask_typographic_single_quotes_and_possessives() {
        let text = "The validator shall verify the tree’s root hash and ‘must’.";
        let masked = mask_literals(text);
        assert_eq!(masked.len(), text.len());
        assert_eq!(
            masked,
            "The validator shall verify the tree’s root hash and ‘    ’."
        );
        assert!(find_word_ci(&masked, "must").is_empty());
        assert_eq!(find_word_ci(&masked, "tree’s").len(), 1);
    }

    #[test]
    fn words_and_possessives() {
        let text = "tree's branch and don’t do_that, a''b foo-bar";
        let ws = words(text);
        let extracted: Vec<&str> = ws.iter().filter_map(|w| text.get(w.start..w.end)).collect();
        assert_eq!(
            extracted,
            vec![
                "tree's", "branch", "and", "don’t", "do_that", "a", "b", "foo", "bar"
            ]
        );
    }

    #[test]
    fn hyphen_adjacent_check() {
        let text = "The tool shall run must-gather and -start finish-";
        let ws = words(text);
        let mut must = None;
        let mut gather = None;
        let mut start = None;
        let mut finish = None;
        let mut tool = None;
        for w in &ws {
            if let Some(s) = text.get(w.start..w.end) {
                match s {
                    "must" => must = Some(*w),
                    "gather" => gather = Some(*w),
                    "start" => start = Some(*w),
                    "finish" => finish = Some(*w),
                    "tool" => tool = Some(*w),
                    _ => {}
                }
            }
        }
        assert!(must.is_some_and(|w| hyphen_adjacent(text, w)));
        assert!(gather.is_some_and(|w| hyphen_adjacent(text, w)));
        assert!(start.is_some_and(|w| hyphen_adjacent(text, w)));
        assert!(finish.is_some_and(|w| hyphen_adjacent(text, w)));
        assert!(tool.is_some_and(|w| !hyphen_adjacent(text, w)));
    }

    #[test]
    fn contains_phrase_scenarios() {
        let text = "The on-call engineer shall verify the acceptance criterion.";
        assert!(contains_phrase(text, "on-call engineer"));
        assert!(contains_phrase(text, "acceptance criterion"));
        assert!(contains_phrase(text, "engineer"));
        assert!(!contains_phrase(text, "operator"));
        assert!(!contains_phrase(text, "acceptance criteria"));

        // Hyphen boundary should not match substring of hyphenated word
        assert!(!contains_phrase("pre-acceptance criterion", "acceptance"));
        assert!(!contains_phrase("acceptance-criterion-extra", "criterion"));
    }
}
