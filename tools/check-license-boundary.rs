#!/usr/bin/env -S CARGO_TARGET_DIR=/tmp/onix-license-boundary-script-target nix shell "github:nix-community/fenix?rev=092bd452904e749efa39907aa4a20a42678ac31e#minimal.toolchain" nixpkgs#gcc nixpkgs#clang -c cargo -q -Zscript

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const POLICY_FILE: &str = "license-policy.tsv";
const POLICY_FIELD_COUNT: usize = 3;
const PACKAGE_RULE: &str = "package";
const ARTIFACT_RULE: &str = "artifact";
const CONTAINS_RULE: &str = "contains";
const FORBID_RULE: &str = "forbid";
const FORBID_ANY_RULE: &str = "forbid-any";
const WORKSPACE_PACKAGE_SET_RULE: &str = "workspace-package-set";
const PACKAGE_SECTION: &str = "[package]";
const WORKSPACE_PACKAGE_SECTION: &str = "[workspace.package]";
const LICENSE_KEY: &str = "license";

#[derive(Clone, Debug, Eq, PartialEq)]
enum RuleKind {
    Package,
    Artifact,
    Contains,
    Forbid,
    ForbidAny,
    WorkspacePackageSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Rule {
    kind: RuleKind,
    path: PathBuf,
    value: String,
}

fn parse_rule_kind(value: &str) -> Result<RuleKind, String> {
    match value {
        PACKAGE_RULE => Ok(RuleKind::Package),
        ARTIFACT_RULE => Ok(RuleKind::Artifact),
        CONTAINS_RULE => Ok(RuleKind::Contains),
        FORBID_RULE => Ok(RuleKind::Forbid),
        FORBID_ANY_RULE => Ok(RuleKind::ForbidAny),
        WORKSPACE_PACKAGE_SET_RULE => Ok(RuleKind::WorkspacePackageSet),
        _ => Err(format!("unknown rule kind: {value}")),
    }
}

fn parse_policy(contents: &str) -> Result<Vec<Rule>, String> {
    let mut rules = Vec::new();
    for (line_index, raw_line) in contents.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != POLICY_FIELD_COUNT {
            let display_line = line_index + 1;
            return Err(format!(
                "policy line {display_line} must contain {POLICY_FIELD_COUNT} tab-separated fields"
            ));
        }
        let kind = parse_rule_kind(fields[0])?;
        let path = PathBuf::from(fields[1]);
        let value = fields[2].to_owned();
        if path.as_os_str().is_empty() || value.is_empty() {
            let display_line = line_index + 1;
            return Err(format!("policy line {display_line} contains an empty field"));
        }
        rules.push(Rule { kind, path, value });
    }
    if rules.is_empty() {
        return Err("license policy contains no rules".to_owned());
    }
    Ok(rules)
}

fn quoted_value(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    let prefix = format!("{key} = \"");
    if !trimmed.starts_with(&prefix) || !trimmed.ends_with('"') {
        return None;
    }
    let value = &trimmed[prefix.len()..trimmed.len() - 1];
    Some(value.to_owned())
}

fn section_assignment(contents: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if in_section {
            if let Some(value) = quoted_value(line, key) {
                return Some(value);
            }
        }
    }
    None
}

fn package_uses_workspace_license(contents: &str) -> bool {
    let mut in_package = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            in_package = line == PACKAGE_SECTION;
            continue;
        }
        if in_package && line == "license.workspace = true" {
            return true;
        }
    }
    false
}

fn declared_package_license(manifest: &str, workspace_manifest: &str) -> Result<String, String> {
    if let Some(value) = section_assignment(manifest, PACKAGE_SECTION, LICENSE_KEY) {
        return Ok(value);
    }
    if package_uses_workspace_license(manifest) {
        if let Some(value) =
            section_assignment(workspace_manifest, WORKSPACE_PACKAGE_SECTION, LICENSE_KEY)
        {
            return Ok(value);
        }
        return Err("package inherits a missing workspace license".to_owned());
    }
    Err("package has no explicit or inherited license".to_owned())
}

fn validate_package_license(
    manifest: &str,
    workspace_manifest: &str,
    expected: &str,
) -> Result<(), String> {
    let actual = declared_package_license(manifest, workspace_manifest)?;
    if actual != expected {
        return Err(format!("expected {expected}, found {actual}"));
    }
    Ok(())
}

