//! Run identity for telemetry: resource attributes, timing and identifiers
//! (quality-telemetry 3.7, 3.8, 4.3).
//!
//! Only the values named here are read from the environment or from Git.
//! Nothing here writes a file or opens a connection.

use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use kiro_core::telemetry::{Phase, ResourceIdentity, RunIdentity};

use crate::exit::Failure;
use crate::fs::ProjectRoot;

pub const ENV_TELEMETRY_DIR: &str = "KIRO_TELEMETRY_DIR";
pub const ENV_RUN_KIND: &str = "KIRO_RUN_KIND";
pub const ENV_SERVICE_NAME: &str = "OTEL_SERVICE_NAME";
pub const ENV_RESOURCE_ATTRIBUTES: &str = "OTEL_RESOURCE_ATTRIBUTES";

const RUN_KINDS: &[&str] = &["hook", "agent", "ci", "manual"];

/// The environment values telemetry may read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TelemetryEnvironment {
    pub dir: Option<String>,
    pub run_kind: Option<String>,
    pub service_name: Option<String>,
    pub resource_attributes: Option<String>,
}

impl TelemetryEnvironment {
    #[must_use]
    pub fn capture() -> Self {
        let read = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
        Self {
            dir: read(ENV_TELEMETRY_DIR),
            run_kind: read(ENV_RUN_KIND),
            service_name: read(ENV_SERVICE_NAME),
            resource_attributes: read(ENV_RESOURCE_ATTRIBUTES),
        }
    }
}

/// Validates `KIRO_RUN_KIND`; the default is `manual`.
pub fn run_kind(environment: &TelemetryEnvironment) -> Result<&str, Failure> {
    match environment.run_kind.as_deref() {
        None => Ok("manual"),
        Some(kind) if RUN_KINDS.contains(&kind) => Ok(kind),
        Some(_) => Err(Failure::new(format!(
            "invalid value in environment variable {ENV_RUN_KIND}; expected one of: {}",
            RUN_KINDS.join(", ")
        ))),
    }
}

/// Removes `user:password@` from a repository address.
#[must_use]
pub fn strip_credentials(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let authority_end = rest.find('/').unwrap_or(rest.len());
    let (authority, path) = rest.split_at(authority_end);
    let host = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    format!("{scheme}://{host}{path}")
}

/// One read-only Git query in the project; `None` when Git is absent, the
/// project is no repository or the answer is empty.
fn git(root: &ProjectRoot, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root.path())
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() { None } else { Some(text) }
}

/// Git values of the project, each omitted when unavailable.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitIdentity {
    pub repository: Option<String>,
    pub branch: Option<String>,
    pub revision: Option<String>,
}

impl GitIdentity {
    #[must_use]
    pub fn read(root: &ProjectRoot) -> Self {
        Self {
            repository: git(root, &["config", "--get", "remote.origin.url"])
                .map(|url| strip_credentials(&url)),
            branch: git(root, &["rev-parse", "--abbrev-ref", "HEAD"]),
            revision: git(root, &["rev-parse", "HEAD"]),
        }
    }
}

/// Splits `OTEL_RESOURCE_ATTRIBUTES` (`key=value,key=value`); malformed
/// pairs are skipped.
fn operator_attributes(text: &str) -> Vec<(String, String)> {
    text.split(',')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .filter(|(key, _)| !key.is_empty())
        .collect()
}

/// Builds the resource attributes of a run. Later entries never replace
/// earlier ones, so the operator's attributes cannot overwrite identity.
#[must_use]
pub fn resource_identity(
    environment: &TelemetryEnvironment,
    git: &GitIdentity,
    kind: &str,
) -> ResourceIdentity {
    let mut attributes: Vec<(String, String)> = vec![
        (
            "service.name".to_string(),
            environment
                .service_name
                .clone()
                .unwrap_or_else(|| "kiro-cli".to_string()),
        ),
        (
            "service.version".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        ),
    ];
    let optional = [
        ("vcs.repository.url.full", &git.repository),
        ("vcs.ref.head.name", &git.branch),
        ("vcs.ref.head.revision", &git.revision),
    ];
    for (key, value) in optional {
        if let Some(value) = value {
            attributes.push((key.to_string(), value.clone()));
        }
    }
    attributes.push(("kiro.run.kind".to_string(), kind.to_string()));
    if let Some(text) = &environment.resource_attributes {
        for (key, value) in operator_attributes(text) {
            if !attributes.iter().any(|(existing, _)| *existing == key) {
                attributes.push((key, value));
            }
        }
    }
    ResourceIdentity { attributes }
}

