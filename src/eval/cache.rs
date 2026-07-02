use redb::ReadableDatabase;
use redb::ReadableTable;
use redb::ReadableTableMetadata;

use crate::retention;

type BtreeSet<T> = std::collections::BTreeSet<T>;
type Database = redb::Database;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Record<T> = preserves::Record<T>;
type Result<T> = crate::error::Result<T>;
type TableDefinition<K, V> = redb::TableDefinition<'static, K, V>;
type Value<T> = preserves::Value<T>;

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

fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    crate::preserves_rail::value_to_iovalue(value)
}

pub const INLINE_OUTPUT_LIMIT: usize = 4096;

const MAX_EVAL_CACHE_SCAN_ENTRIES: usize = 100_000;
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
pub struct EvalCacheKeyInput {
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
pub struct EvalCacheKey {
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
pub enum EvalCacheOutputRef {
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
pub struct EvalCacheValueInput {
    pub tier: String,
    pub status: String,
    pub output: Option<IoValue>,
    pub dependency_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCacheValue {
    pub value_ref: String,
    pub key_ref: String,
    pub tier: String,
    pub status: String,
    pub output: EvalCacheOutputRef,
    pub dependency_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCachePut {
    pub key: EvalCacheKey,
    pub value: EvalCacheValue,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCacheGet {
    pub key: EvalCacheKey,
    pub value: EvalCacheValue,
    pub output: Option<IoValue>,
    pub receipt_value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCacheGetInput {
    pub current_policy_refs: Vec<String>,
    pub current_capability_refs: Vec<String>,
    pub current_revocation_refs: Vec<String>,
    pub semantic: bool,
}

impl Default for EvalCacheGetInput {
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
pub struct EvalCacheReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub key_ref: Option<String>,
    pub value_ref: Option<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalCacheStatus {
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
pub struct EvalCacheListFilter {
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
pub struct EvalCacheEntrySummary {
    pub key_ref: String,
    pub operation: String,
    pub tier: String,
    pub status: String,
    pub value_ref: String,
    pub tombstoned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EvalCacheInvalidateInput {
    pub key_ref: Option<String>,
    pub dependency_ref: Option<String>,
    pub policy_ref: Option<String>,
    pub capability_ref: Option<String>,
    pub revocation_ref: Option<String>,
    pub operation: Option<String>,
    pub reason: String,
    pub retention_evidence: retention::DestructiveRetentionEvidence,
    pub apply_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvalCacheInvalidate {
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

#[derive(Debug, Clone, Copy)]
pub struct ChoreographyProjectionKeyInput<'a> {
    pub protocol_artifact_ref: &'a str,
    pub role_ref: &'a str,
    pub closure_hash: &'a str,
    pub dependency_refs: &'a [String],
    pub projector_ref: &'a str,
    pub projector_version: &'a str,
    pub policy_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct EvalCacheReceiptValueInput<'a> {
    operation: &'a str,
    decision: &'a str,
    key_ref: Option<&'a str>,
    value_ref: Option<&'a str>,
    refs: &'a [String],
    diagnostics: &'a [String],
    checks: &'a [(&'a str, &'a str)],
}

pub fn eval_cache_key_value(input: &EvalCacheKeyInput) -> Result<IoValue> {
    validate_key_input(input)?;
    Ok(record("eval-cache-key-v1", vec![
        string(EVAL_CACHE_KEY_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("version", vec![string(&input.version)]),
        record("input", vec![string(&input.input_ref)]),
        record("dependencies", vec![
            string(&input.dependency_closure_hash),
            refs_sequence(&sorted_unique(&input.dependency_refs)),
        ]),
        record("handler-profile", vec![optional_ref_value(input.handler_profile_ref.as_deref())]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("capability", vec![refs_sequence(&sorted_unique(&input.capability_refs))]),
        record("revocation", vec![refs_sequence(&sorted_unique(&input.revocation_refs))]),
        record("tool", vec![string(&input.tool_ref), string(&input.tool_version)]),
        record("assumptions", vec![refs_sequence(&sorted_unique(&input.assumption_refs))]),
        checks_value(&["domain-separated-key", "no-name-key", "determinism-inputs-bound"]),
    ]))
}

pub fn parse_eval_cache_key(value: &IoValue) -> Result<EvalCacheKey> {
    let fields = value
        .collect_simple_record("eval-cache-key-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-key-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_KEY_SCHEMA, "eval cache key")?;
    let deps = value_to_iovalue(&fields[4]);
    let dep_fields = simple_record(&deps, "dependencies", 2)?;
    let tool = value_to_iovalue(&fields[9]);
    let tool_fields = simple_record(&tool, "tool", 2)?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "no-name-key", "eval cache key")?;
    Ok(EvalCacheKey {
        key_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        version: record_string(&fields[2], "version")?,
        input_ref: record_ref(&fields[3], "input")?,
        dependency_closure_hash: required_ref(&dep_fields[0], "dependency closure hash")?,
        dependency_refs: parse_ref_sequence_value(&dep_fields[1], "dependency refs")?,
        handler_profile_ref: record_optional_ref(&fields[5], "handler-profile")?,
        policy_refs: record_ref_sequence(&fields[6], "policy")?,
        capability_refs: record_ref_sequence(&fields[7], "capability")?,
        revocation_refs: record_ref_sequence(&fields[8], "revocation")?,
        tool_ref: required_ref(&tool_fields[0], "tool ref")?,
        tool_version: required_string(&tool_fields[1], "tool version")?,
        assumption_refs: record_ref_sequence(&fields[10], "assumptions")?,
        value: value.clone(),
    })
}

pub fn eval_cache_value_value(
    key_ref: &str,
    input: &EvalCacheValueInput,
    output_ref: &EvalCacheOutputRef,
) -> Result<IoValue> {
    validate_ref(key_ref, "eval cache key ref")?;
    validate_value_input(input)?;
    validate_output_ref(output_ref)?;
    Ok(record("eval-cache-value-v1", vec![
        string(EVAL_CACHE_VALUE_SCHEMA),
        record("key", vec![string(key_ref)]),
        record("tier", vec![string(&input.tier)]),
        record("status", vec![string(&input.status)]),
        output_ref_value(output_ref),
        record("dependencies", vec![refs_sequence(&sorted_unique(&input.dependency_refs))]),
        record("policy", vec![refs_sequence(&sorted_unique(&input.policy_refs))]),
        record("evidence", vec![refs_sequence(&sorted_unique(&input.evidence_refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value(&["determinism-inputs-bound", "output-integrity", "negative-inputs-bound"]),
    ]))
}

pub fn parse_eval_cache_value(value: &IoValue) -> Result<EvalCacheValue> {
    let fields = value
        .collect_simple_record("eval-cache-value-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-value-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_VALUE_SCHEMA, "eval cache value")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "determinism-inputs-bound", "eval cache value")?;
    Ok(EvalCacheValue {
        value_ref: canonical_hash(value)?,
        key_ref: record_ref(&fields[1], "key")?,
        tier: record_string(&fields[2], "tier")?,
        status: record_string(&fields[3], "status")?,
        output: parse_output_ref(&fields[4])?,
        dependency_refs: record_ref_sequence(&fields[5], "dependencies")?,
        policy_refs: record_ref_sequence(&fields[6], "policy")?,
        evidence_refs: record_ref_sequence(&fields[7], "evidence")?,
        diagnostics: record_string_sequence(&fields[8], "diagnostics")?,
        value: value.clone(),
    })
}

pub fn put(root: &Path, key_input: &EvalCacheKeyInput, value_input: &EvalCacheValueInput) -> Result<EvalCachePut> {
    ensure_dirs(root)?;
    let key_value = eval_cache_key_value(key_input)?;
    let key = parse_eval_cache_key(&key_value)?;
    validate_value_against_key(&key, value_input)?;
    let output_bytes = value_input.output.as_ref().map(canonical_bytes).transpose()?;
    let output_ref = match (value_input.output.as_ref(), output_bytes.as_ref()) {
        (None, None) => EvalCacheOutputRef::None,
        (Some(output), Some(bytes)) if bytes.len() <= INLINE_OUTPUT_LIMIT => EvalCacheOutputRef::Inline {
            output_ref: canonical_hash(output)?,
            length: bytes.len() as u64,
        },
        (Some(output), Some(bytes)) => {
            let chunk = crate::chunk_store::put_bytes(
                &chunk_root(root),
                "eval-cache-output",
                bytes,
                DEFAULT_FIXED_V1_CHUNK_SIZE,
            )?;
            EvalCacheOutputRef::ContentRef {
                manifest_ref: chunk.manifest_ref,
                output_ref: canonical_hash(output)?,
                length: bytes.len() as u64,
            }
        }
        _ => {
            return Err(MoltenError::invalid_harness(
                "eval cache output bytes must be present whenever output value is present",
            ));
        }
    };
    let value_value = eval_cache_value_value(&key.key_ref, value_input, &output_ref)?;
    let value = parse_eval_cache_value(&value_value)?;
    let receipt_value = receipt_value(&EvalCacheReceiptValueInput {
        operation: "put",
        decision: "pass",
        key_ref: Some(&key.key_ref),
        value_ref: Some(&value.value_ref),
        refs: &refs_for_key_value(&key, &value),
        diagnostics: &[],
        checks: &[("cache-insert", "pass"), ("determinism-inputs-bound", "pass")],
    })?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_key_value_in_tx(&write_txn, &key, &value, output_bytes.as_deref())?;
    store_receipt_in_tx(&write_txn, &receipt_value)?;
    write_txn.commit().map_err(index_error)?;
    Ok(EvalCachePut {
        key,
        value,
        receipt_value,
    })
}

pub fn get(root: &Path, key_ref: &str, input: &EvalCacheGetInput) -> Result<EvalCacheGet> {
    validate_ref(key_ref, "eval cache key ref")?;
    validate_refs(&input.current_policy_refs, "current policy ref")?;
    validate_refs(&input.current_capability_refs, "current capability ref")?;
    validate_refs(&input.current_revocation_refs, "current revocation ref")?;
    ensure_dirs(root)?;
    if let Some(reason) = tombstone_reason(root, key_ref)? {
        return Err(denied_tombstone(root, key_ref, &reason)?);
    }
    let Some((key, value)) = read_key_value_pair(root, key_ref)? else {
        return Err(denied_missing(root, key_ref)?);
    };
    let refs = refs_for_key_value(&key, &value);
    if value.tier == TIER_PRODUCTION_TRACE_ONLY && input.semantic {
        return Err(denied_trace_only(root, &key.key_ref, &value.value_ref, &refs)?);
    }
    if value.tier == TIER_POLICY_CURRENT && !policy_current_refs_match(&key, input) {
        return Err(denied_stale(root, &key.key_ref, &value.value_ref, &refs)?);
    }
    let output = read_output(root, &key.key_ref, &value)?;
    let receipt_value = hit_receipt(root, &key.key_ref, &value.value_ref, &refs)?;
    Ok(EvalCacheGet {
        key,
        value,
        output,
        receipt_value,
    })
}

fn denied_tombstone(root: &Path, key_ref: &str, reason: &str) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &EvalCacheReceiptValueInput {
        operation: "miss",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: None,
        refs: &[key_ref.to_string()],
        diagnostics: &[format!("cache key tombstoned: {reason}")],
        checks: &[("cache-miss", "pass"), ("tombstone", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache miss: key {key_ref} tombstoned ({})",
        parse_eval_cache_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_missing(root: &Path, key_ref: &str) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &EvalCacheReceiptValueInput {
        operation: "miss",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: None,
        refs: &[key_ref.to_string()],
        diagnostics: &["cache key not found".to_string()],
        checks: &[("cache-miss", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache miss: key {key_ref} not found ({})",
        parse_eval_cache_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_trace_only(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &EvalCacheReceiptValueInput {
        operation: "trace-only",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["production trace-only cache value cannot be returned as semantic output".to_string()],
        checks: &[("trace-only-not-semantic", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache trace-only denial: {}",
        parse_eval_cache_receipt(&receipt)?.receipt_ref
    )))
}

fn denied_stale(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<MoltenError> {
    let receipt = store_and_return_receipt(root, &EvalCacheReceiptValueInput {
        operation: "stale-deny",
        decision: "deny",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &["policy-current refs do not match current request refs".to_string()],
        checks: &[("policy-current-revalidation", "fail"), ("stale-deny", "pass")],
    })?;
    Ok(MoltenError::invalid_harness(format!(
        "eval cache stale policy-current entry denied: {}",
        parse_eval_cache_receipt(&receipt)?.receipt_ref
    )))
}

fn hit_receipt(root: &Path, key_ref: &str, value_ref: &str, refs: &[String]) -> Result<IoValue> {
    store_and_return_receipt(root, &EvalCacheReceiptValueInput {
        operation: "hit",
        decision: "pass",
        key_ref: Some(key_ref),
        value_ref: Some(value_ref),
        refs,
        diagnostics: &[],
        checks: &[("cache-hit", "pass"), ("output-integrity", "pass")],
    })
}

pub fn invalidate(root: &Path, input: &EvalCacheInvalidateInput) -> Result<EvalCacheInvalidate> {
    ensure_dirs(root)?;
    validate_invalidate_input(input)?;
    let selected_key_refs = selected_keys(root, input)?;
    let reason = invalidation_reason(input);
    let requester_ref = retention::destructive_retention_requester_ref(
        &input.retention_evidence,
        "eval-cache-invalidate-missing-requester",
    )?;
    let run = run_retention(root, input, &requester_ref, &selected_key_refs)?;
    let decision = run.decision();
    let invalidated_key_refs = if decision == "pass" {
        selected_key_refs
    } else {
        Vec::new()
    };
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    if decision == "pass" {
        let mut tombstones = write_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
        for key_ref in &invalidated_key_refs {
            tombstones.insert(key_ref.as_str(), reason.as_str()).map_err(index_error)?;
        }
    }
    let refs = invalidate_refs(input, &invalidated_key_refs, &run)?;
    let diagnostics = invalidate_diagnostics(input, decision, &invalidated_key_refs, &run);
    let receipt = invalidate_receipt(decision, &refs, &diagnostics, &run)?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(EvalCacheInvalidate {
        decision: decision.to_string(),
        invalidated_key_refs,
        retention_receipt_refs: run.receipts,
        execution_gate_refs: run.gates,
        receipt_value: receipt,
    })
}

fn selected_keys(root: &Path, input: &EvalCacheInvalidateInput) -> Result<Vec<String>> {
    let mut keys = BtreeSet::new();
    if let Some(key_ref) = input.key_ref.as_ref() {
        keys.insert(key_ref.clone());
    }
    for summary in list(root, &EvalCacheListFilter::default())? {
        if input.operation.as_ref().is_some_and(|operation| operation == &summary.operation) {
            keys.insert(summary.key_ref.clone());
        }
        if has_ref_filter(input) && summary_refs_match(root, input, &summary.key_ref)? {
            keys.insert(summary.key_ref.clone());
        }
    }
    Ok(keys.into_iter().collect())
}

fn has_ref_filter(input: &EvalCacheInvalidateInput) -> bool {
    input.dependency_ref.is_some()
        || input.policy_ref.is_some()
        || input.capability_ref.is_some()
        || input.revocation_ref.is_some()
}

fn summary_refs_match(root: &Path, input: &EvalCacheInvalidateInput, key_ref: &str) -> Result<bool> {
    let Some((key, value)) = read_key_value_pair(root, key_ref)? else {
        return Ok(false);
    };
    Ok(input
        .dependency_ref
        .as_ref()
        .is_some_and(|reference| key.dependency_refs.contains(reference) || value.dependency_refs.contains(reference))
        || input
            .policy_ref
            .as_ref()
            .is_some_and(|reference| key.policy_refs.contains(reference) || value.policy_refs.contains(reference))
        || input.capability_ref.as_ref().is_some_and(|reference| key.capability_refs.contains(reference))
        || input.revocation_ref.as_ref().is_some_and(|reference| key.revocation_refs.contains(reference)))
}

fn invalidation_reason(input: &EvalCacheInvalidateInput) -> String {
    if input.reason.is_empty() {
        "manual-invalidate".to_string()
    } else {
        input.reason.clone()
    }
}

#[derive(Default)]
struct InvalRun {
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipts: Vec<String>,
    gates: Vec<String>,
    denials: Vec<String>,
}

struct InvalStep {
    key_ref: String,
    admission_diagnostics: Vec<String>,
    execution_diagnostics: Vec<String>,
    admission_refs: Vec<String>,
    receipt_ref: String,
    gate_ref: String,
    denied: bool,
}

impl InvalRun {
    fn add(&mut self, step: InvalStep) -> Result<()> {
        for diagnostic in step.admission_diagnostics {
            push_bounded(
                &mut self.admission_diagnostics,
                diagnostic,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention admission diagnostics",
            )?;
        }
        for diagnostic in step.execution_diagnostics {
            push_bounded(
                &mut self.execution_diagnostics,
                diagnostic,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention execution diagnostics",
            )?;
        }
        for reference in step.admission_refs {
            push_bounded(
                &mut self.admission_refs,
                reference,
                MAX_EVAL_CACHE_SCAN_ENTRIES,
                "eval cache retention admission refs",
            )?;
        }
        push_bounded(
            &mut self.receipts,
            step.receipt_ref,
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache retention receipt refs",
        )?;
        push_bounded(
            &mut self.gates,
            step.gate_ref,
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache retention execution gate refs",
        )?;
        if step.denied {
            push_bounded(&mut self.denials, step.key_ref, MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache retention denials")?;
        }
        Ok(())
    }

    fn decision(&self) -> &'static str {
        if self.denials.is_empty() { "pass" } else { "deny" }
    }

    fn has_admission_denial(&self) -> bool {
        !self.admission_diagnostics.is_empty()
    }

    fn has_execution_denial(&self) -> bool {
        !self.execution_diagnostics.is_empty()
    }
}

fn run_retention(
    root: &Path,
    input: &EvalCacheInvalidateInput,
    requester_ref: &str,
    selected_key_refs: &[String],
) -> Result<InvalRun> {
    let mut run = InvalRun::default();
    for key_ref in selected_key_refs {
        run.add(evaluate_invalidate_key(root, input, requester_ref, key_ref)?)?;
    }
    Ok(run)
}

fn evaluate_invalidate_key(
    root: &Path,
    input: &EvalCacheInvalidateInput,
    requester_ref: &str,
    key_ref: &str,
) -> Result<InvalStep> {
    let admission = retention::admit_destructive_retention_evidence(retention::DestructiveRetentionAdmissionInput {
        root,
        evidence: &input.retention_evidence,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: retention::CLASS_EPHEMERAL_CACHE,
        action: retention::ACTION_TOMBSTONE,
    })?;
    let evaluation = retention::evaluate_retention(retention::RetentionEvaluationInput {
        root,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: retention::CLASS_EPHEMERAL_CACHE,
        action: retention::ACTION_TOMBSTONE,
        requester_ref,
        is_reference_index_complete: input.retention_evidence.is_reference_index_complete,
        retained_refs: &input.retention_evidence.retained_refs,
        remote_refs: &input.retention_evidence.remote_refs,
        policy_refs: &input.retention_evidence.policy_refs,
        evidence_refs: &input.retention_evidence.evidence_refs,
        has_delete_authority: admission.has_delete_authority,
        has_remote_gc_clearance: admission.has_remote_gc_clearance,
    })?;
    let apply_ref = matching_apply_ref(ApplyRefMatchInput {
        root,
        apply_refs: &input.apply_refs,
        subsystem: "eval-cache-invalidate",
        action: retention::ACTION_TOMBSTONE,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: retention::CLASS_EPHEMERAL_CACHE,
    });
    let gate = retention::store_retention_gc_execution_gate(retention::RetentionGcExecutionGateInput {
        root,
        subsystem: "eval-cache-invalidate",
        action: retention::ACTION_TOMBSTONE,
        object_ref: key_ref,
        object_kind: "eval-cache-key",
        retention_class: retention::CLASS_EPHEMERAL_CACHE,
        apply_ref,
    })?;
    let is_gate_denied = gate.decision != "pass";
    let is_denied = admission.decision != "pass" || evaluation.receipt.decision != "pass" || is_gate_denied;
    Ok(InvalStep {
        key_ref: key_ref.to_string(),
        admission_diagnostics: admission.diagnostics,
        execution_diagnostics: if is_gate_denied { gate.diagnostics } else { Vec::new() },
        admission_refs: admission.admitted_refs,
        receipt_ref: evaluation.receipt.receipt_ref,
        gate_ref: gate.execution_ref,
        denied: is_denied,
    })
}

struct RefSink {
    refs: Vec<String>,
}

impl RefSink {
    fn new(seed_refs: &[String]) -> Self {
        Self {
            refs: seed_refs.to_vec(),
        }
    }

    fn push(&mut self, reference: &str) -> Result<()> {
        push_bounded(&mut self.refs, reference.to_string(), MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache receipt refs")
    }

    fn push_all(&mut self, references: &[String]) -> Result<()> {
        for reference in references {
            self.push(reference)?;
        }
        Ok(())
    }

    fn finish(self) -> Vec<String> {
        self.refs
    }
}

fn invalidate_refs(
    input: &EvalCacheInvalidateInput,
    invalidated_key_refs: &[String],
    run: &InvalRun,
) -> Result<Vec<String>> {
    let mut sink = RefSink::new(invalidated_key_refs);
    if let Some(requester_ref) = input.retention_evidence.requester_ref.as_ref() {
        sink.push(requester_ref)?;
    }
    sink.push_all(&input.retention_evidence.policy_refs)?;
    sink.push_all(&input.retention_evidence.authority_refs)?;
    sink.push_all(&input.retention_evidence.evidence_refs)?;
    sink.push_all(&input.retention_evidence.retained_refs)?;
    sink.push_all(&input.retention_evidence.remote_peer_refs)?;
    sink.push_all(&input.retention_evidence.remote_refs)?;
    sink.push_all(&input.retention_evidence.reference_index_refs)?;
    sink.push_all(&input.retention_evidence.remote_gc_refs)?;
    sink.push_all(&input.retention_evidence.remote_clearance_refs)?;
    sink.push_all(&run.admission_refs)?;
    sink.push_all(&run.receipts)?;
    sink.push_all(&run.gates)?;
    Ok(sink.finish())
}

fn invalidate_diagnostics(
    input: &EvalCacheInvalidateInput,
    decision: &str,
    invalidated_key_refs: &[String],
    run: &InvalRun,
) -> Vec<String> {
    let mut diagnostics = if decision == "pass" {
        vec![format!("invalidated {} keys", invalidated_key_refs.len())]
    } else {
        vec![format!("retention denied {} keys", run.denials.len())]
    };
    diagnostics.push(format!(
        "retention evidence requester={} policy={} authority={} evidence={} retained={} remote_peers={} remote={} reference_index={} remote_gc={} remote_clearance={} index_complete={}",
        input.retention_evidence.requester_ref.is_some(),
        input.retention_evidence.policy_refs.len(),
        input.retention_evidence.authority_refs.len(),
        input.retention_evidence.evidence_refs.len(),
        input.retention_evidence.retained_refs.len(),
        input.retention_evidence.remote_peer_refs.len(),
        input.retention_evidence.remote_refs.len(),
        input.retention_evidence.reference_index_refs.len(),
        input.retention_evidence.remote_gc_refs.len(),
        input.retention_evidence.remote_clearance_refs.len(),
        input.retention_evidence.is_reference_index_complete
    ));
    diagnostics.extend(run.admission_diagnostics.iter().cloned());
    diagnostics.extend(run.execution_diagnostics.iter().cloned());
    diagnostics
}

fn invalidate_receipt(decision: &str, refs: &[String], diagnostics: &[String], run: &InvalRun) -> Result<IoValue> {
    receipt_value(&EvalCacheReceiptValueInput {
        operation: "invalidate",
        decision,
        key_ref: None,
        value_ref: None,
        refs,
        diagnostics,
        checks: &[
            ("cache-invalidation", if decision == "pass" { "pass" } else { "fail" }),
            ("tombstone", if decision == "pass" { "pass" } else { "fail" }),
            ("retention-receipt-bound", "pass"),
            ("retention-execution-gate", if run.has_execution_denial() { "fail" } else { "pass" }),
            ("retention-authority-evidence", if run.has_admission_denial() { "fail" } else { "pass" }),
            ("deny-before-tombstone", if decision == "pass" { "pass" } else { "fail" }),
        ],
    })
}

pub fn status(root: &Path) -> Result<EvalCacheStatus> {
    ensure_dirs(root)?;
    let mut status = EvalCacheStatus::default();
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    status.keys = checked_table_len(
        read_txn.open_table(INDEX_KEYS).map_err(index_error)?.len().map_err(index_error)?,
        "cache keys",
    )?;
    status.values = checked_table_len(
        read_txn.open_table(INDEX_VALUES).map_err(index_error)?.len().map_err(index_error)?,
        "cache values",
    )?;
    status.tombstones = checked_table_len(
        read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?.len().map_err(index_error)?,
        "cache tombstones",
    )?;
    status.receipts = checked_table_len(
        read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?.len().map_err(index_error)?,
        "cache receipts",
    )?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    for item in values.iter().map_err(index_error)? {
        let (_key, bytes) = item.map_err(index_error)?;
        let value = parse_eval_cache_value(&parse_canonical_bytes(bytes.value())?)?;
        match value.tier.as_str() {
            TIER_PURE => status.pure += 1,
            TIER_SIMULATED => status.simulated += 1,
            TIER_POLICY_CURRENT => status.policy_current += 1,
            TIER_PRODUCTION_TRACE_ONLY => status.trace_only_tier += 1,
            _ => {}
        }
        match value.status.as_str() {
            STATUS_PASS => status.pass += 1,
            STATUS_DENY => status.deny += 1,
            STATUS_ERROR => status.error += 1,
            STATUS_TRACE_ONLY => status.trace_only_status += 1,
            _ => {}
        }
    }
    Ok(status)
}

pub fn list(root: &Path, filter: &EvalCacheListFilter) -> Result<Vec<EvalCacheEntrySummary>> {
    ensure_dirs(root)?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let tombstones = read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
    let mut entries = Vec::new();
    for item in values.iter().map_err(index_error)? {
        let (key_ref, bytes) = item.map_err(index_error)?;
        let key_ref = key_ref.value().to_string();
        let value = parse_eval_cache_value(&parse_canonical_bytes(bytes.value())?)?;
        let Some(key_bytes) = keys.get(key_ref.as_str()).map_err(index_error)? else {
            continue;
        };
        let key = parse_eval_cache_key(&parse_canonical_bytes(key_bytes.value())?)?;
        if filter.operation.as_ref().is_some_and(|operation| operation != &key.operation)
            || filter.tier.as_ref().is_some_and(|tier| tier != &value.tier)
            || filter.status.as_ref().is_some_and(|status| status != &value.status)
            || filter.dependency_ref.as_ref().is_some_and(|reference| {
                !key.dependency_refs.contains(reference) && !value.dependency_refs.contains(reference)
            })
            || filter
                .policy_ref
                .as_ref()
                .is_some_and(|reference| !key.policy_refs.contains(reference) && !value.policy_refs.contains(reference))
            || filter.capability_ref.as_ref().is_some_and(|reference| !key.capability_refs.contains(reference))
            || filter.revocation_ref.as_ref().is_some_and(|reference| !key.revocation_refs.contains(reference))
            || filter.evidence_ref.as_ref().is_some_and(|reference| !value.evidence_refs.contains(reference))
        {
            continue;
        }
        push_bounded(
            &mut entries,
            EvalCacheEntrySummary {
                key_ref: key_ref.clone(),
                operation: key.operation,
                tier: value.tier,
                status: value.status,
                value_ref: value.value_ref,
                tombstoned: tombstones.get(key_ref.as_str()).map_err(index_error)?.is_some(),
            },
            MAX_EVAL_CACHE_SCAN_ENTRIES,
            "eval cache entries",
        )?;
    }
    entries.sort_by(|left, right| left.key_ref.cmp(&right.key_ref));
    Ok(entries)
}

pub fn read_key(root: &Path, key_ref: &str) -> Result<EvalCacheKey> {
    validate_ref(key_ref, "eval cache key ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let Some(bytes) = keys.get(key_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache key {key_ref} not found")));
    };
    parse_eval_cache_key(&parse_canonical_bytes(bytes.value())?)
}

pub fn read_value(root: &Path, key_ref: &str) -> Result<EvalCacheValue> {
    validate_ref(key_ref, "eval cache key ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let Some(bytes) = values.get(key_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache value for key {key_ref} not found")));
    };
    parse_eval_cache_value(&parse_canonical_bytes(bytes.value())?)
}

pub fn read_receipt(root: &Path, receipt_ref: &str) -> Result<EvalCacheReceipt> {
    validate_ref(receipt_ref, "eval cache receipt ref")?;
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let receipts = read_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    let Some(bytes) = receipts.get(receipt_ref).map_err(index_error)? else {
        return Err(MoltenError::invalid_harness(format!("eval cache receipt {receipt_ref} not found")));
    };
    parse_eval_cache_receipt(&parse_canonical_bytes(bytes.value())?)
}

pub fn rebuild_index(root: &Path) -> Result<IoValue> {
    ensure_dirs(root)?;
    let keys_values = {
        let db = ensure_index_tables(root)?;
        let read_txn = db.begin_read().map_err(index_error)?;
        let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        let mut pairs = Vec::new();
        for item in keys.iter().map_err(index_error)? {
            let (key_ref, key_bytes) = item.map_err(index_error)?;
            if let Some(value_bytes) = values.get(key_ref.value()).map_err(index_error)? {
                let key = parse_eval_cache_key(&parse_canonical_bytes(key_bytes.value())?)?;
                let value = parse_eval_cache_value(&parse_canonical_bytes(value_bytes.value())?)?;
                push_bounded(&mut pairs, (key, value), MAX_EVAL_CACHE_SCAN_ENTRIES, "eval cache index pairs")?;
            }
        }
        pairs
    };
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    clear_derived_index_tables_in_tx(&write_txn)?;
    for (key, value) in &keys_values {
        store_derived_indexes_in_tx(&write_txn, key, value)?;
    }
    let refs = keys_values.iter().map(|(key, _value)| key.key_ref.clone()).collect::<Vec<_>>();
    let receipt = receipt_value(&EvalCacheReceiptValueInput {
        operation: "index-rebuild",
        decision: "pass",
        key_ref: None,
        value_ref: None,
        refs: &refs,
        diagnostics: &[format!("rebuilt {} cache entries", keys_values.len())],
        checks: &[("redb-index-rebuild", "pass"), ("derived-index-ready", "pass")],
    })?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(receipt)
}

pub fn schema_fingerprint_key_input(
    normalized_shape_ref: &str,
    tool_ref: &str,
    tool_version: &str,
    policy_refs: &[String],
) -> Result<EvalCacheKeyInput> {
    validate_ref(normalized_shape_ref, "schema fingerprint shape ref")?;
    Ok(EvalCacheKeyInput {
        operation: "schema-fingerprint".to_string(),
        version: "v1".to_string(),
        input_ref: normalized_shape_ref.to_string(),
        dependency_closure_hash: canonical_hash(&record("eval-cache-empty-closure", Vec::new()))?,
        dependency_refs: Vec::new(),
        handler_profile_ref: None,
        policy_refs: policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: tool_ref.to_string(),
        tool_version: tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn schema_compatibility_key_input(input: &SchemaCompatibilityKeyInput<'_>) -> Result<EvalCacheKeyInput> {
    let mut dependencies = vec![
        input.expected_identity_ref.to_string(),
        input.actual_identity_ref.to_string(),
    ];
    if let Some(alias_ref) = input.alias_ref {
        validate_ref(alias_ref, "schema compatibility alias ref")?;
        dependencies.push(alias_ref.to_string());
    }
    if let Some(migration_ref) = input.migration_ref {
        validate_ref(migration_ref, "schema compatibility migration ref")?;
        dependencies.push(migration_ref.to_string());
    }
    dependencies.sort();
    let closure_hash = canonical_hash(&record("eval-cache-schema-compat-closure", vec![refs_sequence(&dependencies)]))?;
    Ok(EvalCacheKeyInput {
        operation: "schema-compat".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-schema-compat-input", vec![
            string(input.expected_identity_ref),
            string(input.actual_identity_ref),
            optional_ref_value(input.alias_ref),
            optional_ref_value(input.migration_ref),
        ]))?,
        dependency_closure_hash: closure_hash,
        dependency_refs: dependencies,
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.tool_ref.to_string(),
        tool_version: input.tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn artifact_closure_key_input(input: &ArtifactClosureKeyInput<'_>) -> Result<EvalCacheKeyInput> {
    validate_refs(input.root_refs, "artifact closure root ref")?;
    validate_ref(input.closure_hash, "artifact closure hash")?;
    validate_refs(input.dependency_refs, "artifact closure dependency ref")?;
    Ok(EvalCacheKeyInput {
        operation: "artifact-closure".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-artifact-closure-input", vec![refs_sequence(&sorted_unique(
            input.root_refs,
        ))]))?,
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.tool_ref.to_string(),
        tool_version: input.tool_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn choreography_projection_key_input(input: &ChoreographyProjectionKeyInput<'_>) -> Result<EvalCacheKeyInput> {
    validate_ref(input.protocol_artifact_ref, "choreography protocol artifact ref")?;
    validate_ref(input.role_ref, "choreography role ref")?;
    validate_ref(input.closure_hash, "choreography closure hash")?;
    validate_refs(input.dependency_refs, "choreography dependency ref")?;
    validate_ref(input.projector_ref, "choreography projector ref")?;
    Ok(EvalCacheKeyInput {
        operation: "choreography-projection".to_string(),
        version: "v1".to_string(),
        input_ref: canonical_hash(&record("eval-cache-choreography-projection-input", vec![
            string(input.protocol_artifact_ref),
            string(input.role_ref),
        ]))?,
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: None,
        policy_refs: input.policy_refs.to_vec(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.projector_ref.to_string(),
        tool_version: input.projector_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn wasm_inspection_key_placeholder(
    module_artifact_ref: &str,
    inspector_ref: &str,
    inspector_version: &str,
) -> Result<EvalCacheKeyInput> {
    validate_ref(module_artifact_ref, "wasm module artifact ref")?;
    Ok(EvalCacheKeyInput {
        operation: "wasm-inspection".to_string(),
        version: "v1".to_string(),
        input_ref: module_artifact_ref.to_string(),
        dependency_closure_hash: canonical_hash(&record("eval-cache-wasm-closure", vec![string(module_artifact_ref)]))?,
        dependency_refs: vec![module_artifact_ref.to_string()],
        handler_profile_ref: None,
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: inspector_ref.to_string(),
        tool_version: inspector_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn transcript_run_key_placeholder(input: &TranscriptRunKeyInput<'_>) -> Result<EvalCacheKeyInput> {
    validate_ref(input.transcript_ref, "transcript ref")?;
    validate_ref(input.handler_profile_ref, "handler profile ref")?;
    Ok(EvalCacheKeyInput {
        operation: "transcript-run".to_string(),
        version: "v1".to_string(),
        input_ref: input.transcript_ref.to_string(),
        dependency_closure_hash: input.closure_hash.to_string(),
        dependency_refs: input.dependency_refs.to_vec(),
        handler_profile_ref: Some(input.handler_profile_ref.to_string()),
        policy_refs: Vec::new(),
        capability_refs: Vec::new(),
        revocation_refs: Vec::new(),
        tool_ref: input.harness_ref.to_string(),
        tool_version: input.harness_version.to_string(),
        assumption_refs: Vec::new(),
    })
}

pub fn parse_eval_cache_receipt(value: &IoValue) -> Result<EvalCacheReceipt> {
    let fields = value
        .collect_simple_record("eval-cache-receipt-v1", Some(8))
        .ok_or_else(|| MoltenError::invalid_harness("expected <eval-cache-receipt-v1 ...>"))?;
    require_schema(&fields[0], EVAL_CACHE_RECEIPT_SCHEMA, "eval cache receipt")?;
    let checks = parse_checks(&fields[7])?;
    if checks.is_empty() {
        return Err(MoltenError::invalid_harness("eval cache receipt missing checks"));
    }
    Ok(EvalCacheReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        key_ref: record_optional_ref(&fields[3], "key")?,
        value_ref: record_optional_ref(&fields[4], "value")?,
        value: value.clone(),
    })
}

fn read_key_value_pair(root: &Path, key_ref: &str) -> Result<Option<(EvalCacheKey, EvalCacheValue)>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let keys = read_txn.open_table(INDEX_KEYS).map_err(index_error)?;
    let values = read_txn.open_table(INDEX_VALUES).map_err(index_error)?;
    let Some(key_bytes) = keys.get(key_ref).map_err(index_error)? else {
        return Ok(None);
    };
    let Some(value_bytes) = values.get(key_ref).map_err(index_error)? else {
        return Ok(None);
    };
    let key = parse_eval_cache_key(&parse_canonical_bytes(key_bytes.value())?)?;
    let value = parse_eval_cache_value(&parse_canonical_bytes(value_bytes.value())?)?;
    Ok(Some((key, value)))
}

fn read_output(root: &Path, key_ref: &str, value: &EvalCacheValue) -> Result<Option<IoValue>> {
    match &value.output {
        EvalCacheOutputRef::None => Ok(None),
        EvalCacheOutputRef::Inline { output_ref, length } => {
            let db = ensure_index_tables(root)?;
            let read_txn = db.begin_read().map_err(index_error)?;
            let outputs = read_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
            let Some(bytes) = outputs.get(key_ref).map_err(index_error)? else {
                return Err(MoltenError::invalid_harness(format!("missing inline eval cache output for {key_ref}")));
            };
            let bytes = bytes.value().to_vec();
            if bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness("eval cache inline output length mismatch"));
            }
            let output = parse_canonical_bytes(&bytes)?;
            let actual_ref = canonical_hash(&output)?;
            if &actual_ref != output_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "eval cache output hash mismatch: got {actual_ref}, expected {output_ref}"
                )));
            }
            Ok(Some(output))
        }
        EvalCacheOutputRef::ContentRef {
            manifest_ref,
            output_ref,
            length,
        } => {
            let read = crate::chunk_store::read_object(&chunk_root(root), manifest_ref)?;
            if read.bytes.len() as u64 != *length {
                return Err(MoltenError::invalid_harness("eval cache content output length mismatch"));
            }
            let output = parse_canonical_bytes(&read.bytes)?;
            let actual_ref = canonical_hash(&output)?;
            if &actual_ref != output_ref {
                return Err(MoltenError::invalid_harness(format!(
                    "eval cache output hash mismatch: got {actual_ref}, expected {output_ref}"
                )));
            }
            Ok(Some(output))
        }
    }
}

fn store_key_value_in_tx(
    write_txn: &redb::WriteTransaction,
    key: &EvalCacheKey,
    value: &EvalCacheValue,
    output_bytes: Option<&[u8]>,
) -> Result<()> {
    {
        let mut keys = write_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        keys.insert(key.key_ref.as_str(), canonical_bytes(&key.value)?.as_slice()).map_err(index_error)?;
    }
    {
        let mut values = write_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        values
            .insert(key.key_ref.as_str(), canonical_bytes(&value.value)?.as_slice())
            .map_err(index_error)?;
    }
    if let (EvalCacheOutputRef::Inline { .. }, Some(output_bytes)) = (&value.output, output_bytes) {
        let mut outputs = write_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
        outputs.insert(key.key_ref.as_str(), output_bytes).map_err(index_error)?;
    }
    store_derived_indexes_in_tx(write_txn, key, value)
}

fn store_derived_indexes_in_tx(
    write_txn: &redb::WriteTransaction,
    key: &EvalCacheKey,
    value: &EvalCacheValue,
) -> Result<()> {
    insert_str_index(write_txn, INDEX_OPERATION, "operation", &key.operation, &key.key_ref)?;
    insert_str_index(write_txn, INDEX_STATUS, "status", &value.status, &key.key_ref)?;
    insert_str_index(write_txn, INDEX_TIER, "tier", &value.tier, &key.key_ref)?;
    for reference in &key.dependency_refs {
        insert_str_index(write_txn, INDEX_DEPENDENCY, "dependency", reference, &key.key_ref)?;
    }
    for reference in &value.dependency_refs {
        insert_str_index(write_txn, INDEX_DEPENDENCY, "dependency", reference, &key.key_ref)?;
    }
    for reference in &key.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", reference, &key.key_ref)?;
    }
    for reference in &value.policy_refs {
        insert_str_index(write_txn, INDEX_POLICY, "policy", reference, &key.key_ref)?;
    }
    for reference in &key.capability_refs {
        insert_str_index(write_txn, INDEX_CAPABILITY, "capability", reference, &key.key_ref)?;
    }
    for reference in &key.revocation_refs {
        insert_str_index(write_txn, INDEX_REVOCATION, "revocation", reference, &key.key_ref)?;
    }
    for reference in &value.evidence_refs {
        insert_str_index(write_txn, INDEX_EVIDENCE, "evidence", reference, &key.key_ref)?;
    }
    Ok(())
}

fn insert_str_index(
    write_txn: &redb::WriteTransaction,
    table: TableDefinition<&str, &str>,
    index_kind: &str,
    indexed: &str,
    key_ref: &str,
) -> Result<()> {
    let index_key =
        canonical_hash(&record("eval-cache-index-key", vec![string(index_kind), string(indexed), string(key_ref)]))?;
    let mut table = write_txn.open_table(table).map_err(index_error)?;
    table.insert(index_key.as_str(), key_ref).map_err(index_error)?;
    Ok(())
}

fn store_and_return_receipt(root: &Path, input: &EvalCacheReceiptValueInput<'_>) -> Result<IoValue> {
    let receipt = receipt_value(input)?;
    let db = ensure_index_tables(root)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    store_receipt_in_tx(&write_txn, &receipt)?;
    write_txn.commit().map_err(index_error)?;
    Ok(receipt)
}

fn store_receipt_in_tx(write_txn: &redb::WriteTransaction, receipt: &IoValue) -> Result<()> {
    let parsed = parse_eval_cache_receipt(receipt)?;
    let mut receipts = write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    receipts
        .insert(parsed.receipt_ref.as_str(), canonical_bytes(receipt)?.as_slice())
        .map_err(index_error)?;
    Ok(())
}

fn tombstone_reason(root: &Path, key_ref: &str) -> Result<Option<String>> {
    let db = ensure_index_tables(root)?;
    let read_txn = db.begin_read().map_err(index_error)?;
    let tombstones = read_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
    Ok(tombstones.get(key_ref).map_err(index_error)?.map(|value| value.value().to_string()))
}

fn receipt_value(input: &EvalCacheReceiptValueInput<'_>) -> Result<IoValue> {
    validate_non_empty(input.operation, "eval cache receipt operation")?;
    if !matches!(input.decision, "pass" | "deny") {
        return Err(MoltenError::invalid_harness(format!(
            "unsupported eval cache receipt decision {}",
            input.decision
        )));
    }
    if let Some(key_ref) = input.key_ref {
        validate_ref(key_ref, "eval cache receipt key ref")?;
    }
    if let Some(value_ref) = input.value_ref {
        validate_ref(value_ref, "eval cache receipt value ref")?;
    }
    validate_refs(input.refs, "eval cache receipt ref")?;
    Ok(record("eval-cache-receipt-v1", vec![
        string(EVAL_CACHE_RECEIPT_SCHEMA),
        record("operation", vec![string(input.operation)]),
        record("decision", vec![string(input.decision)]),
        record("key", vec![optional_ref_value(input.key_ref)]),
        record("value", vec![optional_ref_value(input.value_ref)]),
        record("refs", vec![refs_sequence(&sorted_unique(input.refs))]),
        record("diagnostics", vec![sequence(input.diagnostics.iter().map(string).collect())]),
        checks_value_from_pairs(input.checks),
    ]))
}

fn refs_for_key_value(key: &EvalCacheKey, value: &EvalCacheValue) -> Vec<String> {
    let mut refs = vec![
        key.key_ref.clone(),
        value.value_ref.clone(),
        key.input_ref.clone(),
        key.dependency_closure_hash.clone(),
        key.tool_ref.clone(),
    ];
    refs.extend(key.dependency_refs.iter().cloned());
    refs.extend(key.policy_refs.iter().cloned());
    refs.extend(key.capability_refs.iter().cloned());
    refs.extend(key.revocation_refs.iter().cloned());
    refs.extend(key.assumption_refs.iter().cloned());
    refs.extend(value.dependency_refs.iter().cloned());
    refs.extend(value.policy_refs.iter().cloned());
    refs.extend(value.evidence_refs.iter().cloned());
    if let Some(handler) = key.handler_profile_ref.as_ref() {
        refs.push(handler.clone());
    }
    sorted_unique(&refs)
}

fn policy_current_refs_match(key: &EvalCacheKey, input: &EvalCacheGetInput) -> bool {
    sorted_unique(&key.policy_refs) == sorted_unique(&input.current_policy_refs)
        && sorted_unique(&key.capability_refs) == sorted_unique(&input.current_capability_refs)
        && sorted_unique(&key.revocation_refs) == sorted_unique(&input.current_revocation_refs)
}

fn validate_key_input(input: &EvalCacheKeyInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_non_empty(&input.version, "eval cache key version")?;
    validate_ref(&input.input_ref, "eval cache input ref")?;
    validate_ref(&input.dependency_closure_hash, "eval cache dependency closure hash")?;
    validate_refs(&input.dependency_refs, "eval cache dependency ref")?;
    if let Some(handler_profile_ref) = input.handler_profile_ref.as_ref() {
        validate_ref(handler_profile_ref, "eval cache handler profile ref")?;
    }
    validate_refs(&input.policy_refs, "eval cache policy ref")?;
    validate_refs(&input.capability_refs, "eval cache capability ref")?;
    validate_refs(&input.revocation_refs, "eval cache revocation ref")?;
    validate_ref(&input.tool_ref, "eval cache tool ref")?;
    validate_non_empty(&input.tool_version, "eval cache tool version")?;
    validate_refs(&input.assumption_refs, "eval cache assumption ref")
}

fn validate_value_input(input: &EvalCacheValueInput) -> Result<()> {
    validate_tier(&input.tier)?;
    validate_status(&input.status)?;
    validate_refs(&input.dependency_refs, "eval cache value dependency ref")?;
    validate_refs(&input.policy_refs, "eval cache value policy ref")?;
    validate_refs(&input.evidence_refs, "eval cache value evidence ref")?;
    if input.tier == TIER_PRODUCTION_TRACE_ONLY {
        if input.status != STATUS_TRACE_ONLY {
            return Err(MoltenError::invalid_harness(
                "production-effectful trace-only cache values must use trace-only status",
            ));
        }
        if input.output.is_some() {
            return Err(MoltenError::invalid_harness(
                "production-effectful trace-only cache values cannot store semantic output",
            ));
        }
    }
    if input.status == STATUS_PASS && input.output.is_none() {
        return Err(MoltenError::invalid_harness("passing eval cache values require output"));
    }
    Ok(())
}

fn validate_value_against_key(key: &EvalCacheKey, input: &EvalCacheValueInput) -> Result<()> {
    if !input.dependency_refs.iter().all(|reference| key.dependency_refs.contains(reference)) {
        return Err(MoltenError::invalid_harness("eval cache value dependencies must be represented in key"));
    }
    if !input.policy_refs.iter().all(|reference| key.policy_refs.contains(reference)) {
        return Err(MoltenError::invalid_harness("eval cache value policy refs must be represented in key"));
    }
    if matches!(input.status.as_str(), STATUS_DENY | STATUS_ERROR) {
        if input.evidence_refs.is_empty() {
            return Err(MoltenError::invalid_harness("deterministic negative cache results require evidence refs"));
        }
        for evidence_ref in &input.evidence_refs {
            if !key.assumption_refs.contains(evidence_ref)
                && !key.policy_refs.contains(evidence_ref)
                && !key.capability_refs.contains(evidence_ref)
                && !key.revocation_refs.contains(evidence_ref)
            {
                return Err(MoltenError::invalid_harness(
                    "negative cache result evidence refs must be represented in key assumptions or policy inputs",
                ));
            }
        }
    }
    Ok(())
}

fn validate_output_ref(output: &EvalCacheOutputRef) -> Result<()> {
    match output {
        EvalCacheOutputRef::None => Ok(()),
        EvalCacheOutputRef::Inline { output_ref, .. } => validate_ref(output_ref, "eval cache inline output ref"),
        EvalCacheOutputRef::ContentRef {
            manifest_ref,
            output_ref,
            ..
        } => {
            validate_ref(manifest_ref, "eval cache content manifest ref")?;
            validate_ref(output_ref, "eval cache content output ref")
        }
    }
}

fn validate_invalidate_input(input: &EvalCacheInvalidateInput) -> Result<()> {
    if let Some(key_ref) = input.key_ref.as_ref() {
        validate_ref(key_ref, "invalidate key ref")?;
    }
    if let Some(dependency_ref) = input.dependency_ref.as_ref() {
        validate_ref(dependency_ref, "invalidate dependency ref")?;
    }
    if let Some(policy_ref) = input.policy_ref.as_ref() {
        validate_ref(policy_ref, "invalidate policy ref")?;
    }
    if let Some(capability_ref) = input.capability_ref.as_ref() {
        validate_ref(capability_ref, "invalidate capability ref")?;
    }
    if let Some(revocation_ref) = input.revocation_ref.as_ref() {
        validate_ref(revocation_ref, "invalidate revocation ref")?;
    }
    if let Some(operation) = input.operation.as_ref() {
        validate_operation(operation)?;
    }
    validate_refs(&input.apply_refs, "invalidate apply ref")?;
    retention::validate_destructive_retention_evidence(&input.retention_evidence)?;
    Ok(())
}

fn validate_operation(operation: &str) -> Result<()> {
    validate_non_empty(operation, "eval cache operation")?;
    if operation.chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_') {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "eval cache operation {operation} must use lowercase ascii, digits, '-' or '_'"
        )))
    }
}

fn validate_tier(tier: &str) -> Result<()> {
    if matches!(tier, TIER_PURE | TIER_SIMULATED | TIER_POLICY_CURRENT | TIER_PRODUCTION_TRACE_ONLY) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported eval cache tier {tier}")))
    }
}

fn validate_status(status: &str) -> Result<()> {
    if matches!(status, STATUS_PASS | STATUS_DENY | STATUS_ERROR | STATUS_TRACE_ONLY) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported eval cache status {status}")))
    }
}

fn output_ref_value(output: &EvalCacheOutputRef) -> IoValue {
    record("output", vec![match output {
        EvalCacheOutputRef::None => record("none", Vec::new()),
        EvalCacheOutputRef::Inline { output_ref, length } => {
            record("inline", vec![string(output_ref), u64_value(*length)])
        }
        EvalCacheOutputRef::ContentRef {
            manifest_ref,
            output_ref,
            length,
        } => record("content-ref", vec![string(manifest_ref), string(output_ref), u64_value(*length)]),
    }])
}

fn parse_output_ref(value: &Value<IoValue>) -> Result<EvalCacheOutputRef> {
    let value = value_to_iovalue(value);
    let output = simple_record(&value, "output", 1)?;
    let payload = value_to_iovalue(&output[0]);
    if payload.collect_simple_record("none", Some(0)).is_some() {
        return Ok(EvalCacheOutputRef::None);
    }
    if let Some(inline) = payload.collect_simple_record("inline", Some(2)) {
        return Ok(EvalCacheOutputRef::Inline {
            output_ref: required_ref(&inline[0], "inline output ref")?,
            length: required_u64(&inline[1], "inline output length")?,
        });
    }
    if let Some(content) = payload.collect_simple_record("content-ref", Some(3)) {
        return Ok(EvalCacheOutputRef::ContentRef {
            manifest_ref: required_ref(&content[0], "content output manifest ref")?,
            output_ref: required_ref(&content[1], "content output ref")?,
            length: required_u64(&content[2], "content output length")?,
        });
    }
    Err(MoltenError::invalid_harness("eval cache output must be none, inline, or content-ref"))
}

fn clear_derived_index_tables_in_tx(write_txn: &redb::WriteTransaction) -> Result<()> {
    clear_str_table(write_txn, INDEX_OPERATION)?;
    clear_str_table(write_txn, INDEX_DEPENDENCY)?;
    clear_str_table(write_txn, INDEX_POLICY)?;
    clear_str_table(write_txn, INDEX_CAPABILITY)?;
    clear_str_table(write_txn, INDEX_REVOCATION)?;
    clear_str_table(write_txn, INDEX_EVIDENCE)?;
    clear_str_table(write_txn, INDEX_STATUS)?;
    clear_str_table(write_txn, INDEX_TIER)
}

fn clear_str_table(write_txn: &redb::WriteTransaction, table: TableDefinition<&str, &str>) -> Result<()> {
    let mut table = write_txn.open_table(table).map_err(index_error)?;
    let keys = str_table_keys(&table)?;
    for key in keys {
        table.remove(key.as_str()).map_err(index_error)?;
    }
    Ok(())
}

fn str_table_keys(table: &redb::Table<'_, &str, &str>) -> Result<Vec<String>> {
    table
        .iter()
        .map_err(index_error)?
        .map(|item| item.map(|(key, _value)| key.value().to_string()).map_err(index_error))
        .collect()
}

fn ensure_dirs(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).map_err(MoltenError::from)?;
    std::fs::create_dir_all(chunk_root(root)).map_err(MoltenError::from)
}

fn ensure_index_tables(root: &Path) -> Result<Database> {
    ensure_dirs(root)?;
    let db = Database::create(index_path(root)).map_err(index_error)?;
    let write_txn = db.begin_write().map_err(index_error)?;
    {
        write_txn.open_table(INDEX_KEYS).map_err(index_error)?;
        write_txn.open_table(INDEX_VALUES).map_err(index_error)?;
        write_txn.open_table(INDEX_OUTPUTS).map_err(index_error)?;
        write_txn.open_table(INDEX_TOMBSTONES).map_err(index_error)?;
        write_txn.open_table(INDEX_OPERATION).map_err(index_error)?;
        write_txn.open_table(INDEX_DEPENDENCY).map_err(index_error)?;
        write_txn.open_table(INDEX_POLICY).map_err(index_error)?;
        write_txn.open_table(INDEX_CAPABILITY).map_err(index_error)?;
        write_txn.open_table(INDEX_REVOCATION).map_err(index_error)?;
        write_txn.open_table(INDEX_EVIDENCE).map_err(index_error)?;
        write_txn.open_table(INDEX_STATUS).map_err(index_error)?;
        write_txn.open_table(INDEX_TIER).map_err(index_error)?;
        write_txn.open_table(INDEX_RECEIPTS).map_err(index_error)?;
    }
    write_txn.commit().map_err(index_error)?;
    Ok(db)
}

fn chunk_root(root: &Path) -> PathBuf {
    root.join("chunks")
}

fn index_path(root: &Path) -> PathBuf {
    root.join(INDEX_FILE)
}

fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

fn sorted_unique(refs: &[String]) -> Vec<String> {
    refs.iter().cloned().collect::<BtreeSet<_>>().into_iter().collect()
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(fields) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&fields[0], "optional ref").map(Some);
    }
    required_ref(value, "optional ref").map(Some)
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_string(&record[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    required_ref(&record[0], label)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_optional_ref_value(&record[0])
}

fn record_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    parse_ref_sequence_value(&record[0], label)
}

fn record_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let record = simple_record(&value, label, 1)?;
    let items = required_sequence(&record[0], label)?;
    items.iter().map(|item| required_string(item, label)).collect()
}

fn parse_ref_sequence_value(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let items = required_sequence(value, label)?;
    let mut refs = Vec::with_capacity(items.len());
    for item in items.iter() {
        refs.push(required_ref(item, label)?);
    }
    Ok(refs)
}

fn checks_value(names: &[&str]) -> IoValue {
    checks_value_from_pairs(&names.iter().map(|name| (*name, "pass")).collect::<Vec<_>>())
}

fn checks_value_from_pairs(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<String>> {
    let value = value_to_iovalue(value);
    let checks = simple_record(&value, "checks", 1)?;
    let items = required_sequence(&checks[0], "checks")?;
    let mut parsed = Vec::with_capacity(items.len());
    for item in items.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record(&item, "check", 2)?;
        let name = required_string(&check[0], "check name")?;
        let status = required_string(&check[1], "check status")?;
        if status != "pass" && status != "fail" {
            return Err(MoltenError::invalid_harness(format!("eval cache check {name} has status {status}")));
        }
        parsed.push(name);
    }
    Ok(parsed)
}

fn require_check(checks: &[String], expected: &str, context: &str) -> Result<()> {
    if checks.iter().any(|check| check == expected) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing {expected} check")))
    }
}

fn require_schema(value: &Value<IoValue>, expected: &str, context: &str) -> Result<()> {
    let actual = required_string(value, context)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("unsupported {context} schema {actual}; expected {expected}")))
    }
}

fn simple_record<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

#[allow(clippy::owned_cow)]
fn required_sequence<'a>(value: &'a Value<IoValue>, field: &str) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let value = required_string(value, field)?;
    validate_ref(&value, field)?;
    Ok(value)
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn checked_table_len(count: u64, label: &str) -> Result<usize> {
    usize::try_from(count).map_err(|_| MoltenError::invalid_harness(format!("{label} count {count} exceeds usize")))
}

fn push_bounded<T>(values: &mut impl crate::bounded::VecSink<T>, value: T, maximum: usize, label: &str) -> Result<()> {
    let total = values
        .item_count()
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness(format!("{label} count overflow")))?;
    if total > maximum {
        return Err(MoltenError::invalid_harness(format!("{label} count {total} exceeds bound {maximum}")));
    }
    values.push_item(value);
    Ok(())
}

fn validate_ref(value_ref: &str, field: &str) -> Result<()> {
    validate_non_empty(value_ref, field)?;
    validate_content_ref(value_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref, got {value_ref}: {error}"))
    })
}

struct ApplyRefMatchInput<'a> {
    root: &'a Path,
    apply_refs: &'a [String],
    subsystem: &'a str,
    action: &'a str,
    object_ref: &'a str,
    object_kind: &'a str,
    retention_class: &'a str,
}

fn matching_apply_ref<'a>(input: ApplyRefMatchInput<'a>) -> Option<&'a str> {
    let mut fallback_ref = None;
    for apply_ref in input.apply_refs {
        let Ok(apply) = retention::read_retention_gc_apply(input.root, apply_ref) else {
            if fallback_ref.is_none() {
                fallback_ref = Some(apply_ref.as_str());
            }
            continue;
        };
        if apply.decision == "pass"
            && apply.subsystem == input.subsystem
            && apply.action == input.action
            && apply.object_ref == input.object_ref
            && apply.object_kind == input.object_kind
            && apply.retention_class == input.retention_class
        {
            return Some(apply_ref.as_str());
        }
        if fallback_ref.is_none() {
            fallback_ref = Some(apply_ref.as_str());
        }
    }
    fallback_ref
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    for value_ref in refs {
        validate_ref(value_ref, field)?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} cannot be empty")))
    } else {
        Ok(())
    }
}

