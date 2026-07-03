use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::ReadableTableMetadata;

type BtreeSet<T> = std::collections::BTreeSet<T>;
type Database = redb::Database;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type TableDefinition<K, V> = redb::TableDefinition<'static, K, V>;
type PreservesValue<T> = preserves::Value<T>;

const DEFAULT_FIXED_V1_CHUNK_SIZE: u64 = crate::chunk_store::DEFAULT_FIXED_V1_CHUNK_SIZE;
const EVAL_CACHE_KEY_SCHEMA: &str = crate::preserves_rail::EVAL_CACHE_KEY_SCHEMA;
const EVAL_CACHE_RECEIPT_SCHEMA: &str = crate::preserves_rail::EVAL_CACHE_RECEIPT_SCHEMA;
const EVAL_CACHE_VALUE_SCHEMA: &str = crate::preserves_rail::EVAL_CACHE_VALUE_SCHEMA;

fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    crate::preserves_rail::canonical_bytes(value)
}

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    crate::preserves_rail::parse_canonical_bytes(bytes)
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

pub const INLINE_OUTPUT_LIMIT: usize = 4096;

const MAX_EVAL_CACHE_SCAN_ENTRIES: usize = 100_000;
const CACHE_HIT_VALIDITY_DIAGNOSTIC_CAPACITY: usize = 8;
const _: () = assert!(MAX_EVAL_CACHE_SCAN_ENTRIES > 0);

pub const TIER_PURE: &str = "pure";
pub const TIER_SIMULATED: &str = "simulated";
pub const TIER_POLICY_CURRENT: &str = "policy-current";
pub const TIER_PRODUCTION_TRACE_ONLY: &str = "production-effectful-trace-only";

pub const STATUS_PASS: &str = "pass";
pub const STATUS_DENY: &str = "deny";
pub const STATUS_ERROR: &str = "error";
pub const STATUS_TRACE_ONLY: &str = "trace-only";

const INDEX_FILE: &str = "eval-cache.redb";
const INDEX_KEYS: TableDefinition<&str, &[u8]> = TableDefinition::new("eval_cache_keys_v1");
const INDEX_VALUES: TableDefinition<&str, &[u8]> = TableDefinition::new("eval_cache_values_v1");
const INDEX_OUTPUTS: TableDefinition<&str, &[u8]> = TableDefinition::new("eval_cache_outputs_v1");
const INDEX_TOMBSTONES: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_tombstones_v1");
const INDEX_OPERATION: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_operation_v1");
const INDEX_DEPENDENCY: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_dependency_v1");
const INDEX_POLICY: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_policy_v1");
const INDEX_CAPABILITY: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_capability_v1");
const INDEX_REVOCATION: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_revocation_v1");
const INDEX_EVIDENCE: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_evidence_v1");
const INDEX_STATUS: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_status_v1");
const INDEX_TIER: TableDefinition<&str, &str> = TableDefinition::new("eval_cache_tier_v1");
const INDEX_RECEIPTS: TableDefinition<&str, &[u8]> = TableDefinition::new("eval_cache_receipts_v1");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub operation: String,
    pub version: String,
    pub input_ref: String,
    pub dependency_closure_hash: String,
    pub dependency_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub tool_ref: String,
    pub tool_version: String,
    pub assumption_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub key_ref: String,
    pub operation: String,
    pub version: String,
    pub input_ref: String,
    pub dependency_closure_hash: String,
    pub dependency_refs: Vec<String>,
    pub handler_profile_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub capability_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub tool_ref: String,
    pub tool_version: String,
    pub assumption_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputRef {
    None,
    Inline {
        output_ref: String,
        length: u64,
    },
    ContentRef {
        manifest_ref: String,
        output_ref: String,
        length: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueInput {
    pub tier: String,
    pub status: String,
    pub output: Option<IoValue>,
    pub dependency_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub value_ref: String,
    pub key_ref: String,
    pub tier: String,
    pub status: String,
    pub output: OutputRef,
    pub dependency_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Put {
    pub key: Key,
    pub value: Value,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Get {
    pub key: Key,
    pub value: Value,
    pub output: Option<IoValue>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetInput {
    pub current_policy_refs: Vec<String>,
    pub current_capability_refs: Vec<String>,
    pub current_revocation_refs: Vec<String>,
    pub semantic: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct CacheHitValidityInput<'a> {
    pub key: &'a Key,
    pub value: &'a Value,
    pub current_policy_refs: &'a [String],
    pub current_capability_refs: &'a [String],
    pub current_revocation_refs: &'a [String],
    pub requested_dependency_refs: &'a [String],
    pub expected_output_ref: Option<&'a str>,
    pub semantic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheHitValidityDecision {
    pub decision: String,
    pub diagnostics: Vec<String>,
}

impl Default for GetInput {
    fn default() -> Self {
        Self {
            current_policy_refs: Vec::new(),
            current_capability_refs: Vec::new(),
            current_revocation_refs: Vec::new(),
            semantic: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Receipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub key_ref: Option<String>,
    pub value_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Status {
    pub keys: usize,
    pub values: usize,
    pub tombstones: usize,
    pub receipts: usize,
    pub pure: usize,
    pub simulated: usize,
    pub policy_current: usize,
    pub trace_only_tier: usize,
    pub pass: usize,
    pub deny: usize,
    pub error: usize,
    pub trace_only_status: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListFilter {
    pub operation: Option<String>,
    pub tier: Option<String>,
    pub status: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub evidence_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySummary {
    pub key_ref: String,
    pub operation: String,
    pub tier: String,
    pub status: String,
    pub value_ref: String,
    pub tombstoned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InvalidateInput {
    pub key_ref: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub operation: Option<String>,
    pub reason: String,
    pub retention_evidence: crate::retention::DestructiveEvidence,
    pub apply_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalidation {
    pub decision: String,
    pub invalidated_key_refs: Vec<String>,
    pub retention_receipt_refs: Vec<String>,
    pub execution_gate_refs: Vec<String>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct SchemaCompatibilityKeyInput<'a> {
    pub expected_identity_ref: &'a str,
    pub actual_identity_ref: &'a str,
    pub alias_ref: Option<&'a str>,
    pub migration_ref: Option<&'a str>,
    pub tool_ref: &'a str,
    pub tool_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactClosureKeyInput<'a> {
    pub root_refs: &'a [String],
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub tool_ref: &'a str,
    pub tool_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct TranscriptRunKeyInput<'a> {
    pub transcript_ref: &'a str,
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub handler_profile_ref: &'a str,
    pub harness_ref: &'a str,
    pub harness_version: &'a str,
}