fn validate_artifact(contents: Option<&str>, marker: &str) -> Result<(), String> {
    let Some(contents) = contents else {
        return Err("required license artifact is missing".to_owned());
    };
    if !contents.contains(marker) {
        return Err(format!("license artifact does not contain marker: {marker}"));
    }
    Ok(())
}

fn validate_contains(contents: &str, marker: &str) -> Result<(), String> {
    if !contents.contains(marker) {
        return Err(format!("required marker is missing: {marker}"));
    }
    Ok(())
}

fn validate_forbid(contents: &str, marker: &str) -> Result<(), String> {
    if contents.contains(marker) {
        return Err(format!("forbidden dependency or marker is present: {marker}"));
    }
    Ok(())
}

fn validate_forbid_any(contents: &str, markers: &str) -> Result<(), String> {
    for marker in markers.split(',') {
        let marker = marker.trim();
        if marker.is_empty() {
            return Err("forbid-any contains an empty marker".to_owned());
        }
        validate_forbid(contents, marker)?;
    }
    Ok(())
}

fn run_self_tests() {
    let policy = "package\tCargo.toml\tMPL-2.0\nartifact\tLICENSE\tMozilla Public License";
    let rules = parse_policy(policy).expect("positive policy fixture must parse");
    assert_eq!(rules.len(), 2);
    assert!(parse_policy("package\tCargo.toml").is_err());
    assert!(parse_policy("unknown\tCargo.toml\tMPL-2.0").is_err());

    let direct_manifest = "[package]\nname = \"core\"\nlicense = \"MPL-2.0\"\n";
    assert!(validate_package_license(direct_manifest, "", "MPL-2.0").is_ok());
    assert!(validate_package_license(direct_manifest, "", "AGPL-3.0-or-later").is_err());

    let inherited_manifest = "[package]\nname = \"shell\"\nlicense.workspace = true\n";
    let workspace_manifest = "[workspace.package]\nlicense = \"AGPL-3.0-or-later\"\n";
    assert!(
        validate_package_license(inherited_manifest, workspace_manifest, "AGPL-3.0-or-later")
            .is_ok()
    );
    assert!(validate_package_license(inherited_manifest, "", "AGPL-3.0-or-later").is_err());

    assert!(validate_artifact(Some("Mozilla Public License Version 2.0"), "Mozilla").is_ok());
    assert!(validate_artifact(None, "Mozilla").is_err());
    assert!(validate_contains("package = MPL-2.0", "MPL-2.0").is_ok());
    assert!(validate_contains("package = MPL-2.0", "AGPL-3.0-or-later").is_err());
    assert!(validate_forbid("dependencies = []", "host-controller").is_ok());
    assert!(validate_forbid("host-controller = true", "host-controller").is_err());
    assert!(validate_forbid_any("dependencies = []", "host-a,host-b").is_ok());
    assert!(validate_forbid_any("host-b = true", "host-a,host-b").is_err());
    assert!(validate_forbid_any("dependencies = []", "host-a,").is_err());

    let workspace = "[workspace]\nmembers = [\"crates/core\", \"crates/shell\"]\n";
    let complete_rules = vec![
        Rule {
            kind: RuleKind::Package,
            path: PathBuf::from("crates/core/Cargo.toml"),
            value: "MPL-2.0".to_owned(),
        },
        Rule {
            kind: RuleKind::Package,
            path: PathBuf::from("crates/shell/Cargo.toml"),
            value: "AGPL-3.0-or-later".to_owned(),
        },
    ];
    assert!(validate_workspace_coverage(workspace, &complete_rules).is_ok());
    assert!(validate_workspace_coverage(workspace, &complete_rules[..1]).is_err());

    let mut strict_rules = complete_rules.clone();
    strict_rules.push(Rule {
        kind: RuleKind::WorkspacePackageSet,
        path: PathBuf::from("Cargo.toml"),
        value: "strict".to_owned(),
    });
    assert!(validate_workspace_coverage(workspace, &strict_rules).is_ok());
    strict_rules.push(Rule {
        kind: RuleKind::Package,
        path: PathBuf::from("crates/unknown/Cargo.toml"),
        value: "MPL-2.0".to_owned(),
    });
    assert!(validate_workspace_coverage(workspace, &strict_rules).is_err());
}

