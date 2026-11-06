//! `.kiro/steering/roadmap.md`'s `## Specs (dependency order)` list into
//! entries and known feature names (design component `RoadmapParser`,
//! requirements 7.1–7.6): checkbox entries split into name, description and
//! dependencies with `--` / em-dash separators, backticks stripped, `none` as
//! empty, unknown dependencies reported after all entries are read.

use crate::finding::{Finding, codes};
use crate::md::{Heading, LineKind, scan};

/// One roadmap entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapEntry {
    pub name: String,
    pub checked: bool,
    pub description: String,
    pub dependencies: Vec<String>,
    pub line: usize,
}

/// The parsed roadmap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoadmapDoc {
    pub file: String,
    pub outline: Vec<Heading>,
    pub entries: Vec<RoadmapEntry>,
    pub findings: Vec<Finding>,
}

impl RoadmapDoc {
    /// Feature names in document order, deduplicated.
    pub fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for entry in &self.entries {
            if !names.contains(&entry.name) {
                names.push(entry.name.clone());
            }
        }
        names
    }
}

/// Parses `roadmap.md`. Never panics; problems become findings.
pub fn parse(text: &str, file: &str) -> RoadmapDoc {
    let scanned = scan(text, file);
    let mut doc = RoadmapDoc {
        file: file.to_owned(),
        outline: scanned.outline.clone(),
        entries: Vec::new(),
        findings: scanned.findings,
    };

    // The specs region: a heading starting with `specs` and containing
    // `dependency order` (case-insensitive), until the next heading of the
    // same or higher level.
    let region = doc.outline.iter().find_map(|h| {
        let lower = h.text.trim().to_ascii_lowercase();
        let is_specs = lower.starts_with("specs") && lower.contains("dependency order");
        if !is_specs {
            return None;
        }
        let end = scanned
            .lines
            .iter()
            .filter(|l| l.line > h.line)
            .find_map(|l| match l.kind {
                LineKind::Heading { level, text: _ } if level <= h.level => Some(l.line - 1),
                _ => None,
            })
            .unwrap_or_else(|| scanned.lines.last().map(|l| l.line).unwrap_or(h.line));
        Some((h.line + 1, end))
    });

    let Some((start, end)) = region else {
        doc.findings.push(Finding::new(
            codes::ROADMAP_NO_SPECS_SECTION,
            file,
            "roadmap has no `## Specs (dependency order)` section",
        ));
        return doc;
    };

    for line in scanned
        .lines
        .iter()
        .filter(|l| l.line >= start && l.line <= end)
    {
        let LineKind::ListItem {
            bullet: _,
            checkbox,
            marker: _,
            text,
        } = line.kind
        else {
            continue;
        };
        let Some(checkbox) = checkbox else {
            continue;
        };
        let Some(entry) = entry_of(text, checkbox.checked, line.line) else {
            continue;
        };
        doc.entries.push(entry);
    }

    // Unknown dependencies, checked after all entries are read.
    let names = doc.names();
    let mut unknown: Vec<(usize, String)> = Vec::new();
    for entry in &doc.entries {
        for dep in &entry.dependencies {
            if !names.contains(dep) {
                unknown.push((entry.line, dep.clone()));
            }
        }
    }
    for (line, dep) in unknown {
        doc.findings.push(
            Finding::new(
                codes::ROADMAP_DEP_UNKNOWN,
                file,
                format!("dependency {dep} names no roadmap entry"),
            )
            .at(line),
        );
    }
    doc
}

/// `- [ ] name -- description. Dependencies: a, b` in one list item text.
fn entry_of(text: &str, checked: bool, line: usize) -> Option<RoadmapEntry> {
    let separator = text
        .find(" -- ")
        .map(|idx| (idx, " -- ".len()))
        .or_else(|| text.find(" — ").map(|idx| (idx, " — ".len())));
    let (name, rest) = match separator {
        Some((idx, len)) => (text[..idx].trim(), &text[idx + len..]),
        None => {
            // No separator: the first whitespace-delimited token is the name.
            let first = text.split_whitespace().next()?;
            (
                first,
                text.get(first.len()..).unwrap_or_default().trim_start(),
            )
        }
    };
    let name = name.trim_matches('`').trim().to_owned();
    if name.is_empty() {
        return None;
    }
    let (description, deps) = match rest.to_ascii_lowercase().find("dependencies:") {
        Some(idx) => (
            &rest[..idx],
            Some(rest.get(idx + "dependencies:".len()..).unwrap_or_default()),
        ),
        None => (rest, None),
    };
    let description = description.trim().trim_end_matches('.').to_owned();
    let dependencies: Vec<String> = deps
        .map(|raw| {
            raw.split(',')
                .map(str::trim)
                .filter(|d| !d.is_empty())
                .map(|d| d.trim_end_matches('.').trim_matches('`').to_owned())
                .filter(|d| !d.eq_ignore_ascii_case("none"))
                .collect()
        })
        .unwrap_or_default();
    Some(RoadmapEntry {
        name,
        checked,
        description,
        dependencies,
        line,
    })
}

#[allow(clippy::panic)]
#[cfg(test)]
mod tests {
    use crate::finding::{Severity, codes};
    use crate::roadmap::parse;

    const F: &str = ".kiro/steering/roadmap.md";
    const MINIMAL: &str = include_str!("../fixtures/minimal/.kiro/steering/roadmap.md");
    const SELF: &str = include_str!("../fixtures/self/roadmap.md");