fn index_error(error: impl std::fmt::Display) -> MoltenError {
    MoltenError::invalid_harness(format!("eval cache redb index error: {error}"))
}

#[cfg(test)]
mod tests {
    type AtomicU64 = std::sync::atomic::AtomicU64;
    type Ordering = std::sync::atomic::Ordering;
    type PathBuf = std::path::PathBuf;

    type TestCase = hegel::TestCase;

    use super::*;
    use crate::schema_identity;

    #[test]
    fn cache_hit_miss_output_integrity_and_no_name_key() {
        let root = temp_dir("eval-cache-hit");
        let key = key_input("schema-fingerprint", "input", &[]);
        let output = record("fingerprint", vec![string("ok")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[])).expect("put");
        let hit = get(&root, &put.key.key_ref, &EvalCacheGetInput::default()).expect("hit");
        assert_eq!(hit.output, Some(output));
        assert_eq!(hit.key.operation, "schema-fingerprint");
        let miss_key = eval_cache_key_value(&key_input("schema-fingerprint", "changed-input", &[])).expect("miss key");
        let miss = parse_eval_cache_key(&miss_key).expect("parse miss key");
        let error = get(&root, &miss.key_ref, &EvalCacheGetInput::default()).expect_err("miss denied");
        assert!(error.to_string().contains("miss"), "{error}");
        let renamed_key = EvalCacheKeyInput {
            assumption_refs: vec![test_ref("display-name-not-key")],
            ..key.clone()
        };
        let renamed_ref =
            canonical_hash(&eval_cache_key_value(&renamed_key).expect("renamed key")).expect("renamed ref");
        assert_ne!(put.key.key_ref, renamed_ref);
    }

