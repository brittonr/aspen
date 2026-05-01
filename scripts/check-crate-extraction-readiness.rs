#!/usr/bin/env -S RUSTC_WRAPPER= CARGO_INCREMENTAL= nix develop -c cargo -q -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
---

use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;

const OWNER_NEEDED: &str = "owner needed";
const WORKSPACE_INTERNAL: &str = "workspace-internal";
const MANIFEST_REQUIRED_SECTION_GROUPS: &[&[&str]] = &[
    &["## Candidate"],
    &["## Package and release metadata", "## Package metadata"],
    &["## Feature contract"],
    &["## Dependencies", "## Dependency decisions"],
    &["## Compatibility and aliases", "## Compatibility plan"],
    &["## Representative consumers", "Representative consumers:"],
    &["## Dependency exceptions", "## Dependency decisions"],
    &["## Verification rails"],
];
const BOUNDARY_RAIL_PHRASES: &[&str] = &[
    "positive downstream",
    "negative boundary",
    "compatibility",
    "dependency-boundary",
];
const BLOB_CASTORE_CACHE_FAMILY: &str = "blob-castore-cache";
const KV_BRANCH_COMMIT_DAG_FAMILY: &str = "kv-branch-commit-dag";
const PROTOCOL_WIRE_FAMILY: &str = "protocol-wire";
const TRANSPORT_RPC_FAMILY: &str = "transport-rpc";
const BLOB_CASTORE_CACHE_DOWNSTREAM_EVIDENCE: &[&str] = &[
    "i6-downstream-blob-metadata.json",
    "i6-downstream-cache-castore-metadata.json",
    "i6-downstream-blob-forbidden-grep.txt",
    "i6-downstream-cache-castore-forbidden-grep.txt",
];
const KV_BRANCH_COMMIT_DAG_DOWNSTREAM_EVIDENCE: &[&str] = &[
    "i5-downstream-branch-dag-metadata.json",
    "i5-downstream-branch-dag-forbidden-grep.txt",
];
const PROTOCOL_WIRE_DOWNSTREAM_EVIDENCE: &[&str] = &[
    "i5-downstream-protocol-wire-metadata.json",
    "i5-downstream-protocol-wire-forbidden-grep.txt",
    "i3-client-api-compatibility-tests.txt",
];
const TRANSPORT_RPC_DOWNSTREAM_EVIDENCE: &[&str] = &[
    "i7-downstream-transport-metadata.json",
    "i7-downstream-transport-forbidden-grep.txt",
    "i7-downstream-rpc-metadata.json",
    "i7-downstream-rpc-forbidden-grep.txt",
    "v4-compatibility-summary.txt",
];
const FOUNDATIONAL_TYPES_FAMILY: &str = "foundational-types";
const AUTH_TICKET_FAMILY: &str = "auth-ticket";
const JOBS_CI_CORE_FAMILY: &str = "jobs-ci-core";
const TRUST_CRYPTO_SECRETS_FAMILY: &str = "trust-crypto-secrets";
const TESTING_HARNESS_FAMILY: &str = "testing-harness";
const FOUNDATIONAL_TYPES_EVIDENCE: &[&str] = &[
    "foundational-types-downstream-metadata.json",
    "foundational-types-forbidden-boundary.txt",
    "foundational-types-compatibility.txt",
];
const AUTH_TICKET_EVIDENCE: &[&str] = &[
    "auth-ticket-downstream-metadata.json",
    "auth-ticket-forbidden-boundary.txt",
    "auth-ticket-compatibility.txt",
];
const JOBS_CI_CORE_EVIDENCE: &[&str] = &[
    "jobs-ci-core-downstream-metadata.json",
    "jobs-ci-core-forbidden-boundary.txt",
    "jobs-ci-core-compatibility.txt",
];
const TRUST_CRYPTO_SECRETS_EVIDENCE: &[&str] = &[
    "trust-crypto-secrets-downstream-metadata.json",
    "trust-crypto-secrets-forbidden-boundary.txt",
    "trust-crypto-secrets-compatibility.txt",
];
const TESTING_HARNESS_EVIDENCE: &[&str] = &[
    "testing-harness-downstream-metadata.json",
    "testing-harness-forbidden-boundary.txt",
    "testing-harness-compatibility.txt",
];

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    policy: PathBuf,
    #[arg(long)]
    inventory: PathBuf,
    #[arg(long)]
    manifest_dir: PathBuf,
    #[arg(long)]
    candidate_family: String,
    #[arg(long)]
    output_json: PathBuf,
    #[arg(long)]
    output_markdown: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Policy {
    blocked_until_license_publication_decision: Vec<String>,
    forbidden_by_default: Vec<String>,
    feature_gated_by_default: Vec<String>,
    concrete_transport_crates: Vec<String>,
    allowed_reusable_dependencies: Vec<String>,
    candidates: BTreeMap<String, Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    class: String,
    readiness_state: String,
    manifest: String,
    owner: String,
    default_feature_sets: Vec<String>,
    named_reusable_feature_sets: Vec<String>,
    representative_consumers: Vec<String>,
    reexporters: Vec<String>,
    forbidden_unless_feature: Vec<String>,
    exceptions: Vec<Exception>,
}

