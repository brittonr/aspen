#!/usr/bin/env -S RUSTC_WRAPPER= CARGO_INCREMENTAL= nix develop -c cargo -q -Zscript
---cargo
[package]
edition = "2024"

[dependencies]
anyhow = "1"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
---

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use serde::Deserialize;

const DEFAULT_READY_FAMILIES: &[&str] = &[
    "foundational-types",
    "auth-ticket",
    "jobs-ci-core",
    "trust-crypto-secrets",
    "testing-harness",
    "protocol-wire",
    "blob-castore-cache",
    "coordination",
];

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
const PROTOCOL_WIRE_EVIDENCE: &[&str] = &[
    "i5-downstream-protocol-wire-metadata.json",
    "i5-downstream-protocol-wire-forbidden-grep.txt",
    "i3-client-api-compatibility-tests.txt",
];
const BLOB_CASTORE_CACHE_EVIDENCE: &[&str] = &[
    "i6-downstream-blob-metadata.json",
    "i6-downstream-cache-castore-metadata.json",
    "i6-downstream-blob-forbidden-grep.txt",
    "i6-downstream-cache-castore-forbidden-grep.txt",
];

#[derive(Debug, Parser)]
#[command(about = "Run crate-extraction readiness checks for ready extraction families")]
struct Args {
    /// Family to check; may be repeated. Defaults to all ready extraction families.
    #[arg(long = "family")]
    families: Vec<String>,

    #[arg(long, default_value = "docs/crate-extraction/policy.ncl")]
    policy: PathBuf,

    #[arg(long, default_value = "docs/crate-extraction.md")]
    inventory: PathBuf,

    #[arg(long, default_value = "docs/crate-extraction")]
    manifest_dir: PathBuf,

    /// Keep the temporary evidence fixture under target/ and print its path.
    #[arg(long)]
    keep_temp: bool,
}

#[derive(Debug, Deserialize)]
struct Report {
    candidate_family: String,
    passed: bool,
    failures: Vec<String>,
    warnings: Vec<String>,
    checked_candidates: Vec<String>,
}

#[derive(Debug)]
struct FamilyResult {
    report: Report,
    exit_code: i32,
    stderr: String,
}

fn repo_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("failed to run git rev-parse --show-toplevel")?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse --show-toplevel failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    let stdout = String::from_utf8(output.stdout).context("git rev-parse output was not UTF-8")?;
    Ok(PathBuf::from(stdout.trim()))
}

fn required_evidence_for_family(family: &str) -> &'static [&'static str] {
    match family {
        "foundational-types" => FOUNDATIONAL_TYPES_EVIDENCE,
        "auth-ticket" => AUTH_TICKET_EVIDENCE,
        "jobs-ci-core" => JOBS_CI_CORE_EVIDENCE,
        "trust-crypto-secrets" => TRUST_CRYPTO_SECRETS_EVIDENCE,
        "testing-harness" => TESTING_HARNESS_EVIDENCE,
        "protocol-wire" => PROTOCOL_WIRE_EVIDENCE,
        "blob-castore-cache" => BLOB_CASTORE_CACHE_EVIDENCE,
        _ => &[],
    }
}

fn write_fixture(change_dir: &Path, family: &str) -> Result<PathBuf> {
    let evidence_dir = change_dir.join("evidence");
    fs::create_dir_all(&evidence_dir).with_context(|| format!("failed to create {}", evidence_dir.display()))?;
    fs::write(
        change_dir.join("verification.md"),
        format!(
            "# Temporary crate-extraction readiness sweep fixture\n\n\
             ## Task Coverage\n\n\
             - Evidence: generated temporary fixture for `{family}` readiness sweep.\n"
        ),
    )
    .with_context(|| format!("failed to write {}/verification.md", change_dir.display()))?;

    for file_name in required_evidence_for_family(family) {
        let artifact = evidence_dir.join(file_name);
        fs::write(
            &artifact,
            format!(
                "temporary fixture for scripts/check-crate-extraction-ready-families.rs; \
                 family={family}; artifact={file_name}\n"
            ),
        )
        .with_context(|| format!("failed to write {}", artifact.display()))?;
    }
    Ok(evidence_dir)
}

