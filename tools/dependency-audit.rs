#!/usr/bin/env -S CARGO_TARGET_DIR=/tmp/molten-dependency-audit-target nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#gcc -c cargo -q -Zscript
---
[dependencies]
blake3 = "=1.8.5"
serde = { version = "=1.0.228", features = ["derive"] }
serde_json = "=1.0.149"
---
//! Resolved-closure source, license, and advisory audit evidence.
//!
//! This tool is a read-only report over `cargo metadata` plus a reviewed policy
//! and advisory snapshot. It does not replace the release dependency profile,
//! owned-package license checker, legal review, or a runtime security claim.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

const POLICY_SCHEMA: &str = "molten.dependency-audit-policy.v1";
const ADVISORY_SCHEMA: &str = "molten.advisory-snapshot.v1";
const REPORT_SCHEMA: &str = "molten.dependency-audit-report.v1";
const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    resolve: Option<Resolve>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    version: String,
    id: String,
    source: Option<String>,
    license: Option<String>,
    license_file: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
struct Node {
    id: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    deps: Vec<ResolvedDependency>,
}

#[derive(Debug, Deserialize)]
struct ResolvedDependency {
    pkg: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Policy {
    schema: String,
    allowed_sources: Vec<String>,
    allowed_licenses: Vec<String>,
    #[serde(default)]
    allowed_exceptions: Vec<String>,
    workspace_license: String,
    #[serde(default)]
    exceptions: Vec<PolicyException>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PolicyException {
    package: String,
    source: String,
    advisory: Option<String>,
    owner: String,
    rationale: String,
    expires: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Advisories {
    schema: String,
    revision: String,
    generated_at: String,
    #[serde(default)]
    advisories: Vec<Advisory>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Advisory {
    id: String,
    package: String,
    versions: Vec<String>,
    severity: String,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    decision: String,
    as_of: String,
    metadata_ref: String,
    policy_ref: String,
    advisory_snapshot: Option<Snapshot>,
    summary: Summary,
    findings: Vec<Finding>,
    non_claim: &'static str,
}

#[derive(Debug, Serialize)]
struct Snapshot {
    revision: String,
    generated_at: String,
    ref_value: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    packages: usize,
    workspace_packages: usize,
    unrecognized_sources: usize,
    missing_licenses: usize,
    advisory_matches: usize,
    actionable_advisories: usize,
    excepted_advisories: usize,
}

#[derive(Debug, Serialize)]
struct Finding {
    kind: String,
    package: String,
    version: String,
    source: String,
    dependency_path: Vec<String>,
    detail: String,
    disposition: String,
    evidence_ref: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("dependency audit denied: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut metadata = None;
    let mut policy = None;
    let mut advisories = None;
    let mut as_of = None;
    let mut json_out = None;
    let mut markdown_out = None;
    let mut self_test = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--metadata" => metadata = Some(path_arg(&mut args, "--metadata")?),
            "--policy" => policy = Some(path_arg(&mut args, "--policy")?),
            "--advisories" => advisories = Some(path_arg(&mut args, "--advisories")?),
            "--as-of" => as_of = Some(next_arg(&mut args, "--as-of")?),
            "--json-out" => json_out = Some(path_arg(&mut args, "--json-out")?),
            "--markdown-out" => markdown_out = Some(path_arg(&mut args, "--markdown-out")?),
            "--self-test" => self_test = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if self_test {
        return self_tests();
    }
    let metadata_path = metadata.ok_or("usage: dependency-audit --metadata FILE --policy FILE [--advisories FILE] --as-of YYYY-MM-DD")?;
    let policy_path = policy.ok_or("missing --policy")?;
    let as_of = as_of.ok_or("missing --as-of YYYY-MM-DD")?;
    let metadata_bytes = fs::read(&metadata_path).map_err(|e| format!("{}: {e}", metadata_path.display()))?;
    let policy_bytes = fs::read(&policy_path).map_err(|e| format!("{}: {e}", policy_path.display()))?;
    let metadata: Metadata = serde_json::from_slice(&metadata_bytes).map_err(|e| format!("{}: {e}", metadata_path.display()))?;
    let policy: Policy = serde_json::from_slice(&policy_bytes).map_err(|e| format!("{}: {e}", policy_path.display()))?;
    let advisory_bytes = advisories
        .as_ref()
        .map(|path| fs::read(path).map_err(|e| format!("{}: {e}", path.display())))
        .transpose()?;
    let advisories: Option<Advisories> = advisory_bytes
        .as_ref()
        .map(|bytes| serde_json::from_slice(bytes).map_err(|e| format!("advisories: {e}")))
        .transpose()?;
    let report = audit(
        &metadata,
        &policy,
        advisories.as_ref(),
        advisory_bytes.as_deref(),
        &as_of,
        &ref_of(&metadata_bytes),
        &ref_of(&policy_bytes),
    )?;
    let json = serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n";
    let markdown = render_markdown(&report);
    let wrote_json = json_out.is_some();
    let wrote_markdown = markdown_out.is_some();
    if let Some(path) = &json_out {
        fs::write(path, json).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if let Some(path) = &markdown_out {
        fs::write(path, &markdown).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    if !wrote_json && !wrote_markdown {
        print!("{markdown}");
    }
    if report.decision == "deny" {
        return Err("audit findings require review".to_string());
    }
    Ok(())
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("{flag} requires a value"))
}

fn path_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    next_arg(args, flag).map(PathBuf::from)
}

fn ref_of(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

fn audit(
    metadata: &Metadata,
    policy: &Policy,
    advisories: Option<&Advisories>,
    advisory_bytes: Option<&[u8]>,
    as_of: &str,
    metadata_ref: &str,
    policy_ref: &str,
) -> Result<Report, String> {
    if policy.schema != POLICY_SCHEMA {
        return Err(format!("policy schema must be {POLICY_SCHEMA}"));
    }
    if policy.workspace_license.trim().is_empty() {
        return Err("policy workspace_license must not be empty".to_string());
    }
    if let Some(advisories) = advisories {
        if advisories.schema != ADVISORY_SCHEMA {
            return Err(format!("advisory schema must be {ADVISORY_SCHEMA}"));
        }
    }
    let package_by_id: BTreeMap<&str, &Package> = metadata.packages.iter().map(|package| (package.id.as_str(), package)).collect();
    let workspace: BTreeSet<&str> = metadata.workspace_members.iter().map(String::as_str).collect();
    let paths = dependency_paths(metadata, &workspace);
    let allowed_licenses: BTreeSet<&str> = policy.allowed_licenses.iter().map(String::as_str).collect();
    let allowed_exceptions: BTreeSet<&str> = policy.allowed_exceptions.iter().map(String::as_str).collect();
    let mut findings = Vec::new();
    let mut unrecognized_sources = 0;
    let mut missing_licenses = 0;
    let mut advisory_matches = 0;
    let mut actionable = 0;
    let mut excepted = 0;
    for package in &metadata.packages {
        let is_workspace = workspace.contains(package.id.as_str());
        let source = package.source.clone().unwrap_or_else(|| "workspace".to_string());
        if is_workspace {
            if package.license.as_deref() != Some(policy.workspace_license.as_str()) {
                findings.push(finding(
                    "workspace-license",
                    package,
                    &source,
                    &paths,
                    format!("expected workspace license {}, found {}", policy.workspace_license, package.license.clone().unwrap_or_else(|| "none".to_string())),
                    "review",
                    None,
                ));
            }
            continue;
        }
        if !source_allowed(&source, &policy.allowed_sources) {
            unrecognized_sources += 1;
            findings.push(finding(
                "unrecognized-source",
                package,
                &source,
                &paths,
                "source is not in the reviewed policy allowlist".to_string(),
                "actionable",
                None,
            ));
        }
        let license_available = package
            .license
            .as_deref()
            .is_some_and(|license| license_allowed(license, &allowed_licenses, &allowed_exceptions));
        if !license_available {
            missing_licenses += 1;
            findings.push(finding(
                "license-metadata",
                package,
                &source,
                &paths,
                format!(
                    "license expression {} is not in the reviewed allowlist{}",
                    package.license.clone().unwrap_or_else(|| "missing".to_string()),
                    if package.license_file.is_some() { "; a license file is present but not a reviewed expression" } else { "" }
                ),
                "unavailable",
                None,
            ));
        }
        if let Some(advisories) = advisories {
            for advisory in &advisories.advisories {
                if advisory.package != package.name || !version_matches(&advisory.versions, &package.version) {
                    continue;
                }
                advisory_matches += 1;
                let exception = policy.exceptions.iter().find(|exception| {
                    exception.package == package.name
                        && exception.source == source
                        && exception.advisory.as_deref().is_none_or(|id| id == advisory.id)
                        && exception.expires.as_str() >= as_of
                });
                let (disposition, detail, evidence_ref) = match exception {
                    Some(exception) => {
                        excepted += 1;
                        (
                            "excepted",
                            format!(
                                "advisory {} severity {} excepted by {} until {}: {}",
                                advisory.id, advisory.severity, exception.owner, exception.expires, exception.rationale
                            ),
                            None,
                        )
                    }
                    None => {
                        actionable += 1;
                        (
                            "actionable",
                            format!("advisory {} severity {} has no current reviewed exception", advisory.id, advisory.severity),
                            Some(format!("advisory:{}", advisory.id)),
                        )
                    }
                };
                findings.push(finding(
                    "advisory",
                    package,
                    &source,
                    &paths,
                    detail,
                    disposition,
                    evidence_ref,
                ));
            }
        }
    }
    // An expired exception is a stale review input and must fail closed.
    for exception in &policy.exceptions {
        if exception.expires.as_str() < as_of {
            findings.push(Finding {
                kind: "expired-exception".to_string(),
                package: exception.package.clone(),
                version: "*".to_string(),
                source: exception.source.clone(),
                dependency_path: vec![],
                detail: format!("exception owned by {} expired at {}", exception.owner, exception.expires),
                disposition: "actionable".to_string(),
                evidence_ref: None,
            });
            actionable += 1;
        }
    }
    let snapshot = advisories.map(|advisories| Snapshot {
        revision: advisories.revision.clone(),
        generated_at: advisories.generated_at.clone(),
        ref_value: advisory_bytes.map(ref_of).unwrap_or_default(),
    });
    let decision = if advisories.is_none() {
        "deny"
    } else if actionable > 0 || unrecognized_sources > 0 || missing_licenses > 0 {
        "deny"
    } else {
        "pass"
    };
    if advisories.is_none() {
        findings.push(Finding {
            kind: "advisory-snapshot".to_string(),
            package: "*".to_string(),
            version: "*".to_string(),
            source: "*".to_string(),
            dependency_path: vec![],
            detail: "no advisory snapshot supplied; advisories are unavailable, not clean".to_string(),
            disposition: "unavailable".to_string(),
            evidence_ref: None,
        });
    }
    findings.sort_by(|left, right| {
        (&left.package, &left.version, &left.kind, &left.detail).cmp(&(&right.package, &right.version, &right.kind, &right.detail))
    });
    Ok(Report {
        schema: REPORT_SCHEMA,
        decision: decision.to_string(),
        as_of: as_of.to_string(),
        metadata_ref: metadata_ref.to_string(),
        policy_ref: policy_ref.to_string(),
        advisory_snapshot: snapshot,
        summary: Summary {
            packages: package_by_id.len(),
            workspace_packages: workspace.len(),
            unrecognized_sources,
            missing_licenses,
            advisory_matches,
            actionable_advisories: actionable,
            excepted_advisories: excepted,
        },
        findings,
        non_claim: "release review evidence only; not legal advice, a vulnerability-free claim, or runtime security qualification",
    })
}

fn source_allowed(source: &str, allowed: &[String]) -> bool {
    source == CRATES_IO || allowed.iter().any(|prefix| source == prefix || source.starts_with(prefix))
}

fn version_matches(versions: &[String], version: &str) -> bool {
    versions.iter().any(|candidate| candidate == "*" || candidate == version)
}

fn license_allowed(expression: &str, allowed: &BTreeSet<&str>, exceptions: &BTreeSet<&str>) -> bool {
    let normalized = expression
        .replace('/', " OR ")
        .replace(['(', ')'], "")
        .replace("MPL-2.0+", "MPL-2.0");
    let mut saw_term = false;
    for conjunction in normalized.split(" OR ") {
        for raw_term in conjunction.split(" AND ") {
            let term = raw_term.trim();
            if term.is_empty() {
                continue;
            }
            saw_term = true;
            if let Some((base, exception)) = term.split_once(" WITH ") {
                if !allowed.contains(base.trim()) || !exceptions.contains(exception.trim()) {
                    return false;
                }
            } else if !allowed.contains(term) {
                return false;
            }
        }
    }
    saw_term
}

fn dependency_paths(metadata: &Metadata, workspace: &BTreeSet<&str>) -> BTreeMap<String, Vec<String>> {
    let Some(resolve) = &metadata.resolve else {
        return BTreeMap::new();
    };
    let mut adjacency: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for node in &resolve.nodes {
        let mut targets = Vec::new();
        for dependency in &node.deps {
            targets.push(dependency.pkg.as_str());
        }
        for dependency in &node.dependencies {
            targets.push(dependency.as_str());
        }
        adjacency.entry(node.id.as_str()).or_default().extend(targets);
    }
    let mut paths = BTreeMap::new();
    for member in workspace {
        let mut queue = VecDeque::new();
        queue.push_back((*member, vec![(*member).to_string()]));
        let mut seen = BTreeSet::new();
        while let Some((current, path)) = queue.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            paths.entry(current.to_string()).or_insert_with(|| path.clone());
            for target in adjacency.get(current).into_iter().flatten() {
                let mut next = path.clone();
                next.push((*target).to_string());
                queue.push_back((target, next));
            }
        }
    }
    paths
}

fn finding(
    kind: &str,
    package: &Package,
    source: &str,
    paths: &BTreeMap<String, Vec<String>>,
    detail: String,
    disposition: &str,
    evidence_ref: Option<String>,
) -> Finding {
    Finding {
        kind: kind.to_string(),
        package: package.name.clone(),
        version: package.version.clone(),
        source: source.to_string(),
        dependency_path: paths.get(package.id.as_str()).cloned().unwrap_or_default(),
        detail,
        disposition: disposition.to_string(),
        evidence_ref,
    }
}

fn render_markdown(report: &Report) -> String {
    let mut lines = vec![
        "# Dependency audit report".to_string(),
        String::new(),
        format!("Decision: **{}**", report.decision),
        format!("As of: `{}`", report.as_of),
        format!("Metadata: `{}`", report.metadata_ref),
        format!("Policy: `{}`", report.policy_ref),
    ];
    match &report.advisory_snapshot {
        Some(snapshot) => lines.push(format!(
            "Advisory snapshot: `{}` at `{}` (`{}`)",
            snapshot.revision, snapshot.generated_at, snapshot.ref_value
        )),
        None => lines.push("Advisory snapshot: unavailable".to_string()),
    }
    lines.extend([
        String::new(),
        format!(
            "Packages: {} total, {} workspace. Unrecognized sources: {}. Missing licenses: {}. Advisory matches: {} ({} actionable, {} excepted).",
            report.summary.packages,
            report.summary.workspace_packages,
            report.summary.unrecognized_sources,
            report.summary.missing_licenses,
            report.summary.advisory_matches,
            report.summary.actionable_advisories,
            report.summary.excepted_advisories
        ),
        String::new(),
        "| Kind | Package | Version | Source | Disposition | Detail | Path |".to_string(),
        "|---|---|---|---|---|---|---|".to_string(),
    ]);
    for finding in &report.findings {
        lines.push(format!(
            "| {} | `{}` | `{}` | `{}` | {} | {} | {} |",
            finding.kind,
            finding.package,
            finding.version,
            finding.source,
            finding.disposition,
            finding.detail,
            if finding.dependency_path.is_empty() { "direct".to_string() } else { finding.dependency_path.join(" → ") }
        ));
    }
    lines.push(String::new());
    lines.push(format!("_{}_", report.non_claim));
    lines.push(String::new());
    lines.join("\n")
}

fn self_tests() -> Result<(), String> {
    let policy = Policy {
        schema: POLICY_SCHEMA.to_string(),
        allowed_sources: vec![CRATES_IO.to_string(), "git+https://github.com/OnixResearch/".to_string()],
        allowed_licenses: vec!["MIT".to_string(), "Apache-2.0".to_string()],
        allowed_exceptions: vec!["LLVM-exception".to_string()],
        workspace_license: "AGPL-3.0-or-later".to_string(),
        exceptions: vec![],
    };
    let allowed: BTreeSet<&str> = ["MIT", "Apache-2.0"].into_iter().collect();
    let exceptions: BTreeSet<&str> = ["LLVM-exception"].into_iter().collect();
    if !license_allowed("MIT OR Apache-2.0", &allowed, &exceptions)
        || !license_allowed("Apache-2.0 WITH LLVM-exception", &allowed, &exceptions)
        || license_allowed("GPL-3.0-only", &allowed, &exceptions)
    {
        return Err("license expression fixture mismatch".to_string());
    }
    let metadata = Metadata {
        packages: vec![
            Package { name: "root".to_string(), version: "0.1.0".to_string(), id: "root".to_string(), source: None, license: Some("AGPL-3.0-or-later".to_string()), license_file: None },
            Package { name: "ok".to_string(), version: "1.0.0".to_string(), id: "ok".to_string(), source: Some(CRATES_IO.to_string()), license: Some("MIT".to_string()), license_file: None },
            Package { name: "affected".to_string(), version: "1.2.3".to_string(), id: "affected".to_string(), source: Some(CRATES_IO.to_string()), license: Some("MIT".to_string()), license_file: None },
            Package { name: "unknown".to_string(), version: "2.0.0".to_string(), id: "unknown".to_string(), source: Some("git+https://example.invalid/repo".to_string()), license: None, license_file: Some("LICENSE".to_string()) },
        ],
        resolve: Some(Resolve { nodes: vec![
            Node { id: "root".to_string(), dependencies: vec!["ok".to_string(), "affected".to_string()], deps: vec![ResolvedDependency { pkg: "ok".to_string() }] },
            Node { id: "ok".to_string(), dependencies: vec![], deps: vec![] },
            Node { id: "affected".to_string(), dependencies: vec![], deps: vec![] },
        ] }),
        workspace_members: vec!["root".to_string()],
    };
    let advisories = Advisories {
        schema: ADVISORY_SCHEMA.to_string(),
        revision: "fixture".to_string(),
        generated_at: "2026-11-12".to_string(),
        advisories: vec![Advisory { id: "RUSTSEC-FIXTURE".to_string(), package: "affected".to_string(), versions: vec!["1.2.3".to_string()], severity: "high".to_string() }],
    };
    let report = audit(&metadata, &policy, Some(&advisories), Some(b"{}"), "2026-11-12", "blake3:m", "blake3:p")?;
    if report.decision != "deny" || report.summary.actionable_advisories != 1 || report.summary.unrecognized_sources != 1 || report.summary.missing_licenses != 1 {
        return Err("negative fixture did not produce the expected findings".to_string());
    }
    let mut excepted = policy;
    excepted.exceptions.push(PolicyException {
        package: "affected".to_string(),
        source: CRATES_IO.to_string(),
        advisory: Some("RUSTSEC-FIXTURE".to_string()),
        owner: "owner".to_string(),
        rationale: "fixture".to_string(),
        expires: "2026-12-31".to_string(),
    });
    let report = audit(&metadata, &excepted, Some(&advisories), Some(b"{}"), "2026-11-12", "blake3:m", "blake3:p")?;
    if report.summary.excepted_advisories != 1 {
        return Err("current exception was not recorded".to_string());
    }
    let mut expired = excepted.clone();
    expired.exceptions[0].expires = "2026-01-01".to_string();
    let report = audit(&metadata, &expired, Some(&advisories), Some(b"{}"), "2026-11-12", "blake3:m", "blake3:p")?;
    if report.summary.actionable_advisories == 0 {
        return Err("expired exception did not become actionable".to_string());
    }
    let report = audit(&metadata, &excepted, None, None, "2026-11-12", "blake3:m", "blake3:p")?;
    if report.decision != "deny" {
        return Err("missing advisory snapshot did not deny".to_string());
    }
    println!("dependency audit self-test ok");
    Ok(())
}