#[derive(Debug, Deserialize)]
struct Exception {
    candidate: String,
    feature_set: String,
    dependency_path: String,
    owner: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoPackage>,
}

#[derive(Debug, Deserialize)]
struct CargoPackage {
    name: String,
    dependencies: Vec<CargoDependency>,
}

#[derive(Debug, Deserialize)]
struct CargoDependency {
    name: String,
    kind: Option<String>,
    optional: bool,
}

#[derive(Debug, Serialize)]
struct Report {
    candidate_family: String,
    passed: bool,
    failures: Vec<String>,
    warnings: Vec<String>,
    checked_candidates: Vec<String>,
}

fn run_command_text(program: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(program).args(args).output().with_context(|| format!("failed to run {program}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("{program} failed: {stderr}");
    }
    String::from_utf8(output.stdout).context("command stdout was not UTF-8")
}

fn export_policy(policy_path: &Path) -> Result<Policy> {
    let path_text = policy_path.to_str().context("policy path is not valid UTF-8")?;
    let json = run_command_text("nix", &["run", "nixpkgs#nickel", "--", "export", "--format", "json", path_text])?;
    serde_json::from_str(&json).context("failed to parse exported Nickel policy JSON")
}

fn load_metadata() -> Result<CargoMetadata> {
    let json = run_command_text("cargo", &["metadata", "--format-version", "1", "--no-deps"])?;
    serde_json::from_str(&json).context("failed to parse cargo metadata JSON")
}

fn package_for_candidate(candidate_key: &str) -> Option<&'static str> {
    match candidate_key {
        "aspen_kv_types" => Some("aspen-kv-types"),
        "aspen_raft_kv_types" => Some("aspen-raft-kv-types"),
        "aspen_redb_storage" => Some("aspen-redb-storage"),
        "aspen_raft_network" => Some("aspen-raft-network"),
        "aspen_raft_compat" => Some("aspen-raft"),
        "aspen_raft_kv" => Some("aspen-raft-kv"),
        "aspen_coordination" => Some("aspen-coordination"),
        "aspen_coordination_protocol" => Some("aspen-coordination-protocol"),
        "aspen_client_api" => Some("aspen-client-api"),
        "aspen_forge_protocol" => Some("aspen-forge-protocol"),
        "aspen_jobs_protocol" => Some("aspen-jobs-protocol"),
        "aspen_transport" => Some("aspen-transport"),
        "aspen_rpc_core" => Some("aspen-rpc-core"),
        "aspen_blob" => Some("aspen-blob"),
        "aspen_castore" => Some("aspen-castore"),
        "aspen_cache" => Some("aspen-cache"),
        "aspen_commit_dag" => Some("aspen-commit-dag"),
        "aspen_kv_branch" => Some("aspen-kv-branch"),
        _ => None,
    }
}

fn candidate_keys_for_family(family: &str) -> Option<&'static [&'static str]> {
    match family {
        BLOB_CASTORE_CACHE_FAMILY => Some(&["aspen_blob", "aspen_castore", "aspen_cache"]),
        KV_BRANCH_COMMIT_DAG_FAMILY => Some(&["aspen_commit_dag", "aspen_kv_branch"]),
        PROTOCOL_WIRE_FAMILY => Some(&["aspen_client_api", "aspen_coordination_protocol", "aspen_forge_protocol", "aspen_jobs_protocol"]),
        TRANSPORT_RPC_FAMILY => Some(&["aspen_transport", "aspen_rpc_core"]),
        FOUNDATIONAL_TYPES_FAMILY => Some(&["foundational_types"]),
        AUTH_TICKET_FAMILY => Some(&["auth_ticket"]),
        JOBS_CI_CORE_FAMILY => Some(&["jobs_ci_core"]),
        TRUST_CRYPTO_SECRETS_FAMILY => Some(&["trust_crypto_secrets"]),
        TESTING_HARNESS_FAMILY => Some(&["testing_harness"]),
        _ => None,
    }
}

