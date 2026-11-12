#!/usr/bin/env -S CARGO_TARGET_DIR=/tmp/molten-capability-report-target nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#gcc -c cargo -q -Zscript
---
[dependencies]
blake3 = "=1.8.5"
serde = { version = "=1.0.228", features = ["derive"] }
serde_json = "=1.0.149"
---
//! Read-only capability qualification report.
//!
//! The report joins the reviewed evidence matrix with a caller-supplied bundle
//! of exact execution receipts. Implementation coverage and recorded execution
//! remain separate fields. A placeholder ref, missing file, wrong subject or
//! scope, or unexecuted matrix row cannot become bounded-observed evidence.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};

const INPUT_SCHEMA: &str = "molten.capability-report.v1";
const MATRIX_SCHEMA: &str = "molten.testing.evidence-matrix.v1";
const RECEIPT_SCHEMA: &str = "molten.capability-evidence.v1";
const NON_CLAIM: &str = "diagnostic report only; it grants no authority, release admission, or production-readiness claim";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    schema: String,
    entries: Vec<MatrixEntry>,
    #[serde(default)]
    exemptions: Vec<Exemption>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MatrixEntry {
    requirement_id: String,
    coverage_kind: String,
    evidence_scope: String,
    target: String,
    command: String,
    #[serde(default)]
    artifact_refs: Vec<String>,
    receipt_ref: String,
    #[serde(default)]
    caveats: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Exemption {
    requirement_id: String,
    reason_class: String,
    evidence_ref: String,
    scope: String,
    review_note: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    schema: String,
    source_revision: String,
    dependency_closure_ref: String,
    feature_set: Vec<String>,
    execution_profile: String,
    platform: String,
    capabilities: Vec<Capability>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    id: String,
    requirements: Vec<String>,
    profiles: Vec<String>,
    evidence: Vec<Evidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    requirement_id: String,
    receipt_path: String,
    declared_ref: String,
    subject_ref: String,
    scope_ref: String,
    decision: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: String,
    requirement_id: String,
    subject_ref: String,
    scope_ref: String,
    decision: String,
    #[serde(default)]
    caveats: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    schema: &'static str,
    source_revision: String,
    dependency_closure_ref: String,
    feature_set: Vec<String>,
    execution_profile: String,
    platform: String,
    capabilities: Vec<CapabilityReport>,
    non_claim: &'static str,
}

#[derive(Debug, Serialize)]
struct CapabilityReport {
    id: String,
    requirements: Vec<String>,
    profiles: Vec<String>,
    implementation: Implementation,
    execution: Execution,
}

#[derive(Debug, Serialize)]
struct Implementation {
    matrix_entries: usize,
    coverage_kinds: Vec<String>,
    scopes: Vec<String>,
    targets: Vec<String>,
}

#[derive(Debug, Serialize)]
struct Execution {
    status: String,
    evidence: Vec<EvidenceReport>,
    diagnostics: Vec<String>,
}

#[derive(Debug, Serialize)]
struct EvidenceReport {
    requirement_id: String,
    declared_ref: String,
    subject_ref: String,
    scope_ref: String,
    decision: String,
    validated: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("capability report denied: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let mut matrix_path = None;
    let mut input_path = None;
    let mut markdown_out = None;
    let mut json_out = None;
    let mut self_test = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--matrix" => matrix_path = Some(next_path(&mut args, "--matrix")?),
            "--input" => input_path = Some(next_path(&mut args, "--input")?),
            "--markdown-out" => markdown_out = Some(next_path(&mut args, "--markdown-out")?),
            "--json-out" => json_out = Some(next_path(&mut args, "--json-out")?),
            "--self-test" => self_test = true,
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if self_test {
        return self_tests();
    }
    let matrix_path = matrix_path.ok_or("usage: capability-report --matrix FILE --input FILE [--markdown-out FILE] [--json-out FILE]")?;
    let input_path = input_path.ok_or("missing --input")?;
    let matrix: Matrix = read_json(&matrix_path)?;
    let input: Input = read_json(&input_path)?;
    let report = build(&matrix, &input)?;
    let markdown = render_markdown(&report);
    let wrote_markdown = markdown_out.is_some();
    let wrote_json = json_out.is_some();
    if let Some(path) = &markdown_out {
        write_out(path, &markdown)?;
    }
    if let Some(path) = &json_out {
        write_out(path, &(serde_json::to_string_pretty(&report).map_err(|e| e.to_string())? + "\n"))?;
    }
    if !wrote_markdown && !wrote_json {
        print!("{markdown}");
    }
    Ok(())
}

fn next_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    args.next().map(PathBuf::from).ok_or_else(|| format!("{flag} requires a path"))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", path.display()))
}

fn write_out(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|error| format!("{}: {error}", path.display()))
}

