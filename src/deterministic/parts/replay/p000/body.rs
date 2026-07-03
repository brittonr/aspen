type OrderedMap<K, V> = std::collections::BTreeMap<K, V>;
type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type Path = std::path::Path;
type PreservesValue<T> = preserves::Value<T>;
type Result<T> = crate::error::Result<T>;

const DEFAULT_FIXED_V1_CHUNK_SIZE: u64 = crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
const DETERMINISTIC_CHAOS_SCHEDULE_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_CHAOS_SCHEDULE_SCHEMA;
const DETERMINISTIC_EFFECT_LOG_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_EFFECT_LOG_SCHEMA;
const DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_FIRST_DIVERGENCE_SCHEMA;
const DETERMINISTIC_FIXTURE_RECORD_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_FIXTURE_RECORD_SCHEMA;
const DETERMINISTIC_INTEGRATION_GATE_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_INTEGRATION_GATE_SCHEMA;
const DETERMINISTIC_REPLAY_INDEX_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_REPLAY_INDEX_SCHEMA;
const DETERMINISTIC_REPLAY_ROLLUP_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_REPLAY_ROLLUP_SCHEMA;
const DETERMINISTIC_REPLAY_VERIFY_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_REPLAY_VERIFY_SCHEMA;
const DETERMINISTIC_RUN_IDENTITY_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_RUN_IDENTITY_SCHEMA;
const DETERMINISTIC_TRACE_PRIVACY_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_TRACE_PRIVACY_SCHEMA;
const DETERMINISTIC_TURN_JOURNAL_SCHEMA: &str = crate::preserves_rail::DETERMINISTIC_TURN_JOURNAL_SCHEMA;

const DEFAULT_ARTIFACT_REF: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";
const DEFAULT_CLOSURE_REF: &str = "blake3:2222222222222222222222222222222222222222222222222222222222222222";
const DEFAULT_INITIAL_STATE_REF: &str = "blake3:3333333333333333333333333333333333333333333333333333333333333333";
const DEFAULT_SCHEMA_REF: &str = "blake3:4444444444444444444444444444444444444444444444444444444444444444";
const DEFAULT_POLICY_REF: &str = "blake3:5555555555555555555555555555555555555555555555555555555555555555";
const DEFAULT_CAPABILITY_REF: &str = "blake3:6666666666666666666666666666666666666666666666666666666666666666";
const DEFAULT_REVOCATION_REF: &str = "blake3:7777777777777777777777777777777777777777777777777777777777777777";
const DEFAULT_HANDLER_PROFILE_REF: &str = "blake3:8888888888888888888888888888888888888888888888888888888888888888";
const DEFAULT_SEED_REF: &str = "blake3:9999999999999999999999999999999999999999999999999999999999999999";
const DEFAULT_RUNTIME_REF: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DEFAULT_TOOL_REF: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const MAX_REPLAY_ROLLUP_INPUTS: usize = 1024;
const MAX_REPLAY_INDEX_INPUTS: usize = 4096;
const FIXTURE_RECORD_FIELD_COUNT: usize = 9;
const FIXTURE_SCHEMA_INDEX: usize = 0;
const FIXTURE_IDENTITY_REF_INDEX: usize = 1;
const FIXTURE_IDENTITY_VALUE_INDEX: usize = 2;
const FIXTURE_EFFECT_LOG_REF_INDEX: usize = 3;
const FIXTURE_EFFECT_LOG_VALUE_INDEX: usize = 4;
const FIXTURE_TURN_JOURNALS_INDEX: usize = 5;
const FIXTURE_OUTPUT_REF_INDEX: usize = 6;
const FIXTURE_FINAL_STATE_REF_INDEX: usize = 7;
const TURN_JOURNAL_FIELD_COUNT: usize = 13;
const TURN_JOURNAL_SCHEMA_INDEX: usize = 0;
const TURN_JOURNAL_SCHEDULER_REF_INDEX: usize = 3;
const TURN_JOURNAL_INPUT_REF_INDEX: usize = 4;
const TURN_JOURNAL_EFFECT_REQUEST_REF_INDEX: usize = 6;
const TURN_JOURNAL_EFFECT_RESPONSE_REF_INDEX: usize = 7;
const TURN_JOURNAL_POLICY_DECISION_REF_INDEX: usize = 8;
const TURN_JOURNAL_ACTION_REF_INDEX: usize = 9;
const TURN_JOURNAL_RECEIPT_REF_INDEX: usize = 10;
const TURN_JOURNAL_OUTPUT_REF_INDEX: usize = 11;
const TURN_JOURNAL_AFTER_STATE_REF_INDEX: usize = 12;

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn content_ref_hex(value: &str) -> Result<&str> {
    crate::preserves_rail::content_ref_hex(value)
}

fn put_bytes(
    root: &Path,
    object_kind: &str,
    bytes: &[u8],
    chunk_size: u64,
) -> Result<crate::chunk_store::ChunkStorePut> {
    crate::chunk_store::put_bytes(root, object_kind, bytes, chunk_size)
}

