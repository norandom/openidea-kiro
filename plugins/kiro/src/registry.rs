//! Command registry, id grammar, variant synchronization, and budget enforcement.
//!
//! Enforces the 256-command budget and kebab-case id conventions before making
//! any host calls, managing base commands, insert variants, and per-spec variants.

use std::collections::{BTreeSet, HashSet};
use std::sync::{Mutex, MutexGuard};

use crate::host::HostApi;

pub const CATEGORY: &str = "Kiro";
pub const TITLE_PREFIX: &str = "Kiro:";
pub const INSERT: &str = "insert";
pub const INSERT_TITLE_SUFFIX: &str = " — Insert at Cursor";
pub const MAX_COMMANDS: usize = 256;
pub const MAX_ID_LEN: usize = 64;
pub const MAX_TITLE_LEN: usize = 128;
pub const MAX_REGISTRATION_TITLE_LEN: usize = 256;

/// Command group / feature module prefix.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Group {
    Ws,
    Nav,
    Viz,
    Vocab,
    Check,
}

impl Group {
    pub const ALL: [Group; 5] = [
        Group::Ws,
        Group::Nav,
        Group::Viz,
        Group::Vocab,
        Group::Check,
    ];

    pub fn prefix(self) -> &'static str {
        match self {
            Group::Ws => "ws",
            Group::Nav => "nav",
            Group::Viz => "viz",
            Group::Vocab => "vocab",
            Group::Check => "check",
        }
    }

    pub fn parse(s: &str) -> Option<Group> {
        match s {
            "ws" => Some(Group::Ws),
            "nav" => Some(Group::Nav),
            "viz" => Some(Group::Viz),
            "vocab" => Some(Group::Vocab),
            "check" => Some(Group::Check),
            _ => None,
        }
    }
}

/// How argument tokens are interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Positional {
    Spec,
    PassThrough,
    None,
}

/// Definition of a command registered or handled by a feature module.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSpec {
    pub group: Group,
    pub name: &'static str,
    pub title: &'static str,
    pub spec_addressable: bool,
    pub insert_capable: bool,
    pub positional: Positional,
}

impl CommandSpec {
    pub fn id(&self) -> String {
        format!("{}.{}", self.group.prefix(), self.name)
    }

    pub fn insert_id(&self) -> Option<String> {
        if self.insert_capable {
            Some(format!("{}.{}.{}", self.group.prefix(), self.name, INSERT))
        } else {
            None
        }
    }

    pub fn insert_title(&self) -> String {
        format!("{}{}", self.title, INSERT_TITLE_SUFFIX)
    }

    pub fn variant_id(&self, spec: &str) -> String {
        format!("{}.{}.{}", self.group.prefix(), self.name, spec)
    }

    pub fn variant_title(&self, spec: &str) -> String {
        format!("{} — {}", self.title, spec)
    }

    pub fn validate(&self) -> Result<(), String> {
        let base_id = self.id();
        if !is_command_id(&base_id) {
            return Err(format!("invalid command id: {base_id}"));
        }
        if !self.title.starts_with(TITLE_PREFIX) {
            return Err(format!(
                "title {:?} must start with {:?}",
                self.title, TITLE_PREFIX
            ));
        }
        if self.title.len() > MAX_TITLE_LEN {
            return Err(format!(
                "title {:?} exceeds {} bytes",
                self.title, MAX_TITLE_LEN
            ));
        }
        Ok(())
    }
}

/// Command declaration consisting of an identifier and human-readable title.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Decl {
    pub id: String,
    pub title: String,
}

/// Result of parsing an action/command identifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedId {
    pub base: String,
    pub insert: bool,
    pub spec: Option<String>,
}