    #[test]
    fn policy_current_revalidates_and_negative_results_require_key_evidence() {
        let root = temp_dir("eval-cache-policy-current");
        let denial_ref = test_ref("denial-input");
        let key = EvalCacheKeyInput {
            policy_refs: vec![test_ref("policy-v1")],
            assumption_refs: vec![denial_ref.clone()],
            ..key_input("schema-compat", "input", &[])
        };
        let output = record("denied", vec![string("policy")]);
        let policy_put = put(&root, &key, &EvalCacheValueInput {
            tier: TIER_POLICY_CURRENT.to_string(),
            status: STATUS_DENY.to_string(),
            output: Some(output),
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: key.policy_refs.clone(),
            evidence_refs: vec![denial_ref],
            diagnostics: vec!["policy denied".to_string()],
        })
        .expect("put policy current denial");
        let current = EvalCacheGetInput {
            current_policy_refs: key.policy_refs.clone(),
            semantic: true,
            ..EvalCacheGetInput::default()
        };
        get(&root, &policy_put.key.key_ref, &current).expect("policy current hit");
        let stale = EvalCacheGetInput {
            current_policy_refs: vec![test_ref("policy-v2")],
            semantic: true,
            ..EvalCacheGetInput::default()
        };
        let error = get(&root, &policy_put.key.key_ref, &stale).expect_err("stale denied");
        assert!(error.to_string().contains("stale"), "{error}");
        let bad = put(&root, &key_input("schema-compat", "bad-negative", &[]), &EvalCacheValueInput {
            tier: TIER_PURE.to_string(),
            status: STATUS_DENY.to_string(),
            output: Some(record("denied", vec![string("bad")])),
            dependency_refs: Vec::new(),
            policy_refs: Vec::new(),
            evidence_refs: vec![test_ref("unbound-denial")],
            diagnostics: vec!["bad negative".to_string()],
        })
        .expect_err("unbound denial evidence rejected");
        assert!(bad.to_string().contains("negative"), "{bad}");
    }