fn build(matrix: &Matrix, input: &Input) -> Result<Report, String> {
    if matrix.schema != MATRIX_SCHEMA {
        return Err(format!("matrix schema must be {MATRIX_SCHEMA}"));
    }
    if input.schema != INPUT_SCHEMA {
        return Err(format!("input schema must be {INPUT_SCHEMA}"));
    }
    for (name, value) in [
        ("source_revision", input.source_revision.as_str()),
        ("dependency_closure_ref", input.dependency_closure_ref.as_str()),
        ("execution_profile", input.execution_profile.as_str()),
        ("platform", input.platform.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("input {name} must not be empty"));
        }
    }
    let mut seen = BTreeSet::new();
    let mut capabilities = Vec::new();
    for capability in &input.capabilities {
        if !seen.insert(capability.id.clone()) {
            return Err(format!("duplicate capability id: {}", capability.id));
        }
        if capability.requirements.is_empty() {
            return Err(format!("capability {} has no requirements", capability.id));
        }
        let requirement_set: BTreeSet<&str> = capability.requirements.iter().map(String::as_str).collect();
        let mut evidence_reports = Vec::new();
        let mut diagnostics = Vec::new();
        let mut has_failed = false;
        let mut has_inconclusive = false;
        let mut has_valid_pass = false;
        for evidence in &capability.evidence {
            if !requirement_set.contains(evidence.requirement_id.as_str()) {
                return Err(format!("capability {} evidence names unlisted requirement {}", capability.id, evidence.requirement_id));
            }
            let (validated, decision, caveats) = validate_evidence(evidence)?;
            diagnostics.extend(caveats);
            match decision.as_str() {
                "pass" if validated => has_valid_pass = true,
                "fail" => has_failed = true,
                "inconclusive" | "unsupported" => has_inconclusive = true,
                _ => {}
            }
            evidence_reports.push(EvidenceReport {
                requirement_id: evidence.requirement_id.clone(),
                declared_ref: evidence.declared_ref.clone(),
                subject_ref: evidence.subject_ref.clone(),
                scope_ref: evidence.scope_ref.clone(),
                decision,
                validated,
            });
        }
        let matching_entries = matrix
            .entries
            .iter()
            .filter(|entry| requirement_set.contains(entry.requirement_id.as_str()))
            .collect::<Vec<_>>();
        let matching_exemptions = matrix
            .exemptions
            .iter()
            .filter(|exemption| requirement_set.contains(exemption.requirement_id.as_str()))
            .collect::<Vec<_>>();
        let status = if has_valid_pass {
            "bounded-observed"
        } else if has_failed {
            "failed"
        } else if has_inconclusive {
            "inconclusive"
        } else if matching_entries.is_empty() && matching_exemptions.is_empty() {
            "unsupported"
        } else if matching_entries.is_empty() {
            diagnostics.push("only reviewed exemptions cover the listed requirements".to_string());
            "unsupported"
        } else {
            diagnostics.push("matrix coverage exists but no validated receipt was supplied".to_string());
            "not-recorded"
        };
        let coverage_kinds = sorted_unique(matching_entries.iter().map(|entry| entry.coverage_kind.clone()));
        let scopes = sorted_unique(matching_entries.iter().map(|entry| entry.evidence_scope.clone()));
        let targets = sorted_unique(matching_entries.iter().map(|entry| entry.target.clone()));
        capabilities.push(CapabilityReport {
            id: capability.id.clone(),
            requirements: capability.requirements.clone(),
            profiles: capability.profiles.clone(),
            implementation: Implementation {
                matrix_entries: matching_entries.len(),
                coverage_kinds,
                scopes,
                targets,
            },
            execution: Execution { status: status.to_string(), evidence: evidence_reports, diagnostics },
        });
    }
    Ok(Report {
        schema: "molten.capability-report.v1",
        source_revision: input.source_revision.clone(),
        dependency_closure_ref: input.dependency_closure_ref.clone(),
        feature_set: input.feature_set.clone(),
        execution_profile: input.execution_profile.clone(),
        platform: input.platform.clone(),
        capabilities,
        non_claim: NON_CLAIM,
    })
}

