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

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const RUNNER_TOOL_VERSION: &str = "local-transcript-runner-v1";

const MAX_TEMP_STATE_ROOT_ATTEMPTS: u64 = 1024;
const MAX_TRANSCRIPT_SEQUENCE_ITEMS: usize = 4_096;

const _: () = assert!(MAX_TRANSCRIPT_SEQUENCE_ITEMS > 0);

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TranscriptParseInput {
    pub dependency_refs: Vec<String>,
    pub dependency_closure_hash: Option<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub expected_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptArtifact {
    pub transcript_ref: String,
    pub source_ref: String,
    pub stanzas: Vec<TranscriptStanza>,
    pub dependency_closure_hash: String,
    pub dependency_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub seed_ref: Option<String>,
    pub expected_refs: Vec<String>,
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
    transcript_ref: &'a str,
    mode: &'a str,
    outcomes: &'a [StanzaOutcome],
    output: Option<&'a IoValue>,
    refs: Vec<String>,
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
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
    last_artifact_ref: Option<String>,
}

pub fn parse_markdown(source: &str, input: &TranscriptParseInput) -> Result<TranscriptArtifact> {
    validate_parse_input(input)?;
    let source_ref = canonical_hash(&string(source))?;
    let stanzas = parse_markdown_stanzas(source)?;
    let stanza_values = stanzas.iter().map(|stanza| stanza.value.clone()).collect::<Vec<_>>();
    let dependency_refs = sorted_unique(&input.dependency_refs);
    let dependency_closure_hash = match input.dependency_closure_hash.as_ref() {
        Some(hash) => hash.clone(),
        None => canonical_hash(&record("transcript-dependency-closure-v1", vec![refs_sequence(&dependency_refs)]))?,
    };
    let value = record("transcript-artifact-v1", vec![
        string(TRANSCRIPT_ARTIFACT_SCHEMA),
        record("source", vec![string(&source_ref)]),
        record("stanzas", vec![sequence(stanza_values)]),
        record("dependencies", vec![string(&dependency_closure_hash), refs_sequence(&dependency_refs)]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref.as_deref())]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        record("revocation", vec![refs_sequence(&sorted_unique(&input.revocation_refs))]),
        record("seed", vec![optional_ref_value(input.seed_ref.as_deref())]),
        record("expected", vec![refs_sequence(&sorted_unique(&input.expected_refs))]),
        checks_value(&[
            "bounded-stanzas",
            "canonical-source-identity",
            "no-ambient-identity",
            "no-ucm-compat",
        ]),
    ]);
    parse_transcript_artifact(&value)
}

pub fn parse_transcript_artifact(value: &IoValue) -> Result<TranscriptArtifact> {
    let fields = value
        .collect_simple_record("transcript-artifact-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <transcript-artifact-v1 ...>"))?;
    require_schema(&fields[0], TRANSCRIPT_ARTIFACT_SCHEMA, "transcript artifact")?;
    let deps = value_to_iovalue(&fields[3]);
    let dep_fields = simple_record(&deps, "dependencies", 2)?;
    let stanzas = record_sequence(&fields[2], "stanzas")?
        .iter()
        .map(|stanza| parse_transcript_stanza(&value_to_iovalue(stanza)))
        .collect::<Result<Vec<_>>>()?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "no-ambient-identity", "transcript artifact")?;
    Ok(TranscriptArtifact {
        transcript_ref: canonical_hash(value)?,
        source_ref: record_ref(&fields[1], "source")?,
        stanzas,
        dependency_closure_hash: required_ref(&dep_fields[0], "dependency closure hash")?,
        dependency_refs: parse_ref_sequence_value(&dep_fields[1], "dependency refs")?,
        handler_profile_ref: record_optional_ref(&fields[4], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[5], "policy")?,
        capability_refs: record_ref_sequence(&fields[6], "capability")?,
        revocation_refs: record_ref_sequence(&fields[7], "revocation")?,
        seed_ref: record_optional_ref(&fields[8], "seed")?,
        expected_refs: record_ref_sequence(&fields[9], "expected")?,
        value: value.clone(),
    })
}
