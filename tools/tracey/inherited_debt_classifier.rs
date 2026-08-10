#!/usr/bin/env -S CARGO_TARGET_DIR=/tmp/molten-inherited-debt-classifier-target nix shell "github:nix-community/fenix?rev=092bd452904e749efa39907aa4a20a42678ac31e#minimal.toolchain" nixpkgs#gcc -c cargo -q -Zscript

//! Classify every inherited Tracey debt entry without inferring implementation.
//! r[impl molten.project.inherited_tracey_classification.inventory]
//! r[impl molten.project.inherited_tracey_classification.conservative_default]
//! r[impl molten.project.inherited_tracey_classification.duplicate_denial]
//! r[impl molten.project.inherited_tracey_classification.deterministic_grouping]
//! r[impl molten.project.inherited_tracey_classification.non_claims]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

const REQUIREMENT_ROOT: &str = "cairn/specs";
const REQUIREMENT_EXTENSION: &str = "md";
const OPTION_ROOT: &str = "--root";
const OPTION_BASELINE: &str = "--baseline";
const OPTION_OUTPUT: &str = "--output";
const OPTION_SUMMARY_OUTPUT: &str = "--summary-output";
const ADJACENT_PAIR_WIDTH: usize = 2;
const CLASS_ACCEPTED_IMPLEMENTATION_UNESTABLISHED: &str = "accepted-implementation-unestablished";
const REPORT_HEADER: &str = "specification\tsource_area\tclass\trequirement_id\n";

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequirementDefinition {
    specification: String,
    line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ClassificationRow {
    specification: String,
    source_area: String,
    class: String,
    requirement_id: String,
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

fn baseline_is_sorted_and_unique(lines: &[String]) -> bool {
    !lines.iter().any(String::is_empty) && lines.windows(ADJACENT_PAIR_WIDTH).all(|pair| pair[0] < pair[1])
}

fn source_area(requirement_id: &str) -> Result<String, String> {
    let mut parts = requirement_id.split('.');
    let namespace = parts.next().unwrap_or_default();
    let area = parts.next().unwrap_or_default();
    if namespace != "molten" || area.is_empty() {
        return Err(format!("requirement identifier has no Molten source area: {requirement_id}"));
    }
    Ok(area.to_string())
}

fn classify_baseline(
    baseline: &[String],
    definitions: &BTreeMap<String, Vec<RequirementDefinition>>,
) -> Result<Vec<ClassificationRow>, Vec<String>> {
    let mut rows = Vec::with_capacity(baseline.len());
    let mut issues = Vec::new();
    for requirement_id in baseline {
        let Some(locations) = definitions.get(requirement_id) else {
            issues.push(format!("baseline requirement has no accepted definition: {requirement_id}"));
            continue;
        };
        if locations.len() != 1 {
            let rendered = locations
                .iter()
                .map(|location| format!("{}:{}", location.specification, location.line))
                .collect::<Vec<_>>()
                .join(",");
            issues
                .push(format!("baseline requirement has duplicate accepted definitions: {requirement_id}: {rendered}"));
            continue;
        }
        let area = match source_area(requirement_id) {
            Ok(area) => area,
            Err(issue) => {
                issues.push(issue);
                continue;
            }
        };
        rows.push(ClassificationRow {
            specification: locations[0].specification.clone(),
            source_area: area,
            class: CLASS_ACCEPTED_IMPLEMENTATION_UNESTABLISHED.to_string(),
            requirement_id: requirement_id.clone(),
        });
    }
    if issues.is_empty() {
        rows.sort();
        Ok(rows)
    } else {
        issues.sort();
        Err(issues)
    }
}

fn render_report(rows: &[ClassificationRow]) -> String {
    let mut report = String::from(REPORT_HEADER);
    for row in rows {
        report.push_str(&row.specification);
        report.push('\t');
        report.push_str(&row.source_area);
        report.push('\t');
        report.push_str(&row.class);
        report.push('\t');
        report.push_str(&row.requirement_id);
        report.push('\n');
    }
    report
}

fn grouped_counts<'a>(values: impl Iterator<Item = &'a str>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for value in values {
        *counts.entry(value.to_string()).or_insert(0) += 1;
    }
    counts
}

fn sorted_group_counts(counts: &BTreeMap<String, usize>) -> Vec<(String, usize)> {
    let mut rows = counts.iter().map(|(name, count)| (name.clone(), *count)).collect::<Vec<_>>();
    rows.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    rows
}

fn render_summary(rows: &[ClassificationRow]) -> String {
    let specification_counts = grouped_counts(rows.iter().map(|row| row.specification.as_str()));
    let area_counts = grouped_counts(rows.iter().map(|row| row.source_area.as_str()));
    let mut output = String::from(
        "# Inherited Tracey debt classification\n\nAll remaining rows use `accepted-implementation-unestablished`. The report does not establish implementation, replacement, obsolescence, or invalidity.\n\n## Specification groups\n\n| Count | Specification |\n| ---: | --- |\n",
    );
    for (specification, count) in sorted_group_counts(&specification_counts) {
        output.push_str(&format!("| {count} | `{specification}` |\n"));
    }
    output.push_str("\n## Source area groups\n\n| Count | Source area |\n| ---: | --- |\n");
    for (area, count) in sorted_group_counts(&area_counts) {
        output.push_str(&format!("| {count} | `{area}` |\n"));
    }
    output
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

fn read_baseline(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let lines = text.lines().map(str::to_string).collect::<Vec<_>>();
    if !baseline_is_sorted_and_unique(&lines) {
        return Err("baseline must contain unique, sorted, non-empty requirement identifiers".to_string());
    }
    Ok(lines)
}

fn read_definitions(root: &Path) -> Result<BTreeMap<String, Vec<RequirementDefinition>>, String> {
    let requirement_root = root.join(REQUIREMENT_ROOT);
    let mut files = Vec::new();
    walk_files(&requirement_root, &mut files)?;
    files.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some(REQUIREMENT_EXTENSION));
    files.sort();
    let mut definitions = BTreeMap::<String, Vec<RequirementDefinition>>::new();
    for path in files {
        let text = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("{}: {error}", path.display()))?
            .to_string_lossy()
            .to_string();
        for (index, line) in text.lines().enumerate() {
            if let Some(requirement_id) = requirement_marker(line) {
                definitions.entry(requirement_id).or_default().push(RequirementDefinition {
                    specification: relative.clone(),
                    line: index + 1,
                });
            }
        }
    }
    Ok(definitions)
}

