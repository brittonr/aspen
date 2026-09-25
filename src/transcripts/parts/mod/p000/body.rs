type Counter = std::sync::atomic::AtomicU64;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type PathBuf = std::path::PathBuf;
type PreservesRecord<T> = preserves::Record<T>;
type PreservesValue<T> = preserves::Value<T>;
type Result<T> = crate::error::Result<T>;
type Set<T> = std::collections::BTreeSet<T>;

const RELAXED: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;

mod fs {
    pub(super) fn create_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir(path)
    }

    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }
}

const TRANSCRIPT_ARTIFACT_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_ARTIFACT_SCHEMA;
const TRANSCRIPT_RUN_RECEIPT_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_RUN_RECEIPT_SCHEMA;
const TRANSCRIPT_STANZA_OUTCOME_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_STANZA_OUTCOME_SCHEMA;
const TRANSCRIPT_STANZA_SCHEMA: &str = crate::preserves_rail::TRANSCRIPT_STANZA_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_text(source: &str) -> Result<IoValue> {
    crate::preserves_rail::parse_text(source)
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

fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

fn u64_value(value: u64) -> IoValue {
    crate::preserves_rail::u64_value(value)
}

// r[impl molten.runtime_spine.canonical_content_refs.migration]
fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const RUNNER_TOOL_VERSION: &str = "local-transcript-runner-v1";

const MAX_TEMP_STATE_ROOT_ATTEMPTS: u64 = 1024;
const MAX_TRANSCRIPT_SEQUENCE_ITEMS: usize = 4_096;
const TRANSCRIPT_ARTIFACT_LEGACY_FIELD_COUNT: usize = 11;
const TRANSCRIPT_ARTIFACT_FIELD_COUNT: usize = 17;
const TRANSCRIPT_STANZA_FIELD_COUNT: usize = 7;
const TRANSCRIPT_RUN_RECEIPT_LEGACY_FIELD_COUNT: usize = 11;
const TRANSCRIPT_RUN_RECEIPT_FIELD_COUNT: usize = 12;

const _: () = assert!(MAX_TRANSCRIPT_SEQUENCE_ITEMS > 0);
const _: () = assert!(TRANSCRIPT_ARTIFACT_FIELD_COUNT > TRANSCRIPT_ARTIFACT_LEGACY_FIELD_COUNT);
const _: () = assert!(TRANSCRIPT_RUN_RECEIPT_FIELD_COUNT > TRANSCRIPT_RUN_RECEIPT_LEGACY_FIELD_COUNT);

static TEMP_STATE_ROOT_COUNTER: Counter = Counter::new(0);

pub const KIND_MOLTEN_CLI: &str = "molten-cli";
pub const KIND_PRESERVES: &str = "preserves";
pub const KIND_ARTIFACT: &str = "artifact";
pub const KIND_POLICY: &str = "policy";
pub const KIND_EXPECT: &str = "expect";
pub const KIND_COMMENT: &str = "comment";

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const DECISION_ERROR: &str = "error";
const DECISION_SKIP: &str = "skip";
const DECISION_KNOWN_BUG: &str = "known-bug";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptParseInput {
    pub dependency_refs: Vec<String>,
    pub dependency_closure_hash: Option<String>,
    pub artifact_refs: Vec<String>,
    pub schema_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub logical_time: Option<u64>,
    pub expected_refs: Vec<String>,
    pub resolution_refs: Vec<String>,
}

impl TranscriptParseInput {
    /// Parse input with no bound references, seed, or logical time.
    pub fn empty() -> Self {
        Self {
            dependency_refs: Vec::new(),
            dependency_closure_hash: None,
            artifact_refs: Vec::new(),
            schema_refs: Vec::new(),
            handler_profile_ref: None,
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            resource_refs: Vec::new(),
            effect_manifest_refs: Vec::new(),
            revocation_refs: Vec::new(),
            seed_ref: None,
            logical_time: None,
            expected_refs: Vec::new(),
            resolution_refs: Vec::new(),
        }
    }
}

impl Default for TranscriptParseInput {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptArtifact {
    pub transcript_ref: String,
    pub source_ref: String,
    pub stanzas: Vec<TranscriptStanza>,
    pub dependency_closure_hash: String,
    pub dependency_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub schema_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_manifest_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub logical_time: Option<u64>,
    pub expected_refs: Vec<String>,
    pub resolution_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptStanza {
    pub stanza_ref: String,
    pub index: u64,
    pub kind: String,
    pub modifiers: Vec<TranscriptModifier>,
    pub content: String,
    pub content_ref: String,
    pub declared_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptModifier {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRunInput {
    pub mode: TranscriptRunMode,
    pub cache_root: Option<PathBuf>,
    pub save_root: Option<PathBuf>,
}

impl Default for TranscriptRunInput {
    fn default() -> Self {
        Self {
            mode: TranscriptRunMode::Fresh,
            cache_root: None,
            save_root: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptRunMode {
    Fresh,
    Save,
    ForkDenied,
    InPlaceDenied,
}

impl TranscriptRunMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            TranscriptRunMode::Fresh => "fresh",
            TranscriptRunMode::Save => "save",
            TranscriptRunMode::ForkDenied => "fork-denied",
            TranscriptRunMode::InPlaceDenied => "in-place-denied",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "fresh" => Ok(Self::Fresh),
            "save" => Ok(Self::Save),
            "fork" | "fork-denied" => Ok(Self::ForkDenied),
            "in-place" | "in-place-denied" => Ok(Self::InPlaceDenied),
            other => Err(MoltenError::invalid_harness(format!("unsupported transcript run mode {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRun {
    pub transcript_ref: String,
    pub decision: String,
    pub stanza_outcomes: Vec<StanzaOutcome>,
    pub receipt_value: IoValue,
    pub receipt_ref: String,
    pub cache_receipt_value: Option<IoValue>,
    pub state_root: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StanzaOutcome {
    pub outcome_ref: String,
    pub index: u64,
    pub kind: String,
    pub decision: String,
    pub output: Option<IoValue>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptRunReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub transcript_ref: String,
    pub mode: String,
    pub outcome_refs: Vec<String>,
    pub value: IoValue,
}

struct RunReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    transcript: &'a TranscriptArtifact,
    mode: &'a str,
    outcomes: &'a [StanzaOutcome],
    output: Option<&'a IoValue>,
    refs: Vec<String>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StanzaAdmissionRefs {
    schema_refs: Vec<String>,
    policy_refs: Vec<String>,
    capability_refs: Vec<String>,
    effect_manifest_refs: Vec<String>,
    resource_refs: Vec<String>,
}

#[derive(Debug)]
struct RunnerState {
    root: PathBuf,
    registry: PathBuf,
    storage: PathBuf,
    cache: PathBuf,
    last_output: Option<IoValue>,
    last_decision: Option<String>,
    last_kind: Option<String>,
    last_diagnostics: Vec<String>,
    last_artifact_ref: Option<String>,
}