fn validate_evidence(evidence: &Evidence) -> Result<(bool, String, Vec<String>), String> {
    let path = Path::new(&evidence.receipt_path);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok((false, "inconclusive".to_string(), vec![format!("missing receipt: {}", evidence.receipt_path)])),
    };
    let actual = format!("blake3:{}", blake3::hash(&bytes).to_hex());
    if actual != evidence.declared_ref {
        return Ok((false, "inconclusive".to_string(), vec![format!(
            "declared ref does not match receipt bytes: {}",
            evidence.receipt_path
        )]));
    }
    let receipt: Receipt = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid receipt JSON: {error}", evidence.receipt_path))?;
    if receipt.schema != RECEIPT_SCHEMA {
        return Ok((false, "inconclusive".to_string(), vec![format!(
            "{}: receipt schema must be {RECEIPT_SCHEMA}",
            evidence.receipt_path
        )]));
    }
    let mut diagnostics = receipt.caveats.clone();
    if receipt.requirement_id != evidence.requirement_id
        || receipt.subject_ref != evidence.subject_ref
        || receipt.scope_ref != evidence.scope_ref
    {
        diagnostics.push(format!("{}: receipt subject, scope, or requirement binding does not match", evidence.receipt_path));
        return Ok((false, "inconclusive".to_string(), diagnostics));
    }
    if receipt.decision != evidence.decision {
        diagnostics.push(format!("{}: receipt decision does not match the report entry", evidence.receipt_path));
        return Ok((false, "inconclusive".to_string(), diagnostics));
    }
    let validated = matches!(receipt.decision.as_str(), "pass" | "fail" | "inconclusive" | "unsupported");
    Ok((validated, receipt.decision, diagnostics))
}

fn sorted_unique(values: impl Iterator<Item = String>) -> Vec<String> {
    values.collect::<BTreeSet<_>>().into_iter().collect()
}

