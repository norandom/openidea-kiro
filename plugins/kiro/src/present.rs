//! Presentation and delivery of notifications, insertions, and messages (task 2.5).

use crate::host::{HostApi, Level};
use kiro_core::finding::{Finding, Severity};
use kiro_core::{clip_lines, findings_text};

pub const NOTIFICATION_BYTES: usize = 15 * 1024; // host clips at 16 KiB
pub const INSERT_BYTES: usize = 1024 * 1024 - 1024; // host clips at 1 MiB
pub const TITLE_BYTES: usize = 256;
pub const ERROR_BYTES: usize = 1024;

/// Result shape for a command execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    pub title: String,
    pub level: Level,
    pub text: String,
}

impl Output {
    /// Constructs an `Output` with `Level::Info`.
    pub fn info(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            level: Level::Info,
            text: text.into(),
        }
    }

    /// Replaces the level of this output.
    #[must_use]
    pub fn with_level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Constructs an `Output` from an intro and findings list.
    ///
    /// The text consists of the intro (if non-empty, followed by a blank line)
    /// plus `findings_text(findings)`. The level is derived via `level_of(findings)`.
    pub fn with_findings(title: impl Into<String>, intro: &str, findings: &[Finding]) -> Self {
        let f_text = findings_text(findings);
        let text = if intro.is_empty() {
            f_text
        } else if f_text.is_empty() {
            intro.to_owned()
        } else {
            format!("{intro}\n\n{f_text}")
        };
        Self {
            title: title.into(),
            level: level_of(findings),
            text,
        }
    }
}

/// Maps finding severities to a presentation notification `Level`.
///
/// Priority: any `Severity::Error` -> `Level::Error`, else any
/// `Severity::Warning` -> `Level::Warning`, else `Level::Info`.
#[must_use]
pub fn level_of(findings: &[Finding]) -> Level {
    if findings.iter().any(|f| f.severity == Severity::Error) {
        Level::Error
    } else if findings.iter().any(|f| f.severity == Severity::Warning) {
        Level::Warning
    } else {
        Level::Info
    }
}

/// Clips notification text to fit within `NOTIFICATION_BYTES`.
///
/// When clipped, appends a trailer naming the insert variant if provided,
/// or a plain line-count trailer if omitted.
#[must_use]
pub fn clip_notification(text: &str, insert_id: Option<&str>) -> String {
    let hint = insert_id.map(|id| format!("run {id} for the full text"));
    clip_lines(text, NOTIFICATION_BYTES, hint.as_deref())
}

/// Clips editor insertion text to fit within `INSERT_BYTES`.
#[must_use]
pub fn clip_insert(text: &str) -> String {
    clip_lines(text, INSERT_BYTES, None)
}

/// Clips a title string to at most `TITLE_BYTES` on a UTF-8 character boundary.
#[must_use]
pub fn clip_title(title: &str) -> String {
    if title.len() <= TITLE_BYTES {
        return title.to_owned();
    }
    let mut cut = TITLE_BYTES;
    while !title.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    title[..cut].to_owned()
}

/// Flattens an error message into a single line of at most `ERROR_BYTES`.
///
/// Newlines are replaced with `"; "`, outer and blank lines are trimmed,
/// and the result is cut on a character boundary at `ERROR_BYTES`.
#[must_use]
pub fn error_line(message: &str) -> String {
    let parts: Vec<&str> = message
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let joined = parts.join("; ");
    if joined.len() <= ERROR_BYTES {
        return joined;
    }
    let mut cut = ERROR_BYTES;
    while !joined.is_char_boundary(cut) {
        cut = cut.saturating_sub(1);
    }
    joined[..cut].to_owned()
}

/// Delivers a command's output to the host.
///
/// Shows exactly one notification (with clipped title and notification body),
/// performs at most one text insertion when `insert` is true, and returns the
/// complete, unclipped text as the command message.
pub fn deliver(host: &dyn HostApi, out: &Output, insert: bool, insert_id: Option<&str>) -> String {
    let title = clip_title(&out.title);
    let body = clip_notification(&out.text, insert_id);
    host.show_notification(out.level, &title, &body);

    if insert {
        let insert_body = clip_insert(&out.text);
        host.insert_text(&insert_body);
    }

    out.text.clone()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::cloned_ref_to_slice_refs
)]
mod tests {
    use super::*;
    use crate::host::{FakeHost, HostEvent};
    use kiro_core::finding::{Finding, Severity};

    fn make_finding(severity: Severity, code: &'static str, msg: &str) -> Finding {
        Finding {
            severity,
            code,
            feature: Some("kiro".to_owned()),
            file: "spec.json".to_owned(),
            line: Some(10),
            message: msg.to_owned(),
        }
    }

    #[test]
    fn level_of_maps_severities_correctly() {
        assert_eq!(level_of(&[]), Level::Info);

        let info = make_finding(Severity::Info, "REQ_COVERAGE_GAP", "info msg");
        assert_eq!(level_of(&[info.clone()]), Level::Info);

        let warn = make_finding(Severity::Warning, "SPEC_ORPHAN", "warn msg");
        assert_eq!(level_of(&[info.clone(), warn.clone()]), Level::Warning);

        let err = make_finding(Severity::Error, "SPEC_UNPARSEABLE", "err msg");
        assert_eq!(level_of(&[info, warn, err]), Level::Error);
    }

