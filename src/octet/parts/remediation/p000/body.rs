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
pub struct PlanInput {
    pub artifacts_dir: PathBuf,
    pub lib_artifacts_dir: Option<PathBuf>,
    pub focused_object_corpus: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
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
    status: StatusArtifact,
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
struct StatusArtifact {
    status: String,
    exit_code: i64,
    metadata: Metadata,
    total_findings: u64,
    warning_findings: u64,
    error_findings: u64,
    autofixable_findings: u64,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct Metadata {
    tool_name: String,
    tool_version: String,
    rustc_version: String,
    toolchain: String,
    profile_name: String,
    profile_hash: String,
    config_hash: String,
}

#[derive(Debug, serde::Deserialize, Clone, PartialEq, Eq)]
struct ObjectCorpusReceipt {
    object_count: Option<u64>,
    source_paths: Option<Vec<String>>,
    object_set_hash: Option<String>,
    pure_cache_blocked_count: Option<u64>,
}

pub fn build_plan(input: &PlanInput) -> Result<Plan> {
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
    Ok(Plan {
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