fn is_runtime_shell(candidate: &Candidate) -> bool {
    candidate.class == "runtime adapter" || candidate.manifest.contains("compat")
}

fn is_aspen_crate(crate_name: &str) -> bool {
    crate_name == "aspen" || crate_name.starts_with("aspen-")
}

fn exception_allows_dependency(candidate: &Candidate, dependency_name: &str) -> bool {
    candidate.exceptions.iter().any(|exception| exception.dependency_path.contains(dependency_name))
}

fn collect_default_forbidden(policy: &Policy, candidate: &Candidate) -> BTreeSet<String> {
    policy
        .forbidden_by_default
        .iter()
        .chain(policy.feature_gated_by_default.iter())
        .chain(policy.concrete_transport_crates.iter())
        .chain(candidate.forbidden_unless_feature.iter())
        .cloned()
        .collect()
}

fn is_normal_dependency(dependency: &CargoDependency) -> bool {
    dependency.kind.is_none() && !dependency.optional
}

fn check_exception(candidate_key: &str, exception: &Exception, failures: &mut Vec<String>) {
    if exception.candidate.trim().is_empty() {
        failures.push(format!("{candidate_key}: exception has empty candidate"));
    }
    if exception.feature_set.trim().is_empty() {
        failures.push(format!("{candidate_key}: exception has empty feature_set"));
    }
    if exception.dependency_path.trim().is_empty() {
        failures.push(format!("{candidate_key}: exception has empty dependency_path"));
    }
    if exception.owner.trim().is_empty() || exception.owner == OWNER_NEEDED {
        failures.push(format!("{candidate_key}: exception `{}` has unassigned owner", exception.dependency_path));
    }
    if exception.reason.trim().is_empty() {
        failures.push(format!("{candidate_key}: exception `{}` has empty reason", exception.dependency_path));
    }
}

fn manifest_text_allows_no_reexporter(text: &str) -> bool {
    let normalized = text.to_lowercase().replace('*', "");
    normalized.contains("compatibility re-exports: none")
}

fn check_manifest(candidate_key: &str, candidate: &Candidate, failures: &mut Vec<String>) -> Result<String> {
    let manifest_path = Path::new(&candidate.manifest);
    let text =
        fs::read_to_string(manifest_path).with_context(|| format!("failed to read manifest {}", candidate.manifest))?;
    for alternatives in MANIFEST_REQUIRED_SECTION_GROUPS {
        if !alternatives.iter().any(|section| text.contains(section)) {
            failures.push(format!("{candidate_key}: manifest missing one of `{}`", alternatives.join("` / `")));
        }
    }
    let lower = text.to_lowercase();
    for phrase in BOUNDARY_RAIL_PHRASES {
        if !lower.contains(phrase) {
            failures.push(format!("{candidate_key}: manifest missing boundary rail phrase `{phrase}`"));
        }
    }
    Ok(text)
}

fn check_readiness(candidate_key: &str, candidate: &Candidate, blocked: &BTreeSet<String>, failures: &mut Vec<String>) {
    if blocked.contains(&candidate.readiness_state) {
        failures.push(format!("{candidate_key}: forbidden readiness state `{}`", candidate.readiness_state));
    }
    if candidate.readiness_state != WORKSPACE_INTERNAL && candidate.owner == OWNER_NEEDED {
        failures.push(format!("{candidate_key}: ready candidate has no owner"));
    }
    if candidate.default_feature_sets.is_empty() {
        failures.push(format!("{candidate_key}: no default feature set recorded"));
    }
}

fn check_direct_deps(
    candidate_key: &str,
    candidate: &Candidate,
    metadata: &CargoMetadata,
    forbidden: &BTreeSet<String>,
    allowed_reusable: &BTreeSet<String>,
    failures: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let Some(package_name) = package_for_candidate(candidate_key) else {
        warnings.push(format!("{candidate_key}: package does not exist yet; direct dependency check deferred"));
        return;
    };
    let Some(package) = metadata.packages.iter().find(|pkg| pkg.name == package_name) else {
        warnings.push(format!("{candidate_key}: package `{package_name}` not found in cargo metadata"));
        return;
    };
    if is_runtime_shell(candidate) {
        return;
    }
    for dep in package.dependencies.iter().filter(|dep| is_normal_dependency(dep)) {
        let has_exception = exception_allows_dependency(candidate, &dep.name);
        if forbidden.contains(&dep.name) && !has_exception {
            failures.push(format!("{candidate_key}: direct forbidden dependency `{}`", dep.name));
        }
        if is_aspen_crate(&dep.name) && !allowed_reusable.contains(&dep.name) && !has_exception {
            failures.push(format!(
                "{candidate_key}: direct dependency `{}` is not in allowed reusable dependencies",
                dep.name
            ));
        }
    }
}