    #[test]
    fn output_with_findings_matches_engine_text_and_derives_level() {
        let err = make_finding(Severity::Error, "SPEC_UNPARSEABLE", "failed parse");
        let out = Output::with_findings("Title", "Intro paragraph", &[err]);
        assert_eq!(out.title, "Title");
        assert_eq!(out.level, Level::Error);
        assert!(out.text.starts_with("Intro paragraph\n\n"));
        assert!(out.text.contains("spec.json:10: ERROR SPEC_UNPARSEABLE failed parse"));

        // When intro is empty, text is byte-identical to findings_text
        let findings = [make_finding(Severity::Warning, "SPEC_ORPHAN", "orphan")];
        let out_no_intro = Output::with_findings("Title", "", &findings);
        assert_eq!(out_no_intro.text, findings_text(&findings));

        // When findings are empty, intro is retained
        let out_empty_findings = Output::with_findings("Title", "No findings here", &[]);
        assert_eq!(out_empty_findings.text, "No findings here");
        assert_eq!(out_empty_findings.level, Level::Info);
    }

    #[test]
    fn clip_notification_handles_small_and_large_text() {
        let small = "Short message\nLine 2";
        assert_eq!(clip_notification(small, Some("ws.specs.insert")), small);

        // 20 KiB text
        let line = "This is a line of text that takes some bytes.\n";
        let count = (20 * 1024) / line.len() + 10;
        let large = line.repeat(count);
        assert!(large.len() >= 20 * 1024);

        // With insert_id
        let clipped_with_hint = clip_notification(&large, Some("ws.specs.insert"));
        assert!(clipped_with_hint.len() <= 16 * 1024);
        assert!(clipped_with_hint.ends_with("— run ws.specs.insert for the full text"));

        // Without insert_id
        let clipped_without_hint = clip_notification(&large, None);
        assert!(clipped_without_hint.len() <= 16 * 1024);
        assert!(!clipped_without_hint.contains("run ws.specs.insert"));
        assert!(clipped_without_hint.contains("more lines"));
    }

    #[test]
    fn clip_insert_respects_insert_budget() {
        let small = "Small insert";
        assert_eq!(clip_insert(small), small);

        let line = format!("{}\n", "A".repeat(2048));
        let count = (1024 * 1024) / line.len() + 100;
        let large = line.repeat(count);
        assert!(large.len() > 1024 * 1024);

        let clipped = clip_insert(&large);
        assert!(clipped.len() <= 1024 * 1024);
        assert!(clipped.contains("more lines"));
    }

    #[test]
    fn clip_title_handles_multibyte_boundary() {
        let short = "Short title";
        assert_eq!(clip_title(short), short);

        // Multi-byte character: Euro sign '€' is 3 bytes (0xE2, 0x82, 0xAC)
        let euro = "€"; // 3 bytes
        // 85 * 3 = 255 bytes. 86 * 3 = 258 bytes.
        let long_multibyte = euro.repeat(90);
        let clipped = clip_title(&long_multibyte);
        assert!(clipped.len() <= TITLE_BYTES);
        assert_eq!(clipped.len(), 85 * 3); // 255 bytes, does not split 86th char
    }

    #[test]
    fn error_line_flattens_newlines_and_caps_at_budget() {
        let err = "  first line \n\n  second line\r\nthird line   ";
        assert_eq!(error_line(err), "first line; second line; third line");

        // Over 1 KiB
        let chunk = "A".repeat(1200);
        let clipped = error_line(&chunk);
        assert_eq!(clipped.len(), ERROR_BYTES);

        // Multibyte cap
        let euro_chunk = "€".repeat(400); // 1200 bytes
        let euro_clipped = error_line(&euro_chunk);
        assert!(euro_clipped.len() <= ERROR_BYTES);
        assert_eq!(euro_clipped.len(), 341 * 3); // 1023 bytes
    }

    #[test]
    fn deliver_notifies_and_optionally_inserts() {
        let host = FakeHost::new();
        let out = Output::info("My Title", "My Full Content");

        // Case 1: insert = false
        let ret1 = deliver(&host, &out, false, Some("ws.specs.insert"));
        assert_eq!(ret1, "My Full Content");

        let events = host.events();
        assert_eq!(events.len(), 1);
        match &events[0] {
            HostEvent::Notify { level, title, body } => {
                assert_eq!(*level, Level::Info);
                assert_eq!(title, "My Title");
                assert_eq!(body, "My Full Content");
            }
            _ => panic!("unexpected event"),
        }

        // Case 2: insert = true
        let host2 = FakeHost::new();
        let ret2 = deliver(&host2, &out, true, Some("ws.specs.insert"));
        assert_eq!(ret2, "My Full Content");

        let events2 = host2.events();
        assert_eq!(events2.len(), 2);
        match &events2[0] {
            HostEvent::Notify { title, body, .. } => {
                assert_eq!(title, "My Title");
                assert_eq!(body, "My Full Content");
            }
            _ => panic!("unexpected event 0"),
        }
        match &events2[1] {
            HostEvent::Insert(text) => {
                assert_eq!(text, "My Full Content");
            }
            _ => panic!("unexpected event 1"),
        }
    }
}