    #[test]
    fn trace_only_and_invalidation_fail_closed() {
        let root = temp_dir("eval-cache-trace");
        let dependency = test_ref("dependency");
        let key = key_input("transcript-run", "trace", std::slice::from_ref(&dependency));
        let trace = put(&root, &key, &EvalCacheValueInput {
            tier: TIER_PRODUCTION_TRACE_ONLY.to_string(),
            status: STATUS_TRACE_ONLY.to_string(),
            output: None,
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: Vec::new(),
            evidence_refs: vec![test_ref("trace-evidence")],
            diagnostics: vec!["production trace only".to_string()],
        })
        .expect("put trace-only");
        let error =
            get(&root, &trace.key.key_ref, &EvalCacheGetInput::default()).expect_err("trace-only semantic denied");
        assert!(error.to_string().contains("trace-only"), "{error}");
        let retention_evidence = retention_evidence(&root, "trace-invalidate");
        let apply_refs = vec![eval_cache_apply_ref(&root, &trace.key.key_ref, &retention_evidence)];
        let invalidated = invalidate(&root, &EvalCacheInvalidateInput {
            dependency_ref: Some(dependency),
            reason: "dependency changed".to_string(),
            retention_evidence,
            apply_refs,
            ..EvalCacheInvalidateInput::default()
        })
        .expect("invalidate dependency");
        assert!(invalidated.invalidated_key_refs.contains(&trace.key.key_ref));
        let miss = get(&root, &trace.key.key_ref, &EvalCacheGetInput {
            semantic: false,
            ..EvalCacheGetInput::default()
        })
        .expect_err("tombstone miss");
        assert!(miss.to_string().contains("tombstoned"), "{miss}");
    }