    #[test]
    fn minimal_fixture_entries_and_known_dependencies() {
        let doc = parse(MINIMAL, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.entries.len(), 2);
        let demo = doc
            .entries
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("demo"));
        assert!(demo.checked);
        assert_eq!(demo.name, "demo");
        assert_eq!(
            demo.description, "Minimal demo feature",
            "trailing dot trimmed per design"
        );
        assert!(
            demo.dependencies.is_empty(),
            "'Dependencies: none' is empty"
        );
        let old = doc.entries.get(1).cloned().unwrap_or_else(|| panic!("old"));
        assert_eq!(old.name, "old");
        assert_eq!(
            old.dependencies,
            ["demo"],
            "a dependency naming a known entry is fine"
        );
        assert_eq!(doc.names(), ["demo", "old"]);
    }

    #[test]
    fn separators_backticks_and_em_dash() {
        let text = "## Specs (dependency order)\n\n- [ ] `alpha` -- First feature. Dependencies: none\n- [x] beta — Second feature with an em dash. Dependencies: alpha\n- [ ] gamma Third-feature-without-separator prose. Dependencies: alpha\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        assert_eq!(doc.entries.len(), 3);
        let alpha = doc
            .entries
            .first()
            .cloned()
            .unwrap_or_else(|| panic!("alpha"));
        assert_eq!(alpha.name, "alpha", "backticks stripped");
        assert!(!alpha.checked);
        let beta = doc
            .entries
            .get(1)
            .cloned()
            .unwrap_or_else(|| panic!("beta"));
        assert!(beta.checked);
        assert_eq!(beta.dependencies, ["alpha"]);
        let gamma = doc
            .entries
            .get(2)
            .cloned()
            .unwrap_or_else(|| panic!("gamma"));
        assert_eq!(gamma.name, "gamma", "no separator: first whitespace token");
    }

    #[test]
    fn description_trailing_dot_trimmed_and_dependencies_variants() {
        let text = "## Specs (dependency order)\n\n- [ ] a -- Some description without dependencies\n- [ ] b -- Ends here.. Dependencies: a.\n- [ ] c -- Deps list. dependencies: a, b , c\n";
        let doc = parse(text, F);
        assert!(doc.findings.is_empty(), "{:?}", doc.findings);
        let a = doc.entries.first().cloned().unwrap_or_else(|| panic!("a"));
        assert_eq!(a.description, "Some description without dependencies");
        let b = doc.entries.get(1).cloned().unwrap_or_else(|| panic!("b"));
        assert_eq!(b.description, "Ends here", "trailing dots trimmed");
        assert_eq!(
            b.dependencies,
            ["a"],
            "trailing dot removed from the dependency"
        );
        let c = doc.entries.get(2).cloned().unwrap_or_else(|| panic!("c"));
        assert_eq!(c.dependencies, ["a", "b", "c"], "split on commas, trimmed");
    }

    #[test]
    fn unknown_dependency_warns_at_the_line_and_forward_references_are_known() {
        let text = "## Specs (dependency order)\n\n- [ ] a -- Depends on its successor. Dependencies: later\n- [ ] later -- Fine. Dependencies: none\n- [ ] b -- Broken. Dependencies: ghost\n";
        let doc = parse(text, F);
        let unknown: Vec<usize> = doc
            .findings
            .iter()
            .filter(|f| f.code == codes::ROADMAP_DEP_UNKNOWN)
            .map(|f| f.line.unwrap_or(0))
            .collect();
        assert_eq!(
            unknown,
            [5],
            "forward reference resolves; ghost at line 5 warns"
        );
        assert!(doc.findings.iter().all(|f| f.severity == Severity::Warning));
    }

    #[test]
    fn missing_section_is_info_with_empty_list() {
        let doc = parse("# Roadmap\n\n## Overview\n\nNothing.\n", F);
        assert!(doc.entries.is_empty());
        assert!(matches!(
            doc.findings.first(),
            Some(f) if f.code == codes::ROADMAP_NO_SPECS_SECTION && f.severity == Severity::Info
        ));
    }

    #[test]
    fn region_ends_at_next_heading_of_equal_or_higher_level() {
        let text = "## Specs (dependency order)\n\n- [ ] a -- One. Dependencies: none\n\n### Notes\n\n- [ ] b -- Inside a deeper section, still counted. Dependencies: none\n\n## After\n\n- [ ] c -- Outside the region. Dependencies: none\n";
        let doc = parse(text, F);
        assert_eq!(
            doc.entries.len(),
            2,
            "deeper heading stays inside; equal level ends it"
        );
        assert!(doc.findings.is_empty());
    }

    #[test]
    fn items_without_checkbox_are_ignored_and_names_deduplicate() {
        let text = "## Specs (dependency order)\n\n- [ ] a -- One. Dependencies: none\n- Bullet without checkbox\n- [ ] a -- Duplicate name. Dependencies: none\n";
        let doc = parse(text, F);
        assert_eq!(doc.entries.len(), 2, "entries are kept as written");
        assert_eq!(doc.names(), ["a"], "names deduplicate in document order");
    }

    #[test]
    fn self_roadmap_fixture_parses_cleanly() {
        let doc = parse(SELF, F);
        assert!(
            doc.entries.len() >= 7,
            "the seven roadmap specs are entries"
        );
        assert!(
            !doc.names().is_empty(),
            "self roadmap yields known feature names"
        );
    }
}