fn check_transitive_deps(
    candidate_key: &str,
    candidate: &Candidate,
    forbidden: &BTreeSet<String>,
    failures: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let Some(package_name) = package_for_candidate(candidate_key) else {
        return;
    };
    if is_runtime_shell(candidate) {
        return;
    }
    let output = Command::new("cargo").args(["tree", "-p", package_name, "-e", "normal"]).output();
    let Ok(output) = output else {
        warnings.push(format!("{candidate_key}: could not run cargo tree"));
        return;
    };
    if !output.status.success() {
        warnings.push(format!("{candidate_key}: cargo tree failed for `{package_name}`"));
        return;
    }
    let tree = String::from_utf8_lossy(&output.stdout);
    for forbidden_name in forbidden {
        let needle = format!("{forbidden_name} v");
        if tree.contains(&needle) && !exception_allows_dependency(candidate, forbidden_name) {
            failures.push(format!("{candidate_key}: transitive forbidden dependency `{forbidden_name}`"));
        }
    }
}

fn check_evidence_index(args: &Args, failures: &mut Vec<String>) {
    let Some(evidence_dir) = args.output_markdown.parent() else {
        failures.push("could not infer evidence directory from output path".to_string());
        return;
    };
    let Some(change_dir) = evidence_dir.parent() else {
        failures.push("could not infer change directory from output path".to_string());
        return;
    };
    let verification_path = change_dir.join("verification.md");
    if !verification_path.exists() {
        failures.push(format!("missing verification index `{}`", verification_path.display()));
        return;
    }
    let Ok(text) = fs::read_to_string(&verification_path) else {
        failures.push(format!("failed to read `{}`", verification_path.display()));
        return;
    };
    if !text.contains("## Task Coverage") || !text.contains("- Evidence:") {
        failures.push("verification index lacks task coverage evidence lines".to_string());
    }
    check_family_evidence(args, evidence_dir, failures);
}

fn check_family_evidence(args: &Args, evidence_dir: &Path, failures: &mut Vec<String>) {
    let required_files = match args.candidate_family.as_str() {
        BLOB_CASTORE_CACHE_FAMILY => BLOB_CASTORE_CACHE_DOWNSTREAM_EVIDENCE,
        KV_BRANCH_COMMIT_DAG_FAMILY => KV_BRANCH_COMMIT_DAG_DOWNSTREAM_EVIDENCE,
        PROTOCOL_WIRE_FAMILY => PROTOCOL_WIRE_DOWNSTREAM_EVIDENCE,
        TRANSPORT_RPC_FAMILY => TRANSPORT_RPC_DOWNSTREAM_EVIDENCE,
        FOUNDATIONAL_TYPES_FAMILY => FOUNDATIONAL_TYPES_EVIDENCE,
        AUTH_TICKET_FAMILY => AUTH_TICKET_EVIDENCE,
        JOBS_CI_CORE_FAMILY => JOBS_CI_CORE_EVIDENCE,
        TRUST_CRYPTO_SECRETS_FAMILY => TRUST_CRYPTO_SECRETS_EVIDENCE,
        TESTING_HARNESS_FAMILY => TESTING_HARNESS_EVIDENCE,
        _ => return,
    };

    for file_name in required_files {
        let artifact = evidence_dir.join(file_name);
        if !artifact.exists() {
            let evidence_kind = if file_name.contains("compatibility") {
                "compatibility evidence"
            } else if file_name.contains("forbidden") || file_name.contains("boundary") {
                "negative boundary evidence"
            } else {
                "downstream fixture evidence"
            };
            failures.push(format!(
                "{}: missing {} `{}`",
                args.candidate_family,
                evidence_kind,
                artifact.display()
            ));
        }
    }
}

fn selected_family_requires_strict_owner(family: &str) -> bool {
    matches!(
        family,
        FOUNDATIONAL_TYPES_FAMILY
            | AUTH_TICKET_FAMILY
            | JOBS_CI_CORE_FAMILY
            | TRUST_CRYPTO_SECRETS_FAMILY
            | TESTING_HARNESS_FAMILY
    )
}

fn check_selected_family_policy_contract(
    family: &str,
    candidate_key: &str,
    candidate: &Candidate,
    failures: &mut Vec<String>,
) {
    if !selected_family_requires_strict_owner(family) {
        return;
    }
    if candidate.owner.trim().is_empty() || candidate.owner == OWNER_NEEDED {
        failures.push(format!("{candidate_key}: selected family has unassigned owner"));
    }
    if !candidate.forbidden_unless_feature.iter().any(|name| name == "aspen") {
        failures.push(format!("{candidate_key}: selected family does not forbid root `aspen` by default"));
    }
    if candidate.representative_consumers.is_empty() {
        failures.push(format!("{candidate_key}: selected family has no representative consumers"));
    }
}