    #[test]
    fn invalidation_requires_retention_pass_before_tombstone() {
        let root = temp_dir("eval-cache-retention");
        let key = key_input("schema-fingerprint", "retained-input", &[]);
        let output = record("fingerprint", vec![string("retained")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put retained cache value");
        retention::pin_object(&root, retention::RetentionPinInput {
            object_ref: put.key.key_ref.clone(),
            object_kind: "eval-cache-key".to_string(),
            retention_class: retention::CLASS_EPHEMERAL_CACHE.to_string(),
            source: retention::SOURCE_EVALUATION_CACHE.to_string(),
            reason: "cache hold".to_string(),
            owner_ref: test_ref("retention-owner"),
            expiry_ref: None,
            policy_refs: vec![test_ref("retention-policy")],
            evidence_refs: vec![test_ref("retention-evidence")],
            has_authority: true,
        })
        .expect("retention pin");
        let invalidated = invalidate(&root, &EvalCacheInvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "retained".to_string(),
            retention_evidence: retention_evidence(&root, "retained-invalidate"),
            ..EvalCacheInvalidateInput::default()
        })
        .expect("invalidate retained key");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        assert!(!invalidated.retention_receipt_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &EvalCacheGetInput::default()).expect("retained cache hit");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn invalidation_denies_missing_authority_evidence() {
        let root = temp_dir("eval-cache-missing-authority");
        let key = key_input("schema-fingerprint", "missing-authority", &[]);
        let output = record("fingerprint", vec![string("missing-authority")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let mut retention_evidence = retention_evidence(&root, "missing-authority");
        retention_evidence.authority_refs.clear();
        let invalidated = invalidate(&root, &EvalCacheInvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "missing authority".to_string(),
            retention_evidence,
            ..EvalCacheInvalidateInput::default()
        })
        .expect("invalidate denied");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &EvalCacheGetInput::default()).expect("cache value remains");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn invalidation_denies_retained_reference_evidence() {
        let root = temp_dir("eval-cache-retained-ref");
        let key = key_input("schema-fingerprint", "retained-ref", &[]);
        let output = record("fingerprint", vec![string("retained-ref")]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output.clone()), &key, &[]))
            .expect("put cache value");
        let mut retention_evidence = retention_evidence(&root, "retained-ref");
        retention_evidence.retained_refs = vec![test_ref("retained-dependent-receipt")];
        let invalidated = invalidate(&root, &EvalCacheInvalidateInput {
            key_ref: Some(put.key.key_ref.clone()),
            reason: "retained ref".to_string(),
            retention_evidence,
            ..EvalCacheInvalidateInput::default()
        })
        .expect("invalidate denied");
        assert_eq!(invalidated.decision, "deny");
        assert!(invalidated.invalidated_key_refs.is_empty());
        let hit = get(&root, &put.key.key_ref, &EvalCacheGetInput::default()).expect("cache value remains");
        assert_eq!(hit.output, Some(output));
    }

    #[test]
    fn helper_keys_cover_schema_and_artifact_operations() {
        let shape = record("shape", vec![string("string")]);
        let (_normalized, shape_ref, fingerprint) =
            schema_identity::structural_fingerprint(&shape).expect("fingerprint");
        let key = schema_fingerprint_key_input(&shape_ref, &test_ref("tool"), "v1", &[test_ref("policy")])
            .expect("schema fingerprint key");
        assert_eq!(key.operation, "schema-fingerprint");
        let compat = schema_compatibility_key_input(&SchemaCompatibilityKeyInput {
            expected_identity_ref: &test_ref("expected"),
            actual_identity_ref: &test_ref("actual"),
            alias_ref: Some(&test_ref("alias")),
            migration_ref: None,
            tool_ref: &test_ref("tool"),
            tool_version: "v1",
            policy_refs: &[test_ref("policy")],
        })
        .expect("compat key");
        assert_eq!(compat.operation, "schema-compat");
        assert!(compat.dependency_refs.len() >= 3);
        let closure = artifact_closure_key_input(&ArtifactClosureKeyInput {
            root_refs: &[test_ref("root")],
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("dep")],
            tool_ref: &test_ref("registry-tool"),
            tool_version: "v1",
            policy_refs: &[test_ref("policy")],
        })
        .expect("closure key");
        assert_eq!(closure.operation, "artifact-closure");
        let choreography = choreography_projection_key_input(&ChoreographyProjectionKeyInput {
            protocol_artifact_ref: &test_ref("protocol"),
            role_ref: &test_ref("role"),
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("protocol-dep")],
            projector_ref: &test_ref("trellis-projector"),
            projector_version: "v1",
            policy_refs: &[test_ref("projection-policy")],
        })
        .expect("choreography projection key");
        assert_eq!(choreography.operation, "choreography-projection");
        assert!(choreography.dependency_refs.contains(&test_ref("protocol-dep")));
        let wasm =
            wasm_inspection_key_placeholder(&test_ref("module"), &test_ref("inspector"), "v1").expect("wasm key");
        assert_eq!(wasm.operation, "wasm-inspection");
        let transcript = transcript_run_key_placeholder(&TranscriptRunKeyInput {
            transcript_ref: &test_ref("transcript"),
            closure_hash: &fingerprint,
            dependency_refs: &[test_ref("transcript-dep")],
            handler_profile_ref: &test_ref("handler-profile"),
            harness_ref: &test_ref("harness"),
            harness_version: "v1",
        })
        .expect("transcript key");
        assert_eq!(transcript.operation, "transcript-run");
        assert_eq!(transcript.handler_profile_ref, Some(test_ref("handler-profile")));
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_key_determinism_dependency_invalidation_and_no_name_key(tc: TestCase) {
        let salt = tc.draw(hegel::generators::integers::<u64>().min_value(0).max_value(1_000_000));
        let dependency = test_ref(&format!("dep-{salt}"));
        let root = temp_dir("eval-cache-hegel");
        let key = key_input("artifact-closure", &format!("input-{salt}"), std::slice::from_ref(&dependency));
        let first_key_ref = canonical_hash(&eval_cache_key_value(&key).expect("first key")).expect("first key ref");
        let second_key_ref = canonical_hash(&eval_cache_key_value(&key).expect("second key")).expect("second key ref");
        assert_eq!(first_key_ref, second_key_ref);
        let output = record("closure", vec![string(&dependency)]);
        let put = put(&root, &key, &value_input(TIER_PURE, STATUS_PASS, Some(output), &key, &[])).expect("put");
        let retention_evidence = retention_evidence(&root, "hegel-invalidate");
        let apply_refs = vec![eval_cache_apply_ref(&root, &put.key.key_ref, &retention_evidence)];
        let invalidated = invalidate(&root, &EvalCacheInvalidateInput {
            dependency_ref: Some(dependency),
            reason: "property dependency invalidation".to_string(),
            retention_evidence,
            apply_refs,
            ..EvalCacheInvalidateInput::default()
        })
        .expect("invalidate");
        assert!(invalidated.invalidated_key_refs.contains(&put.key.key_ref));
        let display_name_key = EvalCacheKeyInput {
            assumption_refs: vec![test_ref(&format!("name-{salt}"))],
            ..key
        };
        let display_name_key_ref =
            canonical_hash(&eval_cache_key_value(&display_name_key).expect("display key")).expect("display key ref");
        assert_ne!(put.key.key_ref, display_name_key_ref);
    }