fn render_markdown(report: &Report) -> String {
    let mut lines = vec![
        "# Capability qualification report".to_string(),
        String::new(),
        format!("Source revision: `{}`", report.source_revision),
        format!("Dependency closure: `{}`", report.dependency_closure_ref),
        format!("Execution profile: `{}`", report.execution_profile),
        format!("Platform: `{}`", report.platform),
        format!("Features: `{}`", report.feature_set.join(", ")),
        String::new(),
        "| Capability | Requirements | Implementation | Execution | Evidence |".to_string(),
        "|---|---|---|---|---|".to_string(),
    ];
    for capability in &report.capabilities {
        let implementation = format!(
            "{} matrix entries; scopes: {}; kinds: {}",
            capability.implementation.matrix_entries,
            join_or_none(&capability.implementation.scopes),
            join_or_none(&capability.implementation.coverage_kinds)
        );
        let evidence = if capability.execution.evidence.is_empty() {
            "none".to_string()
        } else {
            capability
                .execution
                .evidence
                .iter()
                .map(|entry| format!("{}:{}", entry.decision, entry.declared_ref))
                .collect::<Vec<_>>()
                .join("<br>")
        };
        lines.push(format!(
            "| `{}` | {} | {} | **{}** | {} |",
            capability.id,
            capability.requirements.join("<br>"),
            implementation,
            capability.execution.status,
            evidence
        ));
    }
    lines.push(String::new());
    lines.push(format!("_{}_", report.non_claim));
    lines.push(String::new());
    lines.join("\n")
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn self_tests() -> Result<(), String> {
    let directory = std::env::temp_dir().join(format!("molten-capability-report-{}", std::process::id()));
    fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let receipt = serde_json::json!({
        "schema": RECEIPT_SCHEMA,
        "requirement_id": "req.one",
        "subject_ref": "blake3:subject",
        "scope_ref": "blake3:scope",
        "decision": "pass",
        "caveats": ["bounded fixture"]
    });
    let receipt_path = directory.join("receipt.json");
    fs::write(&receipt_path, serde_json::to_vec(&receipt).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    let ref_value = format!("blake3:{}", blake3::hash(&fs::read(&receipt_path).unwrap()).to_hex());
    let matrix = Matrix {
        schema: MATRIX_SCHEMA.to_string(),
        entries: vec![MatrixEntry {
            requirement_id: "req.one".to_string(),
            coverage_kind: "positive".to_string(),
            evidence_scope: "unit".to_string(),
            target: "src/lib.rs".to_string(),
            command: "cargo test".to_string(),
            artifact_refs: vec![],
            receipt_ref: ref_value.clone(),
            caveats: vec![],
        }],
        exemptions: vec![],
    };
    let input = Input {
        schema: INPUT_SCHEMA.to_string(),
        source_revision: "rev".to_string(),
        dependency_closure_ref: "blake3:closure".to_string(),
        feature_set: vec!["default".to_string()],
        execution_profile: "unit".to_string(),
        platform: "test".to_string(),
        capabilities: vec![Capability {
            id: "fixture".to_string(),
            requirements: vec!["req.one".to_string()],
            profiles: vec!["unit".to_string()],
            evidence: vec![Evidence {
                requirement_id: "req.one".to_string(),
                receipt_path: receipt_path.display().to_string(),
                declared_ref: ref_value,
                subject_ref: "blake3:subject".to_string(),
                scope_ref: "blake3:scope".to_string(),
                decision: "pass".to_string(),
            }],
        }],
    };
    let report = build(&matrix, &input)?;
    if report.capabilities[0].execution.status != "bounded-observed" {
        return Err("positive fixture did not become bounded-observed".to_string());
    }

    let mut stale = input;
    stale.capabilities[0].evidence[0].declared_ref = "blake3:placeholder".to_string();
    let report = build(&matrix, &stale)?;
    if report.capabilities[0].execution.status != "inconclusive" {
        return Err("placeholder ref did not become inconclusive".to_string());
    }

    let missing = Evidence {
        requirement_id: "req.one".to_string(),
        receipt_path: directory.join("missing.json").display().to_string(),
        declared_ref: "blake3:missing".to_string(),
        subject_ref: "blake3:subject".to_string(),
        scope_ref: "blake3:scope".to_string(),
        decision: "pass".to_string(),
    };
    let mut missing_input = stale;
    missing_input.capabilities[0].evidence[0] = missing;
    let report = build(&matrix, &missing_input)?;
    if report.capabilities[0].execution.status != "inconclusive" {
        return Err("missing receipt did not become inconclusive".to_string());
    }

    fs::remove_dir_all(&directory).map_err(|e| e.to_string())?;
    println!("capability report self-test ok");
    Ok(())
}