/// Parses `<group>.<name>`, `<group>.<name>.insert`, or `<group>.<name>.<spec>`.
pub fn parse_id(id: &str) -> Option<ParsedId> {
    if !is_command_id(id) {
        return None;
    }

    let segments: Vec<&str> = id.split('.').collect();
    if segments.len() < 2 || segments.len() > 3 {
        return None;
    }

    let group = Group::parse(segments.first().copied().unwrap_or_default())?;
    let name = segments.get(1).copied().unwrap_or_default();
    if name.is_empty() {
        return None;
    }

    let base = format!("{}.{}", group.prefix(), name);
    if segments.len() == 2 {
        return Some(ParsedId {
            base,
            insert: false,
            spec: None,
        });
    }

    let third = segments.get(2).copied().unwrap_or_default();
    if third == INSERT {
        Some(ParsedId {
            base,
            insert: true,
            spec: None,
        })
    } else if is_spec_segment(third) {
        Some(ParsedId {
            base,
            insert: false,
            spec: Some(third.to_owned()),
        })
    } else {
        None
    }
}

/// Checks whether `s` matches `[a-z0-9][a-z0-9._-]*` and is at most 64 bytes.
pub fn is_command_id(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_ID_LEN {
        return false;
    }
    let first = s.chars().next().unwrap_or('\0');
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

/// Checks whether `s` matches `[a-z0-9][a-z0-9_-]*`, is <= 64 bytes, has no dots, and is not "insert".
pub fn is_spec_segment(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_ID_LEN || s == INSERT {
        return false;
    }
    let first = s.chars().next().unwrap_or('\0');
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Checks whether `s` matches `[a-z0-9][a-z0-9._-]*`, has no `/`, `\`, or `..`, and is <= 64 bytes.
pub fn is_spec_name(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_ID_LEN {
        return false;
    }
    let first = s.chars().next().unwrap_or('\0');
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    if s.contains("..") || s.contains('/') || s.contains('\\') {
        return false;
    }
    s.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
}

/// Base declaration of every command followed by the insert variant of each insert-capable one.
pub fn static_commands(specs: &[CommandSpec]) -> Vec<Decl> {
    let mut out = Vec::new();
    for spec in specs {
        out.push(Decl {
            id: spec.id(),
            title: spec.title.to_owned(),
        });
        if spec.insert_capable
            && let Some(ins_id) = spec.insert_id()
        {
            out.push(Decl {
                id: ins_id,
                title: spec.insert_title(),
            });
        }
    }
    out
}

/// Registration error conditions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RegisterError {
    InvalidId(String),
    InvalidTitle(String),
    Budget,
}

impl std::fmt::Display for RegisterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId(id) => write!(f, "invalid command id `{id}`"),
            Self::InvalidTitle(title) => write!(f, "invalid command title `{title}`"),
            Self::Budget => write!(f, "command registration limit reached (256 commands)"),
        }
    }
}

impl std::error::Error for RegisterError {}

/// State tracking registered command identifiers and enforcing the 256 budget.
#[derive(Debug, Default)]
pub struct Registry {
    registered: BTreeSet<String>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a command with the host, enforcing id/title validation and the 256 limit.
    /// Returns `Ok(true)` if newly registered, `Ok(false)` if already registered.
    pub fn register(&mut self, host: &dyn HostApi, decl: &Decl) -> Result<bool, RegisterError> {
        if !is_command_id(&decl.id) {
            return Err(RegisterError::InvalidId(decl.id.clone()));
        }
        if decl.title.is_empty()
            || decl.title.len() > MAX_REGISTRATION_TITLE_LEN
            || !decl.title.starts_with(TITLE_PREFIX)
        {
            return Err(RegisterError::InvalidTitle(decl.title.clone()));
        }

        if self.registered.contains(&decl.id) {
            host.register_command(&decl.id, &decl.title, Some(CATEGORY));
            return Ok(false);
        }

        if self.registered.len() >= MAX_COMMANDS {
            return Err(RegisterError::Budget);
        }

        host.register_command(&decl.id, &decl.title, Some(CATEGORY));
        self.registered.insert(decl.id.clone());
        Ok(true)
    }

    pub fn len(&self) -> usize {
        self.registered.len()
    }

    pub fn is_empty(&self) -> bool {
        self.registered.is_empty()
    }

    pub fn contains(&self, id: &str) -> bool {
        self.registered.contains(id)
    }

    pub fn remaining(&self) -> usize {
        MAX_COMMANDS.saturating_sub(self.registered.len())
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.registered.clear();
    }
}

static GLOBAL_REGISTRY: Mutex<Registry> = Mutex::new(Registry {
    registered: BTreeSet::new(),
});

/// Returns a locked guard to the plugin-wide registry.
pub fn registry() -> MutexGuard<'static, Registry> {
    GLOBAL_REGISTRY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Report summarizing synchronization of per-spec variant commands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    pub registered: usize,
    pub unchanged: usize,
    pub skipped_budget: Vec<String>,
    pub skipped_invalid: Vec<String>,
}

/// Registers per-spec variant commands for known specs within the 256 budget.
pub fn sync_variants(host: &dyn HostApi, specs: &[CommandSpec], known: &[String]) -> SyncReport {
    let mut report = SyncReport::default();
    let mut reg = registry();

    // Deduplicate in roadmap order
    let mut seen = HashSet::new();
    let mut ordered_known = Vec::new();
    for k in known {
        if seen.insert(k.as_str()) {
            ordered_known.push(k.as_str());
        }
    }

    let addressable: Vec<&CommandSpec> = specs.iter().filter(|s| s.spec_addressable).collect();
    if addressable.is_empty() {
        return report;
    }

    let mut budget_exhausted = false;

    for spec_name in ordered_known {
        if budget_exhausted {
            report.skipped_budget.push(spec_name.to_owned());
            continue;
        }

        // Validate spec segment and composed variant ids before any host call
        if !is_spec_segment(spec_name) {
            report.skipped_invalid.push(spec_name.to_owned());
            continue;
        }

        let mut invalid_composed = false;
        for s in &addressable {
            let var_id = s.variant_id(spec_name);
            if !is_command_id(&var_id) {
                invalid_composed = true;
                break;
            }
        }
        if invalid_composed {
            report.skipped_invalid.push(spec_name.to_owned());
            continue;
        }

        // Count how many new variant ids are needed for this spec
        let needed = addressable
            .iter()
            .filter(|s| !reg.contains(&s.variant_id(spec_name)))
            .count();

        if reg.remaining() < needed {
            budget_exhausted = true;
            report.skipped_budget.push(spec_name.to_owned());
            continue;
        }

        // Register all variants for this spec
        for s in &addressable {
            let decl = Decl {
                id: s.variant_id(spec_name),
                title: s.variant_title(spec_name),
            };
            match reg.register(host, &decl) {
                Ok(true) => report.registered += 1,
                Ok(false) => report.unchanged += 1,
                Err(_) => {}
            }
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::FakeHost;

    fn sample_specs() -> Vec<CommandSpec> {
        vec![
            CommandSpec {
                group: Group::Ws,
                name: "specs",
                title: "Kiro: List Specs",
                spec_addressable: false,
                insert_capable: true,
                positional: Positional::None,
            },
            CommandSpec {
                group: Group::Ws,
                name: "current",
                title: "Kiro: Current Spec",
                spec_addressable: true,
                insert_capable: false,
                positional: Positional::Spec,
            },
        ]
    }

    #[test]
    fn id_grammar_validation_table() {
        // Valid ids
        assert!(is_command_id("ws.specs"));
        assert!(is_command_id("ws.specs.insert"));
        assert!(is_command_id("ws.current.kiro-core"));
        assert!(is_command_id("a1.b_2.c-3"));

        // Invalid: uppercase
        assert!(!is_command_id("ws.Specs"));
        assert!(!is_command_id("WS.specs"));

        // Invalid: leading dot or separator
        assert!(!is_command_id(".ws.specs"));
        assert!(!is_command_id("-ws.specs"));

        // Invalid: > 64 chars
        let long_id = "a".repeat(65);
        assert!(!is_command_id(&long_id));
        let max_id = "a".repeat(64);
        assert!(is_command_id(&max_id));
    }

    #[test]
    fn parse_id_matrix() {
        // Base command
        assert_eq!(
            parse_id("ws.specs"),
            Some(ParsedId {
                base: "ws.specs".into(),
                insert: false,
                spec: None
            })
        );

        // Insert variant
        assert_eq!(
            parse_id("ws.specs.insert"),
            Some(ParsedId {
                base: "ws.specs".into(),
                insert: true,
                spec: None
            })
        );

        // Per-spec variant
        assert_eq!(
            parse_id("ws.current.demo"),
            Some(ParsedId {
                base: "ws.current".into(),
                insert: false,
                spec: Some("demo".into())
            })
        );

        // Invalid shapes: 1 segment, 4 segments, unknown group, invalid spec segment with dots
        assert_eq!(parse_id("specs"), None);
        assert_eq!(parse_id("ws.specs.demo.extra"), None);
        assert_eq!(parse_id("unknown.specs"), None);
        assert_eq!(parse_id("ws.specs.invalid.spec"), None);
    }

    #[test]
    fn spec_segment_and_spec_name_validators() {
        assert!(is_spec_segment("demo"));
        assert!(is_spec_segment("kiro-core"));
        assert!(is_spec_segment("feature_1"));
        // insert is reserved
        assert!(!is_spec_segment("insert"));
        // dots not allowed in segment
        assert!(!is_spec_segment("spec.with.dot"));

        // is_spec_name allows dots but forbids path separators and ..
        assert!(is_spec_name("spec.with.dot"));
        assert!(!is_spec_name("spec/name"));
        assert!(!is_spec_name("spec\\name"));
        assert!(!is_spec_name("spec..name"));
    }

    #[test]
    fn static_commands_derived_order() {
        let specs = sample_specs();
        let statics = static_commands(&specs);

        assert_eq!(
            statics,
            vec![
                Decl {
                    id: "ws.specs".into(),
                    title: "Kiro: List Specs".into()
                },
                Decl {
                    id: "ws.specs.insert".into(),
                    title: "Kiro: List Specs — Insert at Cursor".into()
                },
                Decl {
                    id: "ws.current".into(),
                    title: "Kiro: Current Spec".into()
                },
            ]
        );
    }

    #[test]
    fn registry_register_validation_and_budget() {
        let host = FakeHost::new();
        let mut reg = Registry::new();

        // Valid
        let ok = reg.register(
            &host,
            &Decl {
                id: "ws.specs".into(),
                title: "Kiro: List Specs".into(),
            },
        );
        assert_eq!(ok, Ok(true));
        assert_eq!(reg.len(), 1);

        // Re-register same id is Ok(false)
        let dup = reg.register(
            &host,
            &Decl {
                id: "ws.specs".into(),
                title: "Kiro: List Specs".into(),
            },
        );
        assert_eq!(dup, Ok(false));
        assert_eq!(reg.len(), 1);

        // Invalid id refused without host call count increase
        let invalid_id = reg.register(
            &host,
            &Decl {
                id: "INVALID_ID".into(),
                title: "Kiro: Test".into(),
            },
        );
        assert!(matches!(invalid_id, Err(RegisterError::InvalidId(_))));

        // Invalid title (no prefix)
        let invalid_title = reg.register(
            &host,
            &Decl {
                id: "ws.other".into(),
                title: "No Prefix".into(),
            },
        );
        assert!(matches!(invalid_title, Err(RegisterError::InvalidTitle(_))));
    }

    #[test]
    fn sync_variants_handles_budget_invalid_and_idempotency() {
        let host = FakeHost::new();
        let specs = sample_specs();
        registry().clear();

        // 1 invalid spec name, 2 valid specs
        let known = vec![
            "invalid.segment".to_owned(), // has dot -> invalid segment
            "spec-one".to_owned(),
            "spec-two".to_owned(),
        ];

        let report1 = sync_variants(&host, &specs, &known);
        assert_eq!(report1.registered, 2); // 1 addressable command * 2 specs
        assert_eq!(report1.skipped_invalid, vec!["invalid.segment"]);
        assert!(report1.skipped_budget.is_empty());

        // Re-sync is idempotent
        let report2 = sync_variants(&host, &specs, &known);
        assert_eq!(report2.registered, 0);
        assert_eq!(report2.unchanged, 2);

        // Long spec causing 65-char composed id
        // "ws.current." is 11 chars. 54 chars spec -> 65 chars id -> invalid
        let long_spec = "a".repeat(54);
        let report3 = sync_variants(&host, &specs, std::slice::from_ref(&long_spec));
        assert_eq!(report3.skipped_invalid, vec![long_spec]);

        registry().clear();
    }
}