    fn key_input(operation: &str, input_label: &str, dependency_refs: &[String]) -> EvalCacheKeyInput {
        let deps = dependency_refs.to_vec();
        EvalCacheKeyInput {
            operation: operation.to_string(),
            version: "v1".to_string(),
            input_ref: test_ref(input_label),
            dependency_closure_hash: canonical_hash(&record("test-closure", vec![refs_sequence(&deps)]))
                .expect("closure"),
            dependency_refs: deps,
            handler_profile_ref: None,
            policy_refs: Vec::new(),
            capability_refs: Vec::new(),
            revocation_refs: Vec::new(),
            tool_ref: test_ref("tool"),
            tool_version: "test-v1".to_string(),
            assumption_refs: Vec::new(),
        }
    }

    fn value_input(
        tier: &str,
        status: &str,
        output: Option<IoValue>,
        key: &EvalCacheKeyInput,
        evidence_refs: &[String],
    ) -> EvalCacheValueInput {
        EvalCacheValueInput {
            tier: tier.to_string(),
            status: status.to_string(),
            output,
            dependency_refs: key.dependency_refs.clone(),
            policy_refs: key.policy_refs.clone(),
            evidence_refs: evidence_refs.to_vec(),
            diagnostics: Vec::new(),
        }
    }

    fn eval_cache_apply_ref(
        root: &std::path::Path,
        key_ref: &str,
        evidence: &retention::DestructiveRetentionEvidence,
    ) -> String {
        let plan = retention::store_retention_gc_plan(retention::RetentionGcPlanInput {
            root,
            subsystem: "eval-cache-invalidate",
            object_ref: key_ref,
            object_kind: "eval-cache-key",
            retention_class: retention::CLASS_EPHEMERAL_CACHE,
            action: retention::ACTION_TOMBSTONE,
            evidence,
        })
        .expect("store cache invalidation plan");
        retention::apply_retention_gc_plan(retention::RetentionGcApplyFromPlanInput {
            root,
            plan_ref: &plan.plan_ref,
        })
        .expect("apply cache invalidation plan")
        .apply_ref
    }