fn read_required(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn check_rule(root: &Path, workspace_manifest: &str, rule: &Rule) -> Result<(), String> {
    let path = root.join(&rule.path);
    match rule.kind {
        RuleKind::Package => {
            let manifest = read_required(&path)?;
            validate_package_license(&manifest, workspace_manifest, &rule.value)
                .map_err(|error| format!("{}: {error}", rule.path.display()))
        }
        RuleKind::Artifact => {
            let contents = fs::read_to_string(&path).ok();
            validate_artifact(contents.as_deref(), &rule.value)
                .map_err(|error| format!("{}: {error}", rule.path.display()))
        }
        RuleKind::Contains => {
            let contents = read_required(&path)?;
            validate_contains(&contents, &rule.value)
                .map_err(|error| format!("{}: {error}", rule.path.display()))
        }
        RuleKind::Forbid => {
            let contents = read_required(&path)?;
            validate_forbid(&contents, &rule.value)
                .map_err(|error| format!("{}: {error}", rule.path.display()))
        }
        RuleKind::ForbidAny => {
            let contents = read_required(&path)?;
            validate_forbid_any(&contents, &rule.value)
                .map_err(|error| format!("{}: {error}", rule.path.display()))
        }
        RuleKind::WorkspacePackageSet => Ok(()),
    }
}

fn repository_root() -> Result<PathBuf, String> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    if let Some(root) = arguments.next() {
        if arguments.next().is_some() {
            return Err("usage: check-license-boundary.rs [repository-root]".to_owned());
        }
        return Ok(PathBuf::from(root));
    }
    env::current_dir().map_err(|error| format!("cannot read current directory: {error}"))
}

fn quoted_strings(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut remainder = line;
    loop {
        let Some(start) = remainder.find('"') else {
            break;
        };
        let after_start = &remainder[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        values.push(after_start[..end].to_owned());
        remainder = &after_start[end + 1..];
    }
    values
}

fn workspace_member_manifests(contents: &str) -> Vec<PathBuf> {
    let mut manifests = Vec::new();
    let mut in_workspace = false;
    let mut in_members = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && !in_members {
            in_workspace = line == "[workspace]";
            continue;
        }
        if in_workspace && line.starts_with("members = [") {
            in_members = true;
        }
        if in_members {
            for member in quoted_strings(line) {
                let manifest = if member == "." {
                    PathBuf::from("Cargo.toml")
                } else {
                    PathBuf::from(member).join("Cargo.toml")
                };
                manifests.push(manifest);
            }
            if line.contains(']') {
                in_members = false;
            }
        }
    }
    if contents.lines().any(|line| line.trim() == PACKAGE_SECTION)
        && !manifests.iter().any(|path| path == Path::new("Cargo.toml"))
    {
        manifests.push(PathBuf::from("Cargo.toml"));
    }
    manifests
}

fn validate_workspace_coverage(workspace_manifest: &str, rules: &[Rule]) -> Result<(), String> {
    let manifests = workspace_member_manifests(workspace_manifest);
    for manifest in &manifests {
        let count = rules
            .iter()
            .filter(|rule| rule.kind == RuleKind::Package && rule.path == *manifest)
            .count();
        if count != 1 {
            return Err(format!(
                "workspace package {} must have exactly one package rule; found {count}",
                manifest.display()
            ));
        }
    }

    let strict = rules.iter().any(|rule| rule.kind == RuleKind::WorkspacePackageSet);
    if strict {
        for rule in rules.iter().filter(|rule| rule.kind == RuleKind::Package) {
            if !manifests.contains(&rule.path) {
                return Err(format!(
                    "unknown package rule outside the workspace set: {}",
                    rule.path.display()
                ));
            }
        }
    }
    Ok(())
}

fn run() -> Result<(), String> {
    run_self_tests();
    let root = repository_root()?;
    let policy_path = root.join(POLICY_FILE);
    let policy = read_required(&policy_path)?;
    let rules = parse_policy(&policy)?;
    let workspace_manifest = fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    validate_workspace_coverage(&workspace_manifest, &rules)?;

    let mut failures = Vec::new();
    for rule in &rules {
        if let Err(error) = check_rule(&root, &workspace_manifest, rule) {
            failures.push(error);
        }
    }
    if !failures.is_empty() {
        return Err(failures.join("\n"));
    }
    println!("license boundary valid: {} rules", rules.len());
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("license boundary invalid:\n{error}");
        std::process::exit(1);
    }
}