/// FNV-1a over `bytes`, started from `seed`.
fn fnv1a(seed: u64, bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64 ^ seed;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Wall-clock start plus a monotonic clock for the phases of one run.
#[derive(Debug)]
pub struct RunTimer {
    start_unix_nanos: u64,
    started: Instant,
    last: Instant,
    phases: Vec<Phase>,
}

impl RunTimer {
    #[must_use]
    pub fn start() -> Self {
        let now = Instant::now();
        let start_unix_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| u64::try_from(elapsed.as_nanos()).ok())
            .unwrap_or(0);
        Self {
            start_unix_nanos,
            started: now,
            last: now,
            phases: Vec::new(),
        }
    }

    fn offset(&self, instant: Instant) -> u64 {
        let nanos = instant.duration_since(self.started).as_nanos();
        self.start_unix_nanos
            .saturating_add(u64::try_from(nanos).unwrap_or(u64::MAX))
    }

    /// Closes the phase that ran since the previous call (or since start).
    pub fn phase(&mut self, name: &'static str) {
        let now = Instant::now();
        self.phases.push(Phase {
            name,
            start_unix_nanos: self.offset(self.last),
            end_unix_nanos: self.offset(now),
        });
        self.last = now;
    }

    /// Finishes the run. Identifiers are derived from the start time, the
    /// process identifier and the revision; no random source is used.
    #[must_use]
    pub fn finish(self, revision: Option<&str>, verdict: &str) -> RunIdentity {
        let mut material = Vec::new();
        material.extend_from_slice(&self.start_unix_nanos.to_be_bytes());
        material.extend_from_slice(&std::process::id().to_be_bytes());
        material.extend_from_slice(revision.unwrap_or("").as_bytes());
        let high = fnv1a(1, &material);
        let low = fnv1a(2, &material);
        let span = fnv1a(3, &material);
        let mut trace_id = [0_u8; 16];
        for (target, source) in trace_id
            .iter_mut()
            .zip(high.to_be_bytes().into_iter().chain(low.to_be_bytes()))
        {
            *target = source;
        }
        if trace_id == [0; 16] {
            trace_id = [1; 16];
        }
        let root_span_id = if span == 0 {
            [1; 8]
        } else {
            span.to_be_bytes()
        };
        RunIdentity {
            trace_id,
            root_span_id,
            start_unix_nanos: self.start_unix_nanos,
            end_unix_nanos: self.offset(self.last),
            phases: self.phases,
            verdict: verdict.to_string(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn credentials_are_removed_from_repository_addresses() {
        assert_eq!(
            strip_credentials("https://user:secret@example.com/org/repo.git"),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            strip_credentials("https://token@example.com/org/repo.git"),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            strip_credentials("https://example.com/org/repo.git"),
            "https://example.com/org/repo.git"
        );
        assert_eq!(
            strip_credentials("git@example.com:org/repo.git"),
            "git@example.com:org/repo.git",
            "scp form carries a login name, not a secret"
        );
        assert_eq!(
            strip_credentials("https://u:p@example.com/a@b/c"),
            "https://example.com/a@b/c",
            "an @ in the path is kept"
        );
    }

    #[test]
    fn run_kind_default_valid_and_invalid() {
        let mut environment = TelemetryEnvironment::default();
        assert_eq!(run_kind(&environment).unwrap(), "manual");
        environment.run_kind = Some("ci".into());
        assert_eq!(run_kind(&environment).unwrap(), "ci");
        environment.run_kind = Some("nightly".into());
        let failure = run_kind(&environment).unwrap_err();
        assert!(failure.0.contains("KIRO_RUN_KIND"));
        assert!(!failure.0.contains("nightly"));
    }

    #[test]
    fn resource_identity_order_and_operator_attributes() {
        let environment = TelemetryEnvironment {
            service_name: Some("my-gate".into()),
            resource_attributes: Some(
                "team=core, service.name=spoof ,broken,deployment.environment=ci".into(),
            ),
            ..TelemetryEnvironment::default()
        };
        let git = GitIdentity {
            repository: Some("https://example.com/r.git".into()),
            branch: Some("main".into()),
            revision: None,
        };
        let identity = resource_identity(&environment, &git, "ci");
        let keys: Vec<&str> = identity
            .attributes
            .iter()
            .map(|(key, _)| key.as_str())
            .collect();
        assert_eq!(
            keys,
            [
                "service.name",
                "service.version",
                "vcs.repository.url.full",
                "vcs.ref.head.name",
                "kiro.run.kind",
                "team",
                "deployment.environment",
            ]
        );
        assert_eq!(
            identity.attributes[0].1, "my-gate",
            "identity is not overwritten"
        );
    }

    #[test]
    fn timer_produces_ordered_phases_and_nonzero_identifiers() {
        let mut timer = RunTimer::start();
        timer.phase("discover");
        timer.phase("check");
        let run = timer.finish(Some("abc123"), "pass");
        assert_eq!(run.phases.len(), 2);
        assert_eq!(run.phases[0].start_unix_nanos, run.start_unix_nanos);
        assert!(run.phases[0].end_unix_nanos <= run.phases[1].start_unix_nanos);
        assert_eq!(run.phases[1].end_unix_nanos, run.end_unix_nanos);
        assert_ne!(run.trace_id, [0; 16]);
        assert_ne!(run.root_span_id, [0; 8]);
        assert_eq!(run.verdict, "pass");
    }
}
