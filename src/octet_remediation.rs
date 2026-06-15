use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

use preserves::IOValue;
use serde::Deserialize;

use crate::error::MoltenError;
use crate::error::Result;
use crate::preserves_rail::OCTET_REMEDIATION_PLAN_SCHEMA;
use crate::preserves_rail::canonical_hash;
use crate::preserves_rail::content_ref_from_bytes;
use crate::preserves_rail::record;
use crate::preserves_rail::sequence;
use crate::preserves_rail::string;
use crate::preserves_rail::u64_value;

const STATUS_NAME: &str = "status.json";
const SUMMARY_NAME: &str = "summary.txt";
const OBJECT_CORPUS_RECEIPT_NAME: &str = "object-corpus-receipt.json";
const MAX_SUMMARY_LINES: usize = 20_000;
const MAX_INDEX_FINDINGS: usize = 50_000;
const MAX_METRIC_ROWS: usize = 128;
const MAX_TOP_FILES: usize = 24;
const MAX_DIAGNOSTICS: usize = 64;

const _: () = assert!(MAX_SUMMARY_LINES > 0);
const _: () = assert!(MAX_INDEX_FINDINGS >= MAX_SUMMARY_LINES);

const CRITICAL_LINTS: &[&str] = &[
    "no_panic",
    "no_unwrap",
    "ambient_clock",
    "unbounded_loop",
    "unbounded_collection_growth",
    "secret_rendering",
    "harness_backdoor",
    "authority_typing",
    "adapter_boundary_gate",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetRemediationPlanInput {
    pub artifacts_dir: PathBuf,
    pub lib_artifacts_dir: Option<PathBuf>,
    pub focused_object_corpus: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetRemediationPlan {
    pub plan_ref: String,
    pub value: IOValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactText {
    name: String,
    content_ref: String,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunArtifacts {
    scope: String,
    status: ArtifactText,
    summary: ArtifactText,
    object_corpus: Option<ArtifactText>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunMetrics {
    scope: String,
    status_ref: String,
    summary_ref: String,
    object_corpus_ref: Option<String>,
    status: OctetStatusArtifact,
    by_crate: Vec<MetricEntry>,
    by_effect: Vec<MetricEntry>,
    by_lint: Vec<MetricEntry>,
    top_project_files: Vec<MetricEntry>,
    findings: Vec<FindingIndexEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectCorpusMetrics {
    content_ref: String,
    object_set_hash: Option<String>,
    object_count: u64,
    pure_cache_blocked_count: u64,
    source_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetricEntry {
    name: String,
    count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FindingIndexEntry {
    lint: String,
    crate_name: String,
    location: String,
    file: String,
    count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SurfaceDefinition {
    name: &'static str,
    files: &'static [&'static str],
    reason: &'static str,
}

struct PlanValueInput<'a> {
    workspace: &'a RunMetrics,
    lib_metrics: Option<&'a RunMetrics>,
    focused_object_corpus: Option<&'a ObjectCorpusMetrics>,
    critical_surfaces: &'a [IOValue],
    diagnostics: &'a [String],
    checks: &'a [(&'static str, &'static str)],
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetStatusArtifact {
    status: String,
    exit_code: i64,
    metadata: OctetMetadata,
    total_findings: u64,
    warning_findings: u64,
    error_findings: u64,
    autofixable_findings: u64,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetMetadata {
    tool_name: String,
    tool_version: String,
    rustc_version: String,
    toolchain: String,
    profile_name: String,
    profile_hash: String,
    config_hash: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
struct OctetObjectCorpusReceipt {
    object_count: Option<u64>,
    source_paths: Option<Vec<String>>,
    object_set_hash: Option<String>,
    pure_cache_blocked_count: Option<u64>,
}

pub fn build_octet_remediation_plan(input: &OctetRemediationPlanInput) -> Result<OctetRemediationPlan> {
    let mut diagnostics = Vec::new();
    let workspace = read_run_artifacts("workspace", &input.artifacts_dir, true)?;
    let workspace_metrics = run_metrics(&workspace, &mut diagnostics)?;
    let lib_metrics = match input.lib_artifacts_dir.as_ref() {
        Some(path) => Some(run_metrics(&read_run_artifacts("lib-only", path, false)?, &mut diagnostics)?),
        None => None,
    };
    let focused_object_corpus = read_focused_object_corpus(input, &workspace, &mut diagnostics)?;
    let critical_surfaces = surface_inventory_values(&workspace_metrics);
    let checks = plan_checks(&workspace_metrics, lib_metrics.as_ref(), focused_object_corpus.as_ref());
    let value = remediation_plan_value(&PlanValueInput {
        workspace: &workspace_metrics,
        lib_metrics: lib_metrics.as_ref(),
        focused_object_corpus: focused_object_corpus.as_ref(),
        critical_surfaces: &critical_surfaces,
        diagnostics: &diagnostics,
        checks: &checks,
    });
    let plan_ref = canonical_hash(&value)?;
    Ok(OctetRemediationPlan {
        plan_ref,
        value,
        diagnostics,
    })
}

fn read_run_artifacts(scope: &str, artifacts_dir: &Path, require_object_corpus: bool) -> Result<RunArtifacts> {
    let status = read_artifact_text(artifacts_dir.join(STATUS_NAME), STATUS_NAME)?;
    let summary = read_artifact_text(artifacts_dir.join(SUMMARY_NAME), SUMMARY_NAME)?;
    let object_path = artifacts_dir.join(OBJECT_CORPUS_RECEIPT_NAME);
    let object_corpus = if object_path.exists() {
        Some(read_artifact_text(object_path, OBJECT_CORPUS_RECEIPT_NAME)?)
    } else if require_object_corpus {
        return Err(MoltenError::invalid_harness(format!(
            "octet remediation requires {} under {}",
            OBJECT_CORPUS_RECEIPT_NAME,
            artifacts_dir.display()
        )));
    } else {
        None
    };
    Ok(RunArtifacts {
        scope: scope.to_string(),
        status,
        summary,
        object_corpus,
    })
}

fn read_artifact_text(path: PathBuf, name: &str) -> Result<ArtifactText> {
    let bytes = fs::read(&path)?;
    let text = String::from_utf8(bytes.clone()).map_err(|error| {
        MoltenError::invalid_harness(format!("octet remediation artifact {name} is not UTF-8: {error}"))
    })?;
    Ok(ArtifactText {
        name: name.to_string(),
        content_ref: bytes_ref(&bytes),
        text,
    })
}

fn run_metrics(artifacts: &RunArtifacts, diagnostics: &mut impl crate::bounded::VecSink<String>) -> Result<RunMetrics> {
    let status: OctetStatusArtifact = serde_json::from_str(&artifacts.status.text)
        .map_err(|error| MoltenError::invalid_harness(format!("malformed {}: {error}", artifacts.status.name)))?;
    let summary = parse_summary_metrics(&artifacts.summary.text)?;
    if summary.parsed_findings != status.total_findings {
        push_diagnostic(
            diagnostics,
            format!(
                "{} summary parsed {} indexed findings but status reports {}",
                artifacts.scope, summary.parsed_findings, status.total_findings
            ),
        );
    }
    Ok(RunMetrics {
        scope: artifacts.scope.clone(),
        status_ref: artifacts.status.content_ref.clone(),
        summary_ref: artifacts.summary.content_ref.clone(),
        object_corpus_ref: artifacts.object_corpus.as_ref().map(|artifact| artifact.content_ref.clone()),
        status,
        by_crate: sorted_metric_entries(summary.by_crate, MAX_METRIC_ROWS),
        by_effect: sorted_metric_entries(summary.by_effect, MAX_METRIC_ROWS),
        by_lint: sorted_metric_entries(summary.by_lint, MAX_METRIC_ROWS),
        top_project_files: sorted_metric_entries(summary.by_file, MAX_TOP_FILES),
        findings: summary.findings,
    })
}

fn read_focused_object_corpus(
    input: &OctetRemediationPlanInput,
    workspace: &RunArtifacts,
    diagnostics: &mut impl crate::bounded::VecSink<String>,
) -> Result<Option<ObjectCorpusMetrics>> {
    let artifact = match input.focused_object_corpus.as_ref() {
        Some(path) => Some(read_artifact_text(path.clone(), OBJECT_CORPUS_RECEIPT_NAME)?),
        None => workspace.object_corpus.clone(),
    };
    let Some(artifact) = artifact else {
        push_diagnostic(diagnostics, "octet remediation has no focused object corpus artifact".to_string());
        return Ok(None);
    };
    let parsed: OctetObjectCorpusReceipt = serde_json::from_str(&artifact.text)
        .map_err(|error| MoltenError::invalid_harness(format!("malformed object corpus receipt: {error}")))?;
    let object_count = parsed
        .object_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing object_count"))?;
    let pure_cache_blocked_count = parsed
        .pure_cache_blocked_count
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing pure_cache_blocked_count"))?;
    let source_paths = parsed
        .source_paths
        .ok_or_else(|| MoltenError::invalid_harness("object corpus receipt missing source_paths"))?;
    Ok(Some(ObjectCorpusMetrics {
        content_ref: artifact.content_ref,
        object_set_hash: parsed.object_set_hash,
        object_count,
        pure_cache_blocked_count,
        source_paths,
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SummaryMetrics {
    by_crate: BTreeMap<String, u64>,
    by_effect: BTreeMap<String, u64>,
    by_lint: BTreeMap<String, u64>,
    by_file: BTreeMap<String, u64>,
    findings: Vec<FindingIndexEntry>,
    parsed_findings: u64,
}

fn parse_summary_metrics(text: &str) -> Result<SummaryMetrics> {
    let by_crate = parse_count_section(text, "By crate:")?;
    let by_effect = parse_count_section(text, "By effect:")?;
    let by_lint = parse_count_section(text, "By lint:")?;
    let findings = parse_index_findings(text)?;
    let parsed_findings = findings.iter().map(|finding| finding.count).sum::<u64>();
    let mut by_file = BTreeMap::new();
    for finding in &findings {
        if finding.file.starts_with("src/") {
            increment_count(&mut by_file, &finding.file, finding.count);
        }
    }
    Ok(SummaryMetrics {
        by_crate,
        by_effect,
        by_lint,
        by_file,
        findings,
        parsed_findings,
    })
}

fn parse_count_section(text: &str, header: &str) -> Result<BTreeMap<String, u64>> {
    let mut values = BTreeMap::new();
    let mut is_parsing_section = false;
    for (line_index, line) in text.lines().enumerate() {
        if line_index >= MAX_SUMMARY_LINES {
            return Err(MoltenError::invalid_harness("octet summary exceeds line bound"));
        }
        let trimmed = line.trim();
        if trimmed == header {
            is_parsing_section = true;
            continue;
        }
        if is_parsing_section && (trimmed.is_empty() || trimmed.ends_with(':')) {
            break;
        }
        if !is_parsing_section {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(count_text) = parts.next() else { continue };
        if let Ok(count) = count_text.parse::<u64>() {
            insert_count_bounded(&mut values, name, count, MAX_METRIC_ROWS, header)?;
        }
    }
    Ok(values)
}

fn parse_index_findings(text: &str) -> Result<Vec<FindingIndexEntry>> {
    let mut by_key: BTreeMap<(String, String, String, String), u64> = BTreeMap::new();
    let mut is_parsing_index = false;
    let mut parsed_rows = 0usize;
    for (line_index, line) in text.lines().enumerate() {
        if line_index >= MAX_SUMMARY_LINES {
            return Err(MoltenError::invalid_harness("octet summary exceeds line bound"));
        }
        let trimmed = line.trim();
        if trimmed == "Index:" {
            is_parsing_index = true;
            continue;
        }
        if !is_parsing_index || trimmed.is_empty() {
            continue;
        }
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        if parts.len() < 4 || !parts[0].starts_with('F') {
            continue;
        }
        parsed_rows = parsed_rows.saturating_add(1);
        if parsed_rows > MAX_INDEX_FINDINGS {
            return Err(MoltenError::invalid_harness("octet finding index exceeds row bound"));
        }
        let lint = parts[1].to_string();
        let crate_name = parts[2].to_string();
        let location = parts[3].to_string();
        let file = location_file(&location);
        let key = (lint, crate_name, location, file);
        *by_key.entry(key).or_insert(0) += 1;
    }
    let mut findings = by_key
        .into_iter()
        .map(|((lint, crate_name, location, file), count)| FindingIndexEntry {
            lint,
            crate_name,
            location,
            file,
            count,
        })
        .collect::<Vec<_>>();
    findings.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then(left.file.cmp(&right.file))
            .then(left.lint.cmp(&right.lint))
            .then(left.location.cmp(&right.location))
    });
    Ok(findings)
}

fn location_file(location: &str) -> String {
    let Some((file, line_text)) = location.rsplit_once(':') else {
        return location.to_string();
    };
    if line_text.chars().all(|ch| ch.is_ascii_digit()) {
        return file.to_string();
    }
    location.to_string()
}

fn surface_inventory_values(metrics: &RunMetrics) -> Vec<IOValue> {
    critical_surface_definitions()
        .iter()
        .map(|surface| surface_inventory_value(surface, metrics))
        .collect()
}

fn surface_inventory_value(surface: &SurfaceDefinition, metrics: &RunMetrics) -> IOValue {
    let mut by_lint = BTreeMap::new();
    let mut total = 0u64;
    for finding in &metrics.findings {
        if surface.files.iter().any(|path| *path == finding.file) {
            total = total.saturating_add(finding.count);
            increment_count(&mut by_lint, &finding.lint, finding.count);
        }
    }
    record("surface", vec![
        record("name", vec![string(surface.name)]),
        record("files", vec![sequence(surface.files.iter().map(string).collect())]),
        record("reason", vec![string(surface.reason)]),
        record("findings", vec![u64_value(total)]),
        record("critical-findings", vec![u64_value(critical_count(&by_lint))]),
        record("by-lint", vec![sequence(metric_values(&sorted_metric_entries(
            by_lint,
            MAX_METRIC_ROWS,
        )))]),
    ])
}

fn critical_surface_definitions() -> Vec<SurfaceDefinition> {
    vec![
        SurfaceDefinition {
            name: "source-gate-and-admission",
            files: &[
                "src/octet_gate.rs",
                "src/node_runtime.rs",
                "src/job_dag.rs",
                "src/upgrades.rs",
            ],
            reason: "strict source-gate, startup, job admission, and upgrade evidence paths must fail closed",
        },
        SurfaceDefinition {
            name: "harness-and-gates",
            files: &["src/harness/gate.rs", "src/harness/schema.rs", "src/harness/runner.rs"],
            reason: "deterministic test evidence and gate receipts are release-blocking",
        },
        SurfaceDefinition {
            name: "node-runtime-startup",
            files: &["src/node_runtime.rs", "src/node_identity.rs", "src/resources.rs"],
            reason: "node startup binds identity, resources, adapters, and source gates before side effects",
        },
        SurfaceDefinition {
            name: "job-execution",
            files: &[
                "src/job_dag.rs",
                "src/artifacts.rs",
                "src/typed_storage.rs",
                "src/eval_cache.rs",
            ],
            reason: "local and remote job execution must preserve artifact and storage evidence",
        },
        SurfaceDefinition {
            name: "ledger-and-evidence",
            files: &["src/ledger.rs", "src/evidence.rs", "src/evidence_chain.rs"],
            reason: "ledger imports and chain receipts are the evidence substrate",
        },
        SurfaceDefinition {
            name: "adapter-boundaries",
            files: &[
                "src/harness/wasm_executor.rs",
                "src/harness/steel_executor.rs",
                "src/effects.rs",
                "src/remote_dataspace.rs",
            ],
            reason: "executor and remote boundaries must deny ambient authority before side effects",
        },
        SurfaceDefinition {
            name: "redaction-and-export",
            files: &[
                "src/catalog.rs",
                "src/catalog_mcp.rs",
                "src/transcripts.rs",
                "src/harness/repro.rs",
            ],
            reason: "rendering and export paths must not leak confidential evidence",
        },
        SurfaceDefinition {
            name: "cli-artifact-output",
            files: &[
                "src/main.rs",
                "src/cli_delivery.rs",
                "src/cli_node.rs",
                "src/cli_job.rs",
                "src/cli_octet.rs",
                "src/cli_plugin.rs",
                "src/cli_protocol.rs",
                "src/cli_provenance.rs",
                "src/cli_repro.rs",
                "src/cli_retention.rs",
                "src/cli_secrets.rs",
            ],
            reason: "the CLI is the imperative shell that materializes canonical artifacts",
        },
    ]
}

fn remediation_plan_value(input: &PlanValueInput<'_>) -> IOValue {
    record("octet-remediation-plan-v1", vec![
        string(OCTET_REMEDIATION_PLAN_SCHEMA),
        record("workspace", vec![run_metrics_value(input.workspace)]),
        record("lib", vec![optional_run_metrics_value(input.lib_metrics)]),
        record("focused-object-corpus", vec![optional_object_corpus_value(input.focused_object_corpus)]),
        record("critical-surfaces", vec![sequence(input.critical_surfaces.to_vec())]),
        record("priority-order", vec![sequence(priority_order_values())]),
        record("no-suppression-policy", vec![no_suppression_policy_value()]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        record("checks", vec![sequence(
            input
                .checks
                .iter()
                .map(|(name, status)| record("check", vec![string(name), string(status)]))
                .collect(),
        )]),
    ])
}

fn run_metrics_value(metrics: &RunMetrics) -> IOValue {
    record("run", vec![
        record("scope", vec![string(&metrics.scope)]),
        record("artifacts", vec![sequence(artifact_ref_values(metrics))]),
        record("status", vec![
            record("state", vec![string(&metrics.status.status)]),
            record("exit-code", vec![string(metrics.status.exit_code.to_string())]),
            record("total", vec![u64_value(metrics.status.total_findings)]),
            record("warnings", vec![u64_value(metrics.status.warning_findings)]),
            record("errors", vec![u64_value(metrics.status.error_findings)]),
            record("autofixable", vec![u64_value(metrics.status.autofixable_findings)]),
        ]),
        record("metadata", vec![
            record("tool", vec![string(&metrics.status.metadata.tool_name)]),
            record("tool-version", vec![string(&metrics.status.metadata.tool_version)]),
            record("rustc", vec![string(&metrics.status.metadata.rustc_version)]),
            record("toolchain", vec![string(&metrics.status.metadata.toolchain)]),
            record("profile-name", vec![string(&metrics.status.metadata.profile_name)]),
            record("profile-hash", vec![string(&metrics.status.metadata.profile_hash)]),
            record("config-hash", vec![string(&metrics.status.metadata.config_hash)]),
        ]),
        record("by-crate", vec![sequence(metric_values(&metrics.by_crate))]),
        record("by-effect", vec![sequence(metric_values(&metrics.by_effect))]),
        record("by-lint", vec![sequence(metric_values(&metrics.by_lint))]),
        record("top-project-files", vec![sequence(metric_values(&metrics.top_project_files))]),
    ])
}

fn optional_run_metrics_value(metrics: Option<&RunMetrics>) -> IOValue {
    match metrics {
        Some(metrics) => record("some", vec![run_metrics_value(metrics)]),
        None => record("none", Vec::new()),
    }
}

fn artifact_ref_values(metrics: &RunMetrics) -> Vec<IOValue> {
    let mut artifacts = vec![
        artifact_ref_value(STATUS_NAME, &metrics.status_ref),
        artifact_ref_value(SUMMARY_NAME, &metrics.summary_ref),
    ];
    if let Some(object_corpus_ref) = metrics.object_corpus_ref.as_ref() {
        artifacts.push(artifact_ref_value(OBJECT_CORPUS_RECEIPT_NAME, object_corpus_ref));
    }
    artifacts
}

fn artifact_ref_value(name: &str, content_ref: &str) -> IOValue {
    record("artifact", vec![
        record("name", vec![string(name)]),
        record("content-ref", vec![string(content_ref)]),
    ])
}

fn optional_object_corpus_value(metrics: Option<&ObjectCorpusMetrics>) -> IOValue {
    match metrics {
        Some(metrics) => record("some", vec![record("object-corpus", vec![
            record("content-ref", vec![string(&metrics.content_ref)]),
            record("object-set-hash", vec![optional_string_value(metrics.object_set_hash.as_deref())]),
            record("object-count", vec![u64_value(metrics.object_count)]),
            record("pure-cache-blocked", vec![u64_value(metrics.pure_cache_blocked_count)]),
            record("source-paths", vec![sequence(metrics.source_paths.iter().map(string).collect())]),
        ])]),
        None => record("none", Vec::new()),
    }
}

fn optional_string_value(value: Option<&str>) -> IOValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn priority_order_values() -> Vec<IOValue> {
    vec![
        priority_value(
            1,
            "critical-deny-classes",
            "remove or review panic, unwrap, ambient time, unbounded loops, and source-gate sentinels before other warning classes",
        ),
        priority_value(
            2,
            "resource-bounds",
            "add explicit bounds for collection growth, queues, reports, and execution loops on evidence paths",
        ),
        priority_value(
            3,
            "api-shape",
            "replace high-arity builders and raw refs with input structs and typed validation helpers",
        ),
        priority_value(
            4,
            "module-splits",
            "split main, job_dag, and node_runtime shells without changing canonical refs",
        ),
        priority_value(
            5,
            "style-autofix",
            "drain import/path/bool naming findings after safety and evidence blockers are under control",
        ),
    ]
}

fn priority_value(rank: u64, name: &str, rationale: &str) -> IOValue {
    record("priority", vec![
        record("rank", vec![u64_value(rank)]),
        record("name", vec![string(name)]),
        record("rationale", vec![string(rationale)]),
    ])
}

fn no_suppression_policy_value() -> IOValue {
    record("policy", vec![
        record("hidden-suppressions", vec![string("deny")]),
        record("retained-warning-requires", vec![string("scheduled-remediation-or-reviewed-quarantine-receipt")]),
        record("strict-gate-warning-only", vec![string("deny")]),
        record("quarantine", vec![string("explicit-expiring-reviewed-critical-findings-only")]),
    ])
}

fn plan_checks(
    workspace: &RunMetrics,
    lib_metrics: Option<&RunMetrics>,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
) -> Vec<(&'static str, &'static str)> {
    vec![
        ("workspace-status-bound", pass_fail(!workspace.status_ref.is_empty())),
        ("workspace-summary-bound", pass_fail(!workspace.summary_ref.is_empty())),
        ("workspace-metrics-captured", pass_fail(metrics_present_or_clean(workspace))),
        ("lib-metrics-captured", pass_fail(lib_metrics.is_some_and(metrics_present_or_clean))),
        ("focused-object-corpus-bound", pass_fail(focused_object_corpus.is_some())),
        ("critical-surface-inventory", "pass"),
        ("priority-order-defined", "pass"),
        ("no-suppression-policy", "pass"),
    ]
}

fn pass_fail(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

fn metrics_present_or_clean(metrics: &RunMetrics) -> bool {
    metrics.status.total_findings == 0 || !metrics.by_lint.is_empty()
}

fn metric_values(entries: &[MetricEntry]) -> Vec<IOValue> {
    entries
        .iter()
        .map(|entry| record("metric", vec![string(&entry.name), u64_value(entry.count)]))
        .collect()
}

fn sorted_metric_entries(counts: BTreeMap<String, u64>, limit: usize) -> Vec<MetricEntry> {
    let mut entries = counts.into_iter().map(|(name, count)| MetricEntry { name, count }).collect::<Vec<_>>();
    entries.sort_by(|left, right| right.count.cmp(&left.count).then(left.name.cmp(&right.name)));
    entries.truncate(limit);
    entries
}

fn insert_count_bounded(
    counts: &mut BTreeMap<String, u64>,
    name: &str,
    count: u64,
    limit: usize,
    label: &str,
) -> Result<()> {
    if !counts.contains_key(name) && counts.len() >= limit {
        return Err(MoltenError::invalid_harness(format!("{label} count section exceeds row bound")));
    }
    counts.insert(name.to_string(), count);
    Ok(())
}

fn increment_count(counts: &mut BTreeMap<String, u64>, name: &str, count: u64) {
    let current = counts.entry(name.to_string()).or_insert(0);
    *current = current.saturating_add(count);
}

fn critical_count(by_lint: &BTreeMap<String, u64>) -> u64 {
    CRITICAL_LINTS.iter().filter_map(|lint| by_lint.get(*lint)).copied().sum()
}

fn push_diagnostic(diagnostics: &mut impl crate::bounded::VecSink<String>, diagnostic: String) {
    if diagnostics.item_count() < MAX_DIAGNOSTICS {
        diagnostics.push_item(diagnostic);
    }
}

fn bytes_ref(bytes: &[u8]) -> String {
    content_ref_from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger;
    use crate::preserves_rail::to_text;

    #[test]
    fn remediation_plan_captures_workspace_and_lib_metrics() {
        let workspace = temp_dir("workspace-lib-metrics-workspace");
        let lib = temp_dir("workspace-lib-metrics-lib");
        write_artifacts(&workspace, status_json(2), summary_text(), object_corpus_json()).expect("write workspace");
        write_artifacts(&lib, status_json(2), summary_text(), object_corpus_json()).expect("write lib");

        let plan = build_octet_remediation_plan(&OctetRemediationPlanInput {
            artifacts_dir: workspace,
            lib_artifacts_dir: Some(lib),
            focused_object_corpus: None,
        })
        .expect("build remediation plan");

        let text = to_text(&plan.value).expect("render plan");
        crate::preserves_rail::validate_content_ref(&plan.plan_ref).expect("plan ref is canonical");
        assert!(text.contains("octet-remediation-plan-v1"));
        assert!(text.contains("source-gate-and-admission"));
        assert!(text.contains("critical-deny-classes"));
        assert_eq!(ledger::artifact_kind(&plan.value), "octet-remediation-plan");
    }

    #[test]
    fn remediation_plan_reports_status_index_mismatch() {
        let workspace = temp_dir("status-index-mismatch-workspace");
        write_artifacts(&workspace, status_json(3), summary_text(), object_corpus_json()).expect("write workspace");

        let plan = build_octet_remediation_plan(&OctetRemediationPlanInput {
            artifacts_dir: workspace,
            lib_artifacts_dir: None,
            focused_object_corpus: None,
        })
        .expect("build remediation plan");

        assert!(plan.diagnostics.iter().any(|diagnostic| diagnostic.contains("status reports 3")));
        assert!(to_text(&plan.value).expect("render plan").contains("focused-object-corpus-bound"));
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        let path = std::env::temp_dir().join(format!("molten-octet-remediation-{name}-{}", std::process::id()));
        let remove_error = match fs::remove_dir_all(&path) {
            Ok(()) => None,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => Some(error.to_string()),
        };
        assert!(remove_error.is_none(), "remove temp dir before test: {remove_error:?}");
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn write_artifacts(dir: &Path, status: String, summary: &'static str, object_corpus: &'static str) -> Result<()> {
        fs::write(dir.join(STATUS_NAME), status)?;
        fs::write(dir.join(SUMMARY_NAME), summary)?;
        fs::write(dir.join(OBJECT_CORPUS_RECEIPT_NAME), object_corpus)?;
        Ok(())
    }

    fn status_json(total_findings: u64) -> String {
        format!(
            r#"{{"status":"warning-only","exit_code":0,"output_format":"human","metadata":{{"tool_name":"cargo-octet","tool_version":"0.1.0","rustc_version":"rustc test","toolchain":"nightly-test","profile_name":"workspace-metadata","profile_hash":"b3:profile","config_hash":"b3:config"}},"total_findings":{total_findings},"warning_findings":{total_findings},"error_findings":0,"autofixable_findings":0}}"#
        )
    }

    fn summary_text() -> &'static str {
        r#"--- octet summary ---
Status: warning-only
Findings: 2
Warnings: 2
Errors: 0
Autofixable: 0

By crate:
  molten             2

By effect:
  failure            1
  resource           1

By lint:
  no_unwrap          1
  unbounded_collection_growth 1

Index:
  F1     no_unwrap                     molten  src/node_runtime.rs:230
  F2     unbounded_collection_growth   molten  src/job_dag.rs:1753
"#
    }

    fn object_corpus_json() -> &'static str {
        r#"{"object_count":2,"source_paths":["src/job_dag.rs","src/main.rs","src/node_runtime.rs"],"object_set_hash":"b3:objects","pure_cache_blocked_count":2}"#
    }
}