    fn retention_evidence(root: &std::path::Path, label: &str) -> retention::DestructiveRetentionEvidence {
        let requester_ref = test_ref(&format!("retention-requester-{label}"));
        let summaries = list(root, &EvalCacheListFilter::default()).expect("list cache for retention evidence");
        let mut policy_refs = Vec::with_capacity(summaries.len());
        let mut authority_refs = Vec::with_capacity(summaries.len());
        let mut evidence_refs = Vec::with_capacity(summaries.len());
        let mut reference_index_refs = Vec::with_capacity(summaries.len());
        for summary in summaries {
            policy_refs.push(store_admission(
                root,
                retention::ADMISSION_KIND_POLICY,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            authority_refs.push(store_admission(
                root,
                retention::ADMISSION_KIND_AUTHORITY,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            evidence_refs.push(store_admission(
                root,
                retention::ADMISSION_KIND_SUPPORTING_EVIDENCE,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
            reference_index_refs.push(store_admission(
                root,
                retention::ADMISSION_KIND_REFERENCE_INDEX,
                label,
                &requester_ref,
                &summary.key_ref,
                &[],
                true,
            ));
        }
        retention::DestructiveRetentionEvidence {
            requester_ref: Some(requester_ref),
            policy_refs,
            authority_refs,
            evidence_refs,
            retained_refs: Vec::new(),
            remote_peer_refs: Vec::new(),
            remote_refs: Vec::new(),
            reference_index_refs,
            remote_gc_refs: Vec::new(),
            remote_clearance_refs: Vec::new(),
            is_reference_index_complete: true,
        }
    }

    fn store_admission(
        root: &std::path::Path,
        kind: &str,
        label: &str,
        requester_ref: &str,
        key_ref: &str,
        remote_refs: &[String],
        is_reference_index_complete: bool,
    ) -> String {
        retention::store_retention_evidence_admission(root, &retention::RetentionEvidenceAdmissionInput {
            kind,
            decision: "pass",
            requester_ref,
            object_ref: key_ref,
            object_kind: "eval-cache-key",
            retention_class: retention::CLASS_EPHEMERAL_CACHE,
            action: retention::ACTION_TOMBSTONE,
            bound_refs: &[test_ref(&format!("{kind}-{label}"))],
            retained_refs: &[],
            remote_refs,
            is_reference_index_complete,
            is_current: true,
            revoked_refs: &[],
            diagnostics: &[],
        })
        .expect("store retention admission")
        .admission_ref
    }

    fn test_ref(label: &str) -> String {
        canonical_hash(&record("eval-cache-test-ref", vec![string(label)])).expect("test ref")
    }

    fn temp_dir(name: &str) -> PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
        let nonce = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{name}-{}-{nonce}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
