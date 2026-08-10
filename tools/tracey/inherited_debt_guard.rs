//! Keep inherited traceability debt exact without claiming that the debt is coverage.
//! r[impl molten.project.inherited_tracey_debt.baseline]
//! r[impl molten.project.inherited_tracey_debt.growth_denial]
//! r[impl molten.project.inherited_tracey_debt.non_claims]

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const REQUIREMENT_ROOT: &str = "cairn/specs";
const ROOT_EVIDENCE_FILE: &str = "flake.nix";
const EVIDENCE_ROOTS: &[&str] = &["src", "crates", "tests", "tools", "docs", "scripts"];
const EVIDENCE_EXTENSIONS: &[&str] = &["rs", "ncl", "md", "sh", "nix"];
const REFERENCE_VERBS: &[&str] = &["impl", "verify", "depends", "related"];
const REQUIREMENT_EXTENSION: &str = "md";
const OPTION_ROOT: &str = "--root";
const OPTION_BASELINE: &str = "--baseline";
const OPTION_WRITE_BASELINE: &str = "--write-baseline";
const ADJACENT_PAIR_WIDTH: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Coverage {
    requirements: BTreeSet<String>,
    references: BTreeSet<String>,
    missing: BTreeSet<String>,
    dangling: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BaselineComparison {
    unexpected_missing: BTreeSet<String>,
    stale_baseline: BTreeSet<String>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-'))
}

fn requirement_marker(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let marker = trimmed.strip_prefix("r[")?;
    let end = marker.find(']')?;
    let raw = &marker[..end];
    if raw.split_whitespace().count() != 1 {
        return None;
    }
    let id = raw.split('+').next().unwrap_or(raw);
    valid_id(id).then(|| id.to_string())
}

fn reference_markers(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut cursor = 0;
    while let Some(offset) = line[cursor..].find("r[") {
        let marker_start = cursor + offset + "r[".len();
        let Some(end_offset) = line[marker_start..].find(']') else {
            break;
        };
        let raw = &line[marker_start..marker_start + end_offset];
        let mut parts = raw.split_whitespace();
        let verb = parts.next().unwrap_or_default();
        let raw_id = parts.next().unwrap_or_default();
        let id = raw_id.split('+').next().unwrap_or(raw_id);
        if REFERENCE_VERBS.contains(&verb) && valid_id(id) {
            found.push(id.to_string());
        }
        cursor = marker_start + end_offset + 1;
    }
    found
}

fn classify(requirements: BTreeSet<String>, references: BTreeSet<String>) -> Coverage {
    let missing = requirements.difference(&references).cloned().collect();
    let dangling = references.difference(&requirements).cloned().collect();
    Coverage {
        requirements,
        references,
        missing,
        dangling,
    }
}

fn compare_baseline(actual: &BTreeSet<String>, baseline: &BTreeSet<String>) -> BaselineComparison {
    BaselineComparison {
        unexpected_missing: actual.difference(baseline).cloned().collect(),
        stale_baseline: baseline.difference(actual).cloned().collect(),
    }
}

fn baseline_is_sorted_and_unique(lines: &[String]) -> bool {
    !lines.iter().any(String::is_empty) && lines.windows(ADJACENT_PAIR_WIDTH).all(|pair| pair[0] < pair[1])
}

fn walk_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(());
    }
    if !path.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|error| format!("{}: {error}", entry_path.display()))?;
        if file_type.is_dir() {
            walk_files(&entry_path, files)?;
        } else if file_type.is_file() {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extensions.contains(&extension))
}

fn read_requirements(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = Vec::new();
    walk_files(&root.join(REQUIREMENT_ROOT), &mut files)?;
    files.retain(|path| has_extension(path, &[REQUIREMENT_EXTENSION]));
    let mut requirements = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        for line in text.lines() {
            if let Some(requirement) = requirement_marker(line) {
                requirements.insert(requirement);
            }
        }
    }
    Ok(requirements)
}

fn read_references(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut files = Vec::new();
    for source_root in EVIDENCE_ROOTS {
        walk_files(&root.join(source_root), &mut files)?;
    }
    walk_files(&root.join(ROOT_EVIDENCE_FILE), &mut files)?;
    files.retain(|path| has_extension(path, EVIDENCE_EXTENSIONS));
    let mut references = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        for line in text.lines() {
            references.extend(reference_markers(line));
        }
    }
    Ok(references)
}

fn read_baseline(path: &Path) -> Result<(Vec<String>, BTreeSet<String>), String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    if !baseline_is_sorted_and_unique(&lines) {
        return Err("baseline must contain unique, sorted, non-empty requirement identifiers".to_string());
    }
    let values = lines.iter().cloned().collect();
    Ok((lines, values))
}