fn run_family(root: &Path, family: &str, work_dir: &Path, args: &Args) -> Result<FamilyResult> {
    let change_dir = work_dir.join(family);
    let evidence_dir = write_fixture(&change_dir, family)?;
    let output_json = evidence_dir.join("readiness.json");
    let output_markdown = evidence_dir.join("readiness.md");
    let output = Command::new("cargo")
        .current_dir(root)
        .env("RUSTC_WRAPPER", "")
        .env("CARGO_INCREMENTAL", "0")
        .args(["-q", "-Zscript"])
        .arg(root.join("scripts/check-crate-extraction-readiness.rs"))
        .arg("--policy")
        .arg(&args.policy)
        .arg("--inventory")
        .arg(&args.inventory)
        .arg("--manifest-dir")
        .arg(&args.manifest_dir)
        .arg("--candidate-family")
        .arg(family)
        .arg("--output-json")
        .arg(&output_json)
        .arg("--output-markdown")
        .arg(&output_markdown)
        .output()
        .with_context(|| format!("failed to run readiness checker for {family}"))?;

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let exit_code = output.status.code().unwrap_or(1);
    let report = if output_json.exists() {
        let text =
            fs::read_to_string(&output_json).with_context(|| format!("failed to read {}", output_json.display()))?;
        serde_json::from_str(&text).with_context(|| format!("failed to parse {}", output_json.display()))?
    } else {
        Report {
            candidate_family: family.to_string(),
            passed: false,
            failures: vec!["readiness checker did not write JSON report".to_string()],
            warnings: Vec::new(),
            checked_candidates: Vec::new(),
        }
    };
    Ok(FamilyResult {
        report,
        exit_code,
        stderr,
    })
}

fn print_result(result: &FamilyResult) {
    let passed = result.report.passed && result.exit_code == 0;
    let status = if passed { "PASS" } else { "FAIL" };
    println!(
        "{status} {} checked={} warnings={}",
        result.report.candidate_family,
        result.report.checked_candidates.len(),
        result.report.warnings.len()
    );
    if !passed {
        for failure in &result.report.failures {
            println!("  - {failure}");
        }
        if !result.stderr.is_empty() {
            println!("  - {}", result.stderr);
        }
    }
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<()> {
    if destination.exists() {
        fs::remove_dir_all(destination).with_context(|| format!("failed to remove {}", destination.display()))?;
    }
    fs::create_dir_all(destination).with_context(|| format!("failed to create {}", destination.display()))?;
    for entry in fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target).with_context(|| format!("failed to copy {}", target.display()))?;
        }
    }
    Ok(())
}

fn run(args: Args) -> Result<i32> {
    let root = repo_root()?;
    let families: Vec<String> = if args.families.is_empty() {
        DEFAULT_READY_FAMILIES.iter().map(|family| (*family).to_string()).collect()
    } else {
        args.families.clone()
    };
    let temp_dir = tempfile::Builder::new()
        .prefix("aspen-crate-extraction-sweep-")
        .tempdir()
        .context("failed to create temporary sweep directory")?;

    let results: Vec<FamilyResult> = families
        .iter()
        .map(|family| run_family(&root, family, temp_dir.path(), &args))
        .collect::<Result<Vec<_>>>()?;

    let mut failed = false;
    for result in &results {
        print_result(result);
        failed |= !result.report.passed || result.exit_code != 0;
    }

    if args.keep_temp {
        let keep_path = root.join("target/crate-extraction-ready-family-sweep");
        copy_dir_all(temp_dir.path(), &keep_path)?;
        println!("kept temporary evidence fixture at {}", keep_path.display());
    }

    Ok(if failed { 1 } else { 0 })
}

fn main() -> Result<()> {
    let args = Args::parse();
    let exit_code = run(args)?;
    if exit_code == 0 {
        Ok(())
    } else {
        std::process::exit(exit_code);
    }
}