fn build_report(args: &Args) -> Result<Report> {
    let policy = export_policy(&args.policy)?;
    let metadata = load_metadata()?;
    let blocked: BTreeSet<String> = policy.blocked_until_license_publication_decision.iter().cloned().collect();
    let allowed_reusable: BTreeSet<String> = policy.allowed_reusable_dependencies.iter().cloned().collect();
    let mut failures = Vec::new();
    let mut warnings = Vec::new();
    let mut checked_candidates = Vec::new();

    if !args.inventory.exists() {
        failures.push(format!("missing inventory `{}`", args.inventory.display()));
    }
    if !args.manifest_dir.exists() {
        failures.push(format!("missing manifest dir `{}`", args.manifest_dir.display()));
    }

    let Some(family_keys) = candidate_keys_for_family(&args.candidate_family) else {
        failures.push(format!("unknown candidate family `{}`", args.candidate_family));
        return Ok(Report {
            candidate_family: args.candidate_family.clone(),
            passed: false,
            failures,
            warnings,
            checked_candidates,
        });
    };

    for candidate_key in family_keys {
        let Some(candidate) = policy.candidates.get(*candidate_key) else {
            failures.push(format!("{}: missing policy candidate entry", candidate_key));
            continue;
        };
        checked_candidates.push((*candidate_key).to_string());
        check_readiness(candidate_key, candidate, &blocked, &mut failures);
        check_selected_family_policy_contract(&args.candidate_family, candidate_key, candidate, &mut failures);
        let manifest_text = check_manifest(candidate_key, candidate, &mut failures)?;
        if candidate.representative_consumers.is_empty() {
            failures.push(format!("{candidate_key}: no representative consumers"));
        }
        if candidate.reexporters.is_empty() && !manifest_text_allows_no_reexporter(&manifest_text) {
            failures.push(format!("{candidate_key}: no re-exporters recorded"));
        }
        for exception in &candidate.exceptions {
            check_exception(candidate_key, exception, &mut failures);
        }
        let forbidden = collect_default_forbidden(&policy, candidate);
        check_direct_deps(
            candidate_key,
            candidate,
            &metadata,
            &forbidden,
            &allowed_reusable,
            &mut failures,
            &mut warnings,
        );
        check_transitive_deps(candidate_key, candidate, &forbidden, &mut failures, &mut warnings);
        if !candidate.forbidden_unless_feature.is_empty() && candidate.named_reusable_feature_sets.is_empty() {
            failures
                .push(format!("{candidate_key}: feature-gated forbiddens listed but no named reusable feature sets"));
        }
    }

    check_evidence_index(args, &mut failures);
    Ok(Report {
        candidate_family: args.candidate_family.clone(),
        passed: failures.is_empty(),
        failures,
        warnings,
        checked_candidates,
    })
}

fn write_report(report: &Report, args: &Args) -> Result<()> {
    if let Some(parent) = args.output_json.parent() {
        fs::create_dir_all(parent).with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(report).context("failed to encode JSON report")?;
    fs::write(&args.output_json, json).with_context(|| format!("failed to write {}", args.output_json.display()))?;

    let mut markdown = String::new();
    markdown.push_str("# Crate Extraction Readiness Report\n\n");
    markdown.push_str(&format!("- Candidate family: `{}`\n", report.candidate_family));
    markdown.push_str(&format!("- Passed: `{}`\n", report.passed));
    markdown.push_str(&format!("- Checked candidates: {}\n\n", report.checked_candidates.len()));
    markdown.push_str("## Failures\n\n");
    if report.failures.is_empty() {
        markdown.push_str("- none\n");
    } else {
        for failure in &report.failures {
            markdown.push_str(&format!("- {failure}\n"));
        }
    }
    markdown.push_str("\n## Warnings\n\n");
    if report.warnings.is_empty() {
        markdown.push_str("- none\n");
    } else {
        for warning in &report.warnings {
            markdown.push_str(&format!("- {warning}\n"));
        }
    }
    fs::write(&args.output_markdown, markdown)
        .with_context(|| format!("failed to write {}", args.output_markdown.display()))?;
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let report = build_report(&args)?;
    write_report(&report, &args)?;
    if report.passed {
        Ok(())
    } else {
        anyhow::bail!("crate extraction readiness check failed with {} failure(s)", report.failures.len())
    }
}