fn range_read(
    root: &Path,
    manifest_ref: &str,
    offset: u64,
    length: u64,
) -> Result<crate::chunk_store::ChunkStoreRangeRead> {
    crate::chunk_store::range_read(root, manifest_ref, offset, length)
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

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

fn value_to_iovalue(value: &PreservesValue<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

#[cfg(test)]
fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
}

#[cfg(test)]
fn read_object(root: &Path, manifest_ref: &str) -> Result<crate::chunk_store::ChunkStoreRead> {
    crate::chunk_store::read_object(root, manifest_ref)
}

#[cfg(test)]
fn to_text(value: &IoValue) -> Result<String> {
    crate::preserves_rail::to_text(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayDivergenceKind {
    None,
    Identity,
    Scheduler,
    Input,
    EffectRequest,
    EffectResponse,
    PolicyDecision,
    Action,
    Receipt,
    Output,
    StateHash,
    LiveEffect,
}

impl ReplayDivergenceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ReplayDivergenceKind::None => "none",
            ReplayDivergenceKind::Identity => "identity",
            ReplayDivergenceKind::Scheduler => "scheduler",
            ReplayDivergenceKind::Input => "input",
            ReplayDivergenceKind::EffectRequest => "effect-request",
            ReplayDivergenceKind::EffectResponse => "effect-response",
            ReplayDivergenceKind::PolicyDecision => "policy-decision",
            ReplayDivergenceKind::Action => "action",
            ReplayDivergenceKind::Receipt => "receipt",
            ReplayDivergenceKind::Output => "output",
            ReplayDivergenceKind::StateHash => "state-hash",
            ReplayDivergenceKind::LiveEffect => "live-effect",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayFixtureVariant {
    Baseline,
    ChangedIdentity,
    ChangedScheduler,
    ChangedInput,
    ChangedEffectRequest,
    ChangedEffectResponse,
    ChangedPolicyDecision,
    ChangedAction,
    ChangedReceipt,
    ChangedOutput,
    ChangedStateHash,
    MissingRecordedEffect,
}

#[derive(Clone, Debug)]
pub struct ReplayFixtureRecord {
    pub value: IoValue,
    pub record_ref: String,
    pub identity_ref: String,
    pub effect_log_ref: String,
    pub final_state_ref: String,
    pub output_ref: String,
}

#[derive(Clone, Debug)]
pub struct ReplaySnapshotManifestBundle {
    pub value: IoValue,
    pub bundle_ref: String,
    pub effect_log_manifest_ref: String,
    pub turn_journal_manifest_ref: String,
    pub snapshot_manifest_ref: String,
    pub first_divergence_manifest_ref: Option<String>,
    pub debug_range_receipt_ref: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ReplayVerifyReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: &'static str,
    pub divergence: ReplayDivergenceKind,
    pub first_divergence: Option<IoValue>,
}

#[derive(Clone, Debug)]
pub struct ReplayRollupInput {
    pub expected_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Clone, Debug)]
pub struct ReplayRollupReceipt {
    pub value: IoValue,
    pub rollup_ref: String,
    pub decision: String,
    pub total_count: u64,
    pub pass_count: u64,
    pub deny_count: u64,
}

#[derive(Clone, Debug)]
pub struct ReplayIndexInput {
    pub expected_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Clone, Debug)]
pub struct ReplayIndexReceipt {
    pub value: IoValue,
    pub index_ref: String,
    pub decision: String,
    pub total_count: u64,
    pub pass_count: u64,
    pub deny_count: u64,
    pub raw_receipt_count: u64,
    pub rollup_count: u64,
}

#[derive(Clone, Debug)]
pub struct ChaosScheduleInput {
    pub seed_ref: String,
    pub schedule_position: u64,
    pub event_ref: String,
    pub fault_kind: String,
    pub intensity_percent: u64,
}

#[derive(Clone, Debug)]
pub struct ChaosScheduleReceipt {
    pub value: IoValue,
    pub schedule_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
pub struct TracePrivacyInput {
    pub trace_ref: String,
    pub snapshot_ref: String,
    pub requester_ref: String,
    pub policy_ref: String,
    pub has_export_authority: bool,
    pub contains_sensitive_refs: bool,
}

#[derive(Clone, Debug)]
pub struct TracePrivacyReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
pub struct DeterministicIntegrationInput {
    pub integration_kind: String,
    pub handler_profile_ref: String,
    pub effect_log_ref: String,
    pub snapshot_ref: String,
    pub gate_ref: String,
    pub admitted_live_effects: bool,
}

#[derive(Clone, Debug)]
pub struct DeterministicIntegrationReceipt {
    pub value: IoValue,
    pub receipt_ref: String,
    pub decision: String,
}

#[derive(Clone, Debug)]
struct ParsedReplayVerify {
    receipt_ref: String,
    decision: String,
    divergence: String,
    first_divergence_ref: Option<String>,
    report_refs: Vec<String>,
    final_state_refs: Vec<String>,
}

#[derive(Clone, Debug)]
struct ParsedReplayRollup {
    rollup_ref: String,
    decision: String,
    total_count: u64,
    pass_count: u64,
    deny_count: u64,
    receipt_refs: Vec<String>,
    divergence_counts: OrderedMap<String, u64>,
    first_divergence_refs: Vec<String>,
}
