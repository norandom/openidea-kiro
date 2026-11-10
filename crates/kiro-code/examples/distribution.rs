//! Calibration aid (code-constraints 1.4): the distribution of every
//! metric over one or more repositories, per language, next to the
//! built-in limit. It reads the repositories and writes nothing into them.
//!
//! ```text
//! cargo run --release -p kiro-code --example distribution -- <root>...
//! ```
//!
//! Output: one Markdown table row per repository, language, scope and
//! metric with the count, the 50th, 90th, 95th and 99th percentile, the
//! maximum, the built-in limit and the share of entities above it.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use kiro_code::{measure, profile_for, read_bundle};
use kiro_core::code::config::CodeConfig;

/// Nearest-rank percentile of a sorted, non-empty list.
fn percentile(sorted: &[f64], percent: usize) -> f64 {
    let rank = (sorted.len() * percent).div_ceil(100).max(1);
    sorted.get(rank - 1).copied().unwrap_or(f64::NAN)
}

fn number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
}

fn report(root: &Path, config: &CodeConfig) -> Result<(), String> {
    let canonical = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let bundle = read_bundle(&canonical, &|path| profile_for(path).is_some())
        .map_err(|error| error.to_string())?;
    let facts = measure(&bundle);
    let name = canonical
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut files: BTreeMap<&str, usize> = BTreeMap::new();
    for file in &bundle.files {
        if let Some(profile) = profile_for(&file.path) {
            *files.entry(profile.name).or_default() += 1;
        }
    }
    let listed: Vec<String> = files
        .iter()
        .map(|(language, count)| format!("{count} {language}"))
        .collect();
    eprintln!(
        "{name}: {} files ({})",
        bundle.files.len(),
        listed.join(", ")
    );
    // `KIRO_LIST_FILES=1` lists what was read, to check the corpus.
    if std::env::var_os("KIRO_LIST_FILES").is_some() {
        for file in &bundle.files {
            eprintln!("  {}", file.path);
        }
    }

    // (language, scope, metric) -> values and the limit.
    type Values = (Vec<f64>, Option<f64>);
    let mut groups: BTreeMap<(&str, String, &str), Values> = BTreeMap::new();
    for observation in &facts.observations {
        let Some(profile) = profile_for(&observation.entity.path) else {
            continue;
        };
        let scope = observation.entity.scope;
        let metric = observation.metric.name.as_str();
        let entry = groups
            .entry((profile.name, format!("{scope:?}").to_lowercase(), metric))
            .or_insert_with(|| {
                let limit = config
                    .limit_for(scope, metric, &observation.entity.path)
                    .map(|(limit, _)| limit.maximum);
                (Vec::new(), limit)
            });
        entry.0.push(observation.value);
    }
    for ((language, scope, metric), (mut values, limit)) in groups {
        values.sort_by(f64::total_cmp);
        let above = limit.map(|limit| values.iter().filter(|value| **value > limit).count());
        let share = above.map_or_else(
            || "-".to_string(),
            |above| format!("{:.1}%", 100.0 * above as f64 / values.len() as f64),
        );
        println!(
            "| {name} | {language} | {scope} | {metric} | {} | {} | {} | {} | {} | {} | {} | {share} |",
            values.len(),
            number(percentile(&values, 50)),
            number(percentile(&values, 90)),
            number(percentile(&values, 95)),
            number(percentile(&values, 99)),
            number(values.last().copied().unwrap_or(f64::NAN)),
            limit.map_or_else(|| "-".to_string(), number),
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    let roots: Vec<String> = std::env::args().skip(1).collect();
    if roots.is_empty() {
        eprintln!("usage: distribution <root>...");
        return ExitCode::from(2);
    }
    println!(
        "| Repository | Language | Scope | Metric | n | p50 | p90 | p95 | p99 | max | limit | above |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    let config = CodeConfig::default();
    for root in &roots {
        if let Err(error) = report(Path::new(root), &config) {
            eprintln!("error: {error}");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}