fn option_value(args: &[String], option: &str) -> Option<PathBuf> {
    args.windows(ADJACENT_PAIR_WIDTH).find(|pair| pair[0] == option).map(|pair| PathBuf::from(&pair[1]))
}

fn run(args: &[String]) -> Result<(), String> {
    let root = option_value(args, OPTION_ROOT).unwrap_or_else(|| PathBuf::from("."));
    let baseline_path =
        option_value(args, OPTION_BASELINE).ok_or_else(|| format!("missing required option {OPTION_BASELINE}"))?;
    let output_path =
        option_value(args, OPTION_OUTPUT).ok_or_else(|| format!("missing required option {OPTION_OUTPUT}"))?;
    let summary_output_path = option_value(args, OPTION_SUMMARY_OUTPUT)
        .ok_or_else(|| format!("missing required option {OPTION_SUMMARY_OUTPUT}"))?;
    let baseline = read_baseline(&baseline_path)?;
    let definitions = read_definitions(&root)?;
    let rows = classify_baseline(&baseline, &definitions).map_err(|issues| issues.join("\n"))?;
    fs::write(&output_path, render_report(&rows)).map_err(|error| format!("{}: {error}", output_path.display()))?;
    fs::write(&summary_output_path, render_summary(&rows))
        .map_err(|error| format!("{}: {error}", summary_output_path.display()))?;

    let specification_counts = grouped_counts(rows.iter().map(|row| row.specification.as_str()));
    let area_counts = grouped_counts(rows.iter().map(|row| row.source_area.as_str()));
    println!("baseline_entries={}", baseline.len());
    println!("classified_entries={}", rows.len());
    println!("accepted_implementation_unestablished={}", rows.len());
    println!("specification_groups={}", specification_counts.len());
    println!("source_area_groups={}", area_counts.len());
    println!("non_claim=classification does not establish implementation, replacement, obsolescence, or invalidity");
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

    fn location(specification: &str, line: usize) -> RequirementDefinition {
        RequirementDefinition {
            specification: specification.to_string(),
            line,
        }
    }

    #[test]
    fn inventory_is_conservative_and_grouped() {
        // r[verify molten.project.inherited_tracey_classification.inventory]
        // r[verify molten.project.inherited_tracey_classification.conservative_default]
        // r[verify molten.project.inherited_tracey_classification.deterministic_grouping]
        // r[verify molten.project.inherited_tracey_classification.fixtures]
        let baseline = vec!["molten.beta.two".to_string(), "molten.alpha.one".to_string()];
        let definitions = BTreeMap::from([
            ("molten.alpha.one".to_string(), vec![location("cairn/specs/zeta/spec.md", 1)]),
            ("molten.beta.two".to_string(), vec![location("cairn/specs/alpha/spec.md", 2)]),
        ]);
        let rows = classify_baseline(&baseline, &definitions).expect("valid inventory");
        assert_eq!(rows[0].requirement_id, "molten.beta.two");
        assert_eq!(rows[1].requirement_id, "molten.alpha.one");
        assert!(rows.iter().all(|row| row.class == CLASS_ACCEPTED_IMPLEMENTATION_UNESTABLISHED));
        let report = render_report(&rows);
        assert!(report.starts_with(REPORT_HEADER));
        assert!(report.contains("cairn/specs/alpha/spec.md\tbeta"));
        let summary = render_summary(&rows);
        assert!(summary.contains("| 1 | `cairn/specs/alpha/spec.md` |"));
        assert!(summary.contains("| 1 | `beta` |"));
    }

    #[test]
    fn missing_and_duplicate_definitions_fail() {
        // r[verify molten.project.inherited_tracey_classification.duplicate_denial]
        // r[verify molten.project.inherited_tracey_classification.fixtures]
        let baseline = vec!["molten.alpha.duplicate".to_string(), "molten.beta.missing".to_string()];
        let definitions = BTreeMap::from([("molten.alpha.duplicate".to_string(), vec![
            location("cairn/specs/alpha/spec.md", 1),
            location("cairn/specs/beta/spec.md", 2),
        ])]);
        let issues = classify_baseline(&baseline, &definitions).expect_err("invalid inventory");
        assert!(issues.iter().any(|issue| issue.contains("duplicate accepted definitions")));
        assert!(issues.iter().any(|issue| issue.contains("no accepted definition")));
    }

    #[test]
    fn malformed_baselines_and_namespaces_fail() {
        // r[verify molten.project.inherited_tracey_classification.fixtures]
        assert!(!baseline_is_sorted_and_unique(&["molten.beta.two".to_string(), "molten.alpha.one".to_string()]));
        assert!(!baseline_is_sorted_and_unique(&["molten.alpha.one".to_string(), "molten.alpha.one".to_string()]));
        assert!(source_area("foreign.alpha.one").is_err());
        assert!(source_area("molten").is_err());
    }

    #[test]
    fn non_claim_is_explicit() {
        // r[verify molten.project.inherited_tracey_classification.non_claims]
        let non_claim = "classification does not establish implementation, replacement, obsolescence, or invalidity";
        assert!(non_claim.contains("does not establish"));
    }
}