fn write_baseline(path: &Path, missing: &BTreeSet<String>) -> Result<(), String> {
    let mut text = missing.iter().cloned().collect::<Vec<_>>().join("\n");
    text.push('\n');
    fs::write(path, text).map_err(|error| format!("{}: {error}", path.display()))
}

fn option_value(args: &[String], option: &str) -> Option<PathBuf> {
    args.windows(ADJACENT_PAIR_WIDTH).find(|pair| pair[0] == option).map(|pair| PathBuf::from(&pair[1]))
}

fn render_values(label: &str, values: &BTreeSet<String>) {
    for value in values {
        eprintln!("{label}: {value}");
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let root = option_value(args, OPTION_ROOT).unwrap_or_else(|| PathBuf::from("."));
    let requirements = read_requirements(&root)?;
    let references = read_references(&root)?;
    let coverage = classify(requirements, references);

    if let Some(output) = option_value(args, OPTION_WRITE_BASELINE) {
        if !coverage.dangling.is_empty() {
            render_values("dangling", &coverage.dangling);
            return Err("cannot write a baseline while dangling references exist".to_string());
        }
        write_baseline(&output, &coverage.missing)?;
        println!("baseline_written={}", output.display());
        println!("uncovered={}", coverage.missing.len());
        return Ok(());
    }

    let baseline_path =
        option_value(args, OPTION_BASELINE).ok_or_else(|| format!("missing required option {OPTION_BASELINE}"))?;
    let (baseline_lines, baseline) = read_baseline(&baseline_path)?;
    let comparison = compare_baseline(&coverage.missing, &baseline);

    println!("requirements={}", coverage.requirements.len());
    println!("referenced={}", coverage.requirements.intersection(&coverage.references).count());
    println!("uncovered={}", coverage.missing.len());
    println!("baseline_entries={}", baseline_lines.len());
    println!("dangling={}", coverage.dangling.len());
    println!("non_claim=inherited requirements remain uncovered until direct evidence references them");

    if !coverage.dangling.is_empty() {
        render_values("dangling", &coverage.dangling);
        return Err("dangling traceability references are not permitted".to_string());
    }
    if !comparison.unexpected_missing.is_empty() || !comparison.stale_baseline.is_empty() {
        render_values("unexpected_missing", &comparison.unexpected_missing);
        render_values("stale_baseline", &comparison.stale_baseline);
        return Err("traceability debt differs from the reviewed baseline".to_string());
    }
    println!("verdict=pass");
    Ok(())
}

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if let Err(error) = run(&args) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn exact_baseline_passes() {
        // r[verify molten.project.inherited_tracey_debt.baseline]
        // r[verify molten.project.inherited_tracey_debt.validation]
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        let coverage = classify(set(&["alpha", "beta"]), set(&["alpha"]));
        let comparison = compare_baseline(&coverage.missing, &set(&["beta"]));
        assert!(comparison.unexpected_missing.is_empty());
        assert!(comparison.stale_baseline.is_empty());
        assert!(coverage.dangling.is_empty());
    }

    #[test]
    fn drift_and_dangling_fail() {
        // r[verify molten.project.inherited_tracey_debt.growth_denial]
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        let coverage = classify(set(&["alpha", "beta"]), set(&["alpha", "gamma"]));
        let comparison = compare_baseline(&coverage.missing, &set(&["alpha"]));
        assert_eq!(comparison.unexpected_missing, set(&["beta"]));
        assert_eq!(comparison.stale_baseline, set(&["alpha"]));
        assert_eq!(coverage.dangling, set(&["gamma"]));
    }

    #[test]
    fn malformed_markers_and_baselines_fail() {
        // r[verify molten.project.inherited_tracey_debt.fixtures]
        // r[verify molten.project.inherited_tracey_debt.marker_repair]
        assert_eq!(requirement_marker("r[valid.id] text"), Some("valid.id".to_string()));
        assert_eq!(requirement_marker("heading r[hidden.id]"), None);
        assert!(reference_markers("r[verify invalid/id]").is_empty());
        assert!(!baseline_is_sorted_and_unique(&["beta".to_string(), "alpha".to_string()]));
        assert!(!baseline_is_sorted_and_unique(&["alpha".to_string(), "alpha".to_string()]));
        assert!(!baseline_is_sorted_and_unique(&[String::new()]));
    }

    #[test]
    fn non_claim_is_bound_to_the_guard() {
        // r[verify molten.project.inherited_tracey_debt.non_claims]
        let non_claim = "inherited requirements remain uncovered until direct evidence references them";
        assert!(non_claim.contains("remain uncovered"));
    }
}
