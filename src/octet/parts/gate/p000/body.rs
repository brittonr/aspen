type IoValue = preserves::IOValue;

type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

mod fs {
    #[cfg(test)]
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read(path: impl AsRef<std::path::Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
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

const OCTET_GATE_RECEIPT_SCHEMA: &str = crate::preserves_rail::OCTET_GATE_RECEIPT_SCHEMA;
const OCTET_REVIEW_MANIFEST_SCHEMA: &str = crate::preserves_rail::OCTET_REVIEW_MANIFEST_SCHEMA;
const OCTET_WARNING_BASELINE_SCHEMA: &str = crate::preserves_rail::OCTET_WARNING_BASELINE_SCHEMA;

fn bool_value(value: bool) -> IoValue {
    crate::preserves_rail::bool_value(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_from_bytes(bytes: &[u8]) -> String {
    crate::preserves_rail::content_ref_from_bytes(bytes)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
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

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

const STRICT_PROFILE: &str = "strict-ci";
const QUARANTINE_PROFILE: &str = "quarantine-ci";
const OBJECT_CORPUS_RECEIPT_NAME: &str = "object-corpus-receipt.json";
const COMMAND_NAME: &str = "command.txt";
const STATUS_NAME: &str = "status.json";
const SUMMARY_NAME: &str = "summary.txt";
const OCTET_OBJECT_CORPUS_SCHEMA: &str = "octet.function-object-corpus-receipt.v1";
const MAX_DIAGNOSTICS: usize = 64;
const MAX_OCTET_ARTIFACT_VALUES: usize = 8;
const MAX_OCTET_IMPORTED_REFS: usize = MAX_OCTET_ARTIFACT_VALUES;
const MAX_OCTET_SUMMARY_LINTS: usize = 128;
const MAX_OCTET_COMMAND_TOKENS: usize = 256;
const MAX_OCTET_FINDING_ENTRIES: usize = 100_000;
const MAX_OCTET_STRING_SEQUENCE: usize = 100_000;
const SUPPORTED_OCTET_TOOL_VERSION: &str = "0.1.0";

const _: () = assert!(MAX_OCTET_IMPORTED_REFS <= MAX_OCTET_ARTIFACT_VALUES);
const _: () = assert!(MAX_OCTET_SUMMARY_LINTS > 0);
const _: () = assert!(MAX_OCTET_COMMAND_TOKENS > 0);
const REQUIRED_OBJECT_CORPUS_SOURCE_PATHS: &[&str] = &["src/job/dag.rs", "src/main.rs", "src/node/runtime.rs"];
const DEFAULT_GATE_COMMAND: &str = "cargo octet check --artifact-dir target/octet";
const SOURCE_GATE_CONSUMERS: &[&str] = &[
    "node-startup",
    "job-remote-admission",
    "upgrade-plan",
    "node-control-gate",
];

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
pub struct OctetGateInput {
    pub artifacts_dir: PathBuf,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetGateEvaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetWarningBaselineInput {
    pub artifacts_dir: PathBuf,
    pub created_at: String,
    pub expires_at: String,
    pub target_next: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetWarningBaselineArtifact {
    pub baseline_ref: String,
    pub baseline_value: IoValue,
    pub finding_count: u64,
    pub critical_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetBaselineCheckInput {
    pub artifacts_dir: PathBuf,
    pub baseline_value: IoValue,
    pub profile: String,
    pub as_of: String,
    pub review_values: Vec<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetBaselineEvaluation {
    pub decision: String,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetReviewManifestInput {
    pub profile: String,
    pub expires_at: String,
    pub finding_keys: Vec<String>,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetReviewManifestArtifact {
    pub review_ref: String,
    pub review_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetArtifactLedgerInput {
    pub artifacts_dir: PathBuf,
    pub ledger_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetArtifactLedgerImport {
    pub decision: String,
    pub imported_refs: Vec<String>,
    pub receipt_ref: String,
    pub receipt_value: IoValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetSourceGateValidationInput {
    pub consumer: String,
    pub subject_ref: String,
    pub receipt_value: Option<IoValue>,
    pub source_scope: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OctetSourceGateValidation {
    pub decision: String,
    pub requirement_ref: String,
    pub validation_ref: String,
    pub gate_receipt_ref: Option<String>,
    pub value: IoValue,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedOctetGateReceipt {
    receipt_ref: String,
    decision: String,
    policy_ref: String,
    command_ref: Option<String>,
    status_ref: Option<String>,
    summary_ref: Option<String>,
    findings_ref: Option<String>,
    object_corpus_ref: Option<String>,
    fingerprint_ref: Option<String>,
    config_hash: Option<String>,
    profile_hash: Option<String>,
    toolchain: Option<String>,
    counts: FindingCounts,
    diagnostics: Vec<String>,
    checks: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ValidationRefs {
    counts: FindingCounts,
    receipt_ref: Option<String>,
    policy_ref: Option<String>,
    status_ref: Option<String>,
    summary_ref: Option<String>,
    findings_ref: Option<String>,
    object_corpus_ref: Option<String>,
    fingerprint_ref: Option<String>,
}

struct ReceiptCheckInput<'a> {
    parsed: Option<&'a ParsedOctetGateReceipt>,
    expected: Option<&'a ExpectedMetadata>,
}

struct SourceSetup {
    source_scope: Vec<String>,
    expected: Option<ExpectedMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Check {
    name: &'static str,
    status: &'static str,
}
