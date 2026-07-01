type Map<K, V> = std::collections::BTreeMap<K, V>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Result<T> = crate::error::Result<T>;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IoValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

mod fs {
    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    #[cfg(test)]
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    #[cfg(test)]
    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
}

const STATUS_NAME: &str = "status.json";
const SUMMARY_NAME: &str = "summary.txt";
const OBJECT_CORPUS_RECEIPT_NAME: &str = "object-corpus-receipt.json";
const MAX_SUMMARY_LINES: usize = 20_000;
const MAX_INDEX_FINDINGS: usize = 50_000;
const MAX_METRIC_ROWS: usize = 128;
const MAX_TOP_FILES: usize = 24;
const MAX_DIAGNOSTICS: usize = 64;
const MAX_SOURCE_SCOPE_CLASSIFICATIONS: usize = 512;
const MAX_SOURCE_INVENTORY_PATHS: usize = 20_000;
const WORKSPACE_PATH_PREFIX: &str = "<WORKSPACE>/";
const SOURCE_PATH_PREFIX: &str = "src/";
const TEST_PATH_PREFIX: &str = "tests/";
const CARGO_REGISTRY_MARKER: &str = "/.cargo/registry/";
const RUSTC_SOURCE_PREFIX: &str = "/rustc/";
const MODULE_FILE_COUNT_LINT: &str = "module_file_count";
const UNDERSCORE_MODULE_LINT: &str = "underscore_in_module_filename";
const CLASS_MOLTEN_OWNED_SOURCE: &str = "molten-owned-source";
const CLASS_INTEGRATION_TEST_SOURCE: &str = "integration-test-source";
const CLASS_GENERATED_REMAP_SOURCE: &str = "generated-remapped-dependency-source";
const CLASS_REGISTRY_RUSTLIB_SOURCE: &str = "registry-rustlib-source";
const CLASS_UNKNOWN_SOURCE: &str = "unknown";
const DECISION_ACTIONABLE: &str = "actionable";
const DECISION_IGNORE_EXTERNAL: &str = "ignored-as-external";
const DECISION_BLOCKED: &str = "blocked-pending-tooling";

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
    pub value: IoValue,
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
    critical_surfaces: &'a [IoValue],
    source_scope_classifications: &'a [SourceScopeClassification],
    diagnostics: &'a [String],
    checks: &'a [(&'static str, &'static str)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceScopeClassification {
    lint: String,
    crate_name: String,
    location: String,
    file: String,
    count: u64,
    classification: &'static str,
    decision: &'static str,
    evidence: String,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct OctetStatusArtifact {
    status: String,
    exit_code: i64,
    metadata: OctetMetadata,
    total_findings: u64,
    warning_findings: u64,
    error_findings: u64,
    autofixable_findings: u64,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct OctetMetadata {
    tool_name: String,
    tool_version: String,
    rustc_version: String,
    toolchain: String,
    profile_name: String,
    profile_hash: String,
    config_hash: String,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct OctetObjectCorpusReceipt {
    object_count: Option<u64>,
    source_paths: Option<Vec<String>>,
    object_set_hash: Option<String>,
    pure_cache_blocked_count: Option<u64>,
}

pub fn build_octet_remediation_plan(input: &OctetRemediationPlanInput) -> Result<OctetRemediationPlan> {
    let mut diagnostics = Vec::new();
    let workspace = read_run_artifacts("workspace", &input.artifacts_dir, input.focused_object_corpus.is_none())?;
    let workspace_metrics = run_metrics(&workspace, &mut diagnostics)?;
    let lib_metrics = match input.lib_artifacts_dir.as_ref() {
        Some(path) => Some(run_metrics(&read_run_artifacts("lib-only", path, false)?, &mut diagnostics)?),
        None => None,
    };
    let focused_object_corpus = read_focused_object_corpus(input, &workspace, &mut diagnostics)?;
    let critical_surfaces = surface_inventory_values(&workspace_metrics);
    let source_scope_classifications =
        classify_source_scope_findings(&workspace_metrics, focused_object_corpus.as_ref())?;
    let checks = plan_checks(
        &workspace_metrics,
        lib_metrics.as_ref(),
        focused_object_corpus.as_ref(),
        &source_scope_classifications,
    );
    let value = remediation_plan_value(&PlanValueInput {
        workspace: &workspace_metrics,
        lib_metrics: lib_metrics.as_ref(),
        focused_object_corpus: focused_object_corpus.as_ref(),
        critical_surfaces: &critical_surfaces,
        source_scope_classifications: &source_scope_classifications,
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
    by_crate: Map<String, u64>,
    by_effect: Map<String, u64>,
    by_lint: Map<String, u64>,
    by_file: Map<String, u64>,
    findings: Vec<FindingIndexEntry>,
    parsed_findings: u64,
}

fn parse_summary_metrics(text: &str) -> Result<SummaryMetrics> {
    let by_crate = parse_count_section(text, "By crate:")?;
    let by_effect = parse_count_section(text, "By effect:")?;
    let by_lint = parse_count_section(text, "By lint:")?;
    let findings = parse_index_findings(text)?;
    let parsed_findings = findings.iter().map(|finding| finding.count).sum::<u64>();
    let mut by_file = Map::new();
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

fn parse_count_section(text: &str, header: &str) -> Result<Map<String, u64>> {
    let mut values = Map::new();
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
    let mut by_key: Map<(String, String, String, String), u64> = Map::new();
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

fn classify_source_scope_findings(
    metrics: &RunMetrics,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
) -> Result<Vec<SourceScopeClassification>> {
    let source_inventory = source_path_inventory(focused_object_corpus)?;
    let mut classifications = Vec::new();
    for finding in &metrics.findings {
        if !is_source_scope_lint(&finding.lint) {
            continue;
        }
        push_bounded(
            &mut classifications,
            classify_source_scope_finding(finding, &source_inventory),
            MAX_SOURCE_SCOPE_CLASSIFICATIONS,
            "source-scope classifications",
        )?;
    }
    Ok(classifications)
}

fn source_path_inventory(
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
) -> Result<std::collections::BTreeSet<String>> {
    let mut inventory = std::collections::BTreeSet::new();
    if let Some(corpus) = focused_object_corpus {
        for path in &corpus.source_paths {
            insert_source_path_bounded(&mut inventory, path.clone())?;
        }
    }
    Ok(inventory)
}

fn classify_source_scope_finding(
    finding: &FindingIndexEntry,
    source_inventory: &std::collections::BTreeSet<String>,
) -> SourceScopeClassification {
    let normalized_file = workspace_relative_path(&finding.file);
    let (classification, decision, evidence) = source_scope_decision(&finding.file, normalized_file, source_inventory);
    SourceScopeClassification {
        lint: finding.lint.clone(),
        crate_name: finding.crate_name.clone(),
        location: finding.location.clone(),
        file: finding.file.clone(),
        count: finding.count,
        classification,
        decision,
        evidence,
    }
}

fn is_source_scope_lint(lint: &str) -> bool {
    lint == MODULE_FILE_COUNT_LINT || lint == UNDERSCORE_MODULE_LINT
}

fn workspace_relative_path(file: &str) -> Option<&str> {
    file.strip_prefix(WORKSPACE_PATH_PREFIX)
}

fn source_scope_decision(
    file: &str,
    normalized_file: Option<&str>,
    source_inventory: &std::collections::BTreeSet<String>,
) -> (&'static str, &'static str, String) {
    if inventory_contains(source_inventory, file, normalized_file) {
        return (
            CLASS_MOLTEN_OWNED_SOURCE,
            DECISION_ACTIONABLE,
            "reported path is present in the focused Molten source inventory".to_string(),
        );
    }
    if is_integration_test_path(file, normalized_file) {
        return (
            CLASS_INTEGRATION_TEST_SOURCE,
            DECISION_ACTIONABLE,
            "reported path is an integration-test source path".to_string(),
        );
    }
    if is_registry_or_rustlib_path(file) {
        return (
            CLASS_REGISTRY_RUSTLIB_SOURCE,
            DECISION_IGNORE_EXTERNAL,
            "reported path is under a registry or rustlib source root".to_string(),
        );
    }
    if normalized_file.is_some_and(|path| path.starts_with(SOURCE_PATH_PREFIX)) {
        return (
            CLASS_GENERATED_REMAP_SOURCE,
            DECISION_IGNORE_EXTERNAL,
            "reported <WORKSPACE>/src path is absent from the focused Molten source inventory".to_string(),
        );
    }
    (
        CLASS_UNKNOWN_SOURCE,
        DECISION_BLOCKED,
        "reported path is not in the focused inventory and does not match an external source pattern".to_string(),
    )
}

fn inventory_contains(
    source_inventory: &std::collections::BTreeSet<String>,
    file: &str,
    normalized_file: Option<&str>,
) -> bool {
    source_inventory.contains(file) || normalized_file.is_some_and(|path| source_inventory.contains(path))
}

fn is_integration_test_path(file: &str, normalized_file: Option<&str>) -> bool {
    file.starts_with(TEST_PATH_PREFIX) || normalized_file.is_some_and(|path| path.starts_with(TEST_PATH_PREFIX))
}

fn is_registry_or_rustlib_path(file: &str) -> bool {
    file.contains(CARGO_REGISTRY_MARKER) || file.starts_with(RUSTC_SOURCE_PREFIX)
}

fn insert_source_path_bounded(values: &mut std::collections::BTreeSet<String>, value: String) -> Result<()> {
    if !values.contains(&value) && values.len() >= MAX_SOURCE_INVENTORY_PATHS {
        return Err(MoltenError::invalid_harness("source inventory exceeds path bound"));
    }
    values.insert(value);
    Ok(())
}

fn surface_inventory_values(metrics: &RunMetrics) -> Vec<IoValue> {
    critical_surface_definitions()
        .iter()
        .map(|surface| surface_inventory_value(surface, metrics))
        .collect()
}

fn surface_inventory_value(surface: &SurfaceDefinition, metrics: &RunMetrics) -> IoValue {
    let mut by_lint = Map::new();
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

const SURFACES: &[SurfaceDefinition] = &[
    SurfaceDefinition {
        name: "source-gate-and-admission",
        files: &[
            "src/octet/gate.rs",
            "src/node/runtime.rs",
            "src/job/dag.rs",
            "src/upgrades/mod.rs",
        ],
        reason: "strict source-gate, startup, job admission, and upgrade evidence paths must fail closed",
    },
    SurfaceDefinition {
        name: "harness-and-gates",
        files: &[
            "src/harness/gate.rs",
            "src/harness/schema.rs",
            "src/harness/runner.rs",
            "src/nixos/vm.rs",
            "src/nixos/tests.rs",
        ],
        reason: "deterministic test evidence and gate receipts are release-blocking",
    },
    SurfaceDefinition {
        name: "node-runtime-startup",
        files: &["src/node/runtime.rs", "src/node/identity.rs", "src/resources/mod.rs"],
        reason: "node startup binds identity, resources, adapters, and source gates before side effects",
    },
    SurfaceDefinition {
        name: "job-execution",
        files: &[
            "src/job/dag.rs",
            "src/artifacts/mod.rs",
            "src/typed/storage.rs",
            "src/eval/cache.rs",
        ],
        reason: "local and remote job execution must preserve artifact and storage evidence",
    },
    SurfaceDefinition {
        name: "ledger-and-evidence",
        files: &["src/ledger/mod.rs", "src/evidence/mod.rs", "src/evidence/chain.rs"],
        reason: "ledger imports and chain receipts are the evidence substrate",
    },
    SurfaceDefinition {
        name: "adapter-boundaries",
        files: &[
            "src/harness/wasm/executor.rs",
            "src/harness/steel/executor.rs",
            "src/effects/mod.rs",
            "src/remote/dataspace.rs",
            "src/runtime/envelope/mod.rs",
            "src/runtime/envelope/tests.rs",
        ],
        reason: "executor and remote boundaries must deny ambient authority before side effects",
    },
    SurfaceDefinition {
        name: "redaction-and-export",
        files: &[
            "src/catalog/mod.rs",
            "src/catalog/mcp.rs",
            "src/transcripts/mod.rs",
            "src/harness/repro.rs",
        ],
        reason: "rendering and export paths must not leak confidential evidence",
    },
    SurfaceDefinition {
        name: "cli-artifact-output",
        files: &[
            "src/main.rs",
            "src/main/root.rs",
            "src/main/root/command.rs",
            "src/main/tests.rs",
            "src/main/tests/core/basic.rs",
            "src/main/tests/core/cache.rs",
            "src/main/tests/core/chunk.rs",
            "src/main/tests/core/protocol.rs",
            "src/main/tests/core/retention.rs",
            "src/main/tests/core/storage.rs",
            "src/main/tests/job/job.rs",
            "src/main/tests/job/job-admission.rs",
            "src/main/tests/job/job-run.rs",
            "src/main/tests/job/job-setup.rs",
            "src/main/tests/job/job-stale.rs",
            "src/main/tests/job/job-sync.rs",
            "src/main/tests/job/job-worker.rs",
            "src/main/tests/ops/catalog.rs",
            "src/main/tests/ops/helpers.rs",
            "src/main/tests/ops/misc.rs",
            "src/main/tests/ops/provenance.rs",
            "src/main/tests/ops/receipts.rs",
            "src/main/tests/ops/upgrade.rs",
            "src/cli/core/artifact.rs",
            "src/cli/core/artifact/command.rs",
            "src/cli/core/artifact/io.rs",
            "src/cli/core/artifact/ops.rs",
            "src/cli/core/cache.rs",
            "src/cli/core/cache/command.rs",
            "src/cli/core/catalog.rs",
            "src/cli/core/catalog/command.rs",
            "src/cli/core/catalog/filter.rs",
            "src/cli/core/catalog/io.rs",
            "src/cli/core/catalog/ops.rs",
            "src/cli/core/chunk.rs",
            "src/cli/core/chunk/command.rs",
            "src/cli/core/chunk/io.rs",
            "src/cli/core/chunk/ops.rs",
            "src/cli/workflow/coordination.rs",
            "src/cli/workflow/coordination/bounded.rs",
            "src/cli/workflow/coordination/command.rs",
            "src/cli/workflow/coordination/io.rs",
            "src/cli/workflow/coordination/ops.rs",
            "src/cli/workflow/delivery.rs",
            "src/cli/workflow/delivery/command.rs",
            "src/cli/workflow/delivery/io.rs",
            "src/cli/workflow/delivery/ops.rs",
            "src/cli/ops/dogfood.rs",
            "src/cli/ops/dogfood/archive.rs",
            "src/cli/ops/dogfood/command.rs",
            "src/cli/ops/dogfood/signed.rs",
            "src/cli/evidence/gate.rs",
            "src/cli/evidence/gate/command.rs",
            "src/cli/evidence/gate/io.rs",
            "src/cli/evidence/gate/ops.rs",
            "src/cli/evidence/receipts/command.rs",
            "src/cli/evidence/receipts/io.rs",
            "src/cli/evidence/receipts/keyring.rs",
            "src/cli/evidence/receipts/keys.rs",
            "src/cli/evidence/receipts/operator.rs",
            "src/cli/evidence/receipts/signing.rs",
            "src/cli/test/harness.rs",
            "src/cli/ops/node.rs",
            "src/cli/ops/node/authority.rs",
            "src/cli/ops/node/command.rs",
            "src/cli/ops/node/command/authority.rs",
            "src/cli/ops/node/command/base.rs",
            "src/cli/ops/node/command/control.rs",
            "src/cli/ops/node/command/health.rs",
            "src/cli/ops/node/command/live.rs",
            "src/cli/ops/node/control.rs",
            "src/cli/ops/node/control/ingress.rs",
            "src/cli/ops/node/core.rs",
            "src/cli/ops/node/health.rs",
            "src/cli/ops/node/lifecycle.rs",
            "src/cli/ops/node/workflow.rs",
            "src/cli/ops/node/workflow/ack.rs",
            "src/cli/ops/node/workflow/apply.rs",
            "src/cli/ops/node/workflow/bundle.rs",
            "src/cli/ops/node/workflow/gate.rs",
            "src/cli/ops/nixosvm.rs",
            "src/cli/ops/nixosvm/command.rs",
            "src/cli/ops/nixosvm/io.rs",
            "src/cli/ops/nixosvm/ops.rs",
            "src/cli/workflow/job.rs",
            "src/cli/workflow/job/command.rs",
            "src/cli/workflow/job/command/base.rs",
            "src/cli/workflow/job/command/refs.rs",
            "src/cli/workflow/job/command/sync.rs",
            "src/cli/workflow/job/command/worker.rs",
            "src/cli/workflow/job/core.rs",
            "src/cli/workflow/job/io.rs",
            "src/cli/workflow/job/refs.rs",
            "src/cli/workflow/job/schedule.rs",
            "src/cli/workflow/job/schedule/output.rs",
            "src/cli/workflow/job/schedule/phase.rs",
            "src/cli/workflow/job/sync.rs",
            "src/cli/workflow/job/sync/input.rs",
            "src/cli/workflow/job/worker.rs",
            "src/cli/ops/ledger.rs",
            "src/cli/ops/ledger/command.rs",
            "src/cli/ops/ledger/io.rs",
            "src/cli/ops/ledger/ops.rs",
            "src/cli/ops/octet.rs",
            "src/cli/ops/octet/baseline.rs",
            "src/cli/ops/octet/command.rs",
            "src/cli/ops/octet/io.rs",
            "src/cli/ops/octet/ops.rs",
            "src/cli/ops/plugin.rs",
            "src/cli/ops/prodsoak.rs",
            "src/cli/ops/prodsoak/command.rs",
            "src/cli/evidence/receipts.rs",
            "src/cli/runtime/raft.rs",
            "src/cli/runtime/raft/command.rs",
            "src/cli/runtime/raft/io.rs",
            "src/cli/runtime/raft/ops.rs",
            "src/cli/evidence/report.rs",
            "src/cli/evidence/report/command.rs",
            "src/cli/evidence/report/io.rs",
            "src/cli/evidence/report/ops.rs",
            "src/cli/workflow/protocol.rs",
            "src/cli/workflow/protocol/command.rs",
            "src/cli/workflow/protocol/io.rs",
            "src/cli/workflow/protocol/ops.rs",
            "src/cli/workflow/provenance.rs",
            "src/cli/workflow/provenance/command.rs",
            "src/cli/workflow/provenance/input.rs",
            "src/cli/workflow/provenance/io.rs",
            "src/cli/workflow/provenance/ops.rs",
            "src/cli/workflow/remote.rs",
            "src/cli/workflow/remote/command.rs",
            "src/cli/workflow/remote/io.rs",
            "src/cli/workflow/remote/ops.rs",
            "src/cli/test/replayfixture.rs",
            "src/cli/runtime/repro.rs",
            "src/cli/runtime/repro/bundle.rs",
            "src/cli/runtime/repro/command.rs",
            "src/cli/runtime/repro/io.rs",
            "src/cli/workflow/retention.rs",
            "src/cli/workflow/retention/clearance.rs",
            "src/cli/workflow/retention/command.rs",
            "src/cli/workflow/retention/command/base.rs",
            "src/cli/workflow/retention/command/live.rs",
            "src/cli/workflow/retention/command/ops.rs",
            "src/cli/workflow/retention/core.rs",
            "src/cli/workflow/retention/io.rs",
            "src/cli/workflow/retention/ops.rs",
            "src/cli/workflow/retention/send.rs",
            "src/cli/workflow/retention/workflow.rs",
            "src/cli/runtime/rewrite.rs",
            "src/cli/runtime/rewrite/command.rs",
            "src/cli/runtime/rewrite/input.rs",
            "src/cli/runtime/rewrite/io.rs",
            "src/cli/runtime/rewrite/ops.rs",
            "src/cli/core/schema.rs",
            "src/cli/core/schema/command.rs",
            "src/cli/core/schema/io.rs",
            "src/cli/core/schema/ops.rs",
            "src/cli/runtime/secrets.rs",
            "src/cli/runtime/service.rs",
            "src/cli/runtime/service/command.rs",
            "src/cli/runtime/service/io.rs",
            "src/cli/runtime/service/ops.rs",
            "src/cli/core/storage.rs",
            "src/cli/core/storage/command.rs",
            "src/cli/core/storage/io.rs",
            "src/cli/core/storage/ops.rs",
            "src/cli/core/transcript.rs",
            "src/cli/core/transcript/command.rs",
            "src/cli/core/transcript/io.rs",
            "src/cli/core/transcript/ops.rs",
            "src/cli/runtime/upgrade.rs",
            "src/cli/runtime/upgrade/command.rs",
            "src/cli/runtime/upgrade/io.rs",
            "src/cli/runtime/upgrade/ops.rs",
            "src/cli/runtime/vat.rs",
        ],
        reason: "the CLI is the imperative shell that materializes canonical artifacts",
    },
];

fn critical_surface_definitions() -> Vec<SurfaceDefinition> {
    SURFACES.to_vec()
}

fn remediation_plan_value(input: &PlanValueInput<'_>) -> IoValue {
    record("octet-remediation-plan-v1", vec![
        string(crate::preserves_rail::OCTET_REMEDIATION_PLAN_SCHEMA),
        record("workspace", vec![run_metrics_value(input.workspace)]),
        record("lib", vec![optional_run_metrics_value(input.lib_metrics)]),
        record("focused-object-corpus", vec![optional_object_corpus_value(input.focused_object_corpus)]),
        record("critical-surfaces", vec![sequence(input.critical_surfaces.to_vec())]),
        record("source-scope", vec![source_scope_value(input.source_scope_classifications)]),
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

fn run_metrics_value(metrics: &RunMetrics) -> IoValue {
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

fn optional_run_metrics_value(metrics: Option<&RunMetrics>) -> IoValue {
    match metrics {
        Some(metrics) => record("some", vec![run_metrics_value(metrics)]),
        None => record("none", Vec::new()),
    }
}

fn artifact_ref_values(metrics: &RunMetrics) -> Vec<IoValue> {
    let mut artifacts = vec![
        artifact_ref_value(STATUS_NAME, &metrics.status_ref),
        artifact_ref_value(SUMMARY_NAME, &metrics.summary_ref),
    ];
    if let Some(object_corpus_ref) = metrics.object_corpus_ref.as_ref() {
        artifacts.push(artifact_ref_value(OBJECT_CORPUS_RECEIPT_NAME, object_corpus_ref));
    }
    artifacts
}

fn artifact_ref_value(name: &str, content_ref: &str) -> IoValue {
    record("artifact", vec![
        record("name", vec![string(name)]),
        record("content-ref", vec![string(content_ref)]),
    ])
}

fn optional_object_corpus_value(metrics: Option<&ObjectCorpusMetrics>) -> IoValue {
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

fn optional_string_value(value: Option<&str>) -> IoValue {
    match value {
        Some(value) => record("some", vec![string(value)]),
        None => record("none", Vec::new()),
    }
}

fn source_scope_value(classifications: &[SourceScopeClassification]) -> IoValue {
    record("source-scope-classification-v1", vec![record("classifications", vec![sequence(
        classifications.iter().map(source_scope_classification_value).collect(),
    )])])
}

fn source_scope_classification_value(classification: &SourceScopeClassification) -> IoValue {
    record("classification", vec![
        record("lint", vec![string(&classification.lint)]),
        record("crate", vec![string(&classification.crate_name)]),
        record("location", vec![string(&classification.location)]),
        record("file", vec![string(&classification.file)]),
        record("count", vec![u64_value(classification.count)]),
        record("class", vec![string(classification.classification)]),
        record("decision", vec![string(classification.decision)]),
        record("evidence", vec![string(&classification.evidence)]),
    ])
}

fn priority_order_values() -> Vec<IoValue> {
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

fn priority_value(rank: u64, name: &str, rationale: &str) -> IoValue {
    record("priority", vec![
        record("rank", vec![u64_value(rank)]),
        record("name", vec![string(name)]),
        record("rationale", vec![string(rationale)]),
    ])
}

fn no_suppression_policy_value() -> IoValue {
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
    source_scope_classifications: &[SourceScopeClassification],
) -> Vec<(&'static str, &'static str)> {
    vec![
        ("workspace-status-bound", pass_fail(!workspace.status_ref.is_empty())),
        ("workspace-summary-bound", pass_fail(!workspace.summary_ref.is_empty())),
        ("workspace-metrics-captured", pass_fail(metrics_present_or_clean(workspace))),
        ("lib-metrics-captured", pass_fail(lib_metrics.is_some_and(metrics_present_or_clean))),
        ("focused-object-corpus-bound", pass_fail(focused_object_corpus.is_some())),
        (
            "source-scope-classified",
            pass_fail(source_scope_is_classified(workspace, focused_object_corpus, source_scope_classifications)),
        ),
        (
            "unknown-source-scope-blocked",
            pass_fail(unknown_source_scope_is_blocked(source_scope_classifications)),
        ),
        ("critical-surface-inventory", "pass"),
        ("priority-order-defined", "pass"),
        ("no-suppression-policy", "pass"),
    ]
}

fn source_scope_is_classified(
    workspace: &RunMetrics,
    focused_object_corpus: Option<&ObjectCorpusMetrics>,
    source_scope_classifications: &[SourceScopeClassification],
) -> bool {
    let source_scope_finding_count =
        workspace.findings.iter().filter(|finding| is_source_scope_lint(&finding.lint)).count();
    if source_scope_finding_count == 0 {
        return true;
    }
    focused_object_corpus.is_some() && source_scope_finding_count == source_scope_classifications.len()
}

fn unknown_source_scope_is_blocked(source_scope_classifications: &[SourceScopeClassification]) -> bool {
    source_scope_classifications.iter().all(|classification| {
        classification.classification != CLASS_UNKNOWN_SOURCE || classification.decision == DECISION_BLOCKED
    })
}

fn pass_fail(pass: bool) -> &'static str {
    if pass { "pass" } else { "fail" }
}

fn metrics_present_or_clean(metrics: &RunMetrics) -> bool {
    metrics.status.total_findings == 0 || !metrics.by_lint.is_empty()
}

fn metric_values(entries: &[MetricEntry]) -> Vec<IoValue> {
    entries
        .iter()
        .map(|entry| record("metric", vec![string(&entry.name), u64_value(entry.count)]))
        .collect()
}

fn sorted_metric_entries(counts: Map<String, u64>, limit: usize) -> Vec<MetricEntry> {
    let mut entries = counts.into_iter().map(|(name, count)| MetricEntry { name, count }).collect::<Vec<_>>();
    entries.sort_by(|left, right| right.count.cmp(&left.count).then(left.name.cmp(&right.name)));
    entries.truncate(limit);
    entries
}

fn insert_count_bounded(
    counts: &mut Map<String, u64>,
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

fn increment_count(counts: &mut Map<String, u64>, name: &str, count: u64) {
    let current = counts.entry(name.to_string()).or_insert(0);
    *current = current.saturating_add(count);
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, limit: usize, label: &str) -> Result<()> {
    if values.item_count() >= limit {
        return Err(MoltenError::invalid_harness(format!("{label} exceeds item bound")));
    }
    values.push_item(value);
    Ok(())
}

fn critical_count(by_lint: &Map<String, u64>) -> u64 {
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

    fn artifact_kind(value: &IoValue) -> &'static str {
        crate::ledger::artifact_kind(value)
    }

    fn to_text(value: &IoValue) -> Result<String> {
        crate::preserves_rail::to_text(value)
    }

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
        assert_eq!(artifact_kind(&plan.value), "octet-remediation-plan");
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

    #[test]
    fn source_scope_classifies_inventory_path_as_actionable() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(MODULE_FILE_COUNT_LINT, "src/main.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_MOLTEN_OWNED_SOURCE);
        assert_eq!(classification.decision, DECISION_ACTIONABLE);
    }

    #[test]
    fn source_scope_classifies_absent_workspace_source_as_external() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(UNDERSCORE_MODULE_LINT, "<WORKSPACE>/src/address_lookup.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_GENERATED_REMAP_SOURCE);
        assert_eq!(classification.decision, DECISION_IGNORE_EXTERNAL);
    }

    #[test]
    fn source_scope_blocks_unknown_uninventoried_paths() {
        let inventory = source_inventory("src/main.rs");
        let finding = source_finding(MODULE_FILE_COUNT_LINT, "generated/address_lookup.rs:1");

        let classification = classify_source_scope_finding(&finding, &inventory);

        assert_eq!(classification.classification, CLASS_UNKNOWN_SOURCE);
        assert_eq!(classification.decision, DECISION_BLOCKED);
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

    fn source_inventory(path: &str) -> std::collections::BTreeSet<String> {
        std::collections::BTreeSet::from([path.to_string()])
    }

    fn source_finding(lint: &str, location: &str) -> FindingIndexEntry {
        FindingIndexEntry {
            lint: lint.to_string(),
            crate_name: "molten".to_string(),
            location: location.to_string(),
            file: location_file(location),
            count: 1,
        }
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
  F1     no_unwrap                     molten  src/node/runtime.rs:230
  F2     unbounded_collection_growth   molten  src/job/dag.rs:1753
"#
    }

    fn object_corpus_json() -> &'static str {
        r#"{"object_count":2,"source_paths":["src/job/dag.rs","src/main.rs","src/node/runtime.rs"],"object_set_hash":"b3:objects","pure_cache_blocked_count":2}"#
    }
}
