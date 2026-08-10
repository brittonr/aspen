pub const SERVICE_MONITOR_NOTIFICATION_SCHEMA: &str = "molten.service.monitor-notification.v1";
pub const SERVICE_FAILURE_MARKER_SCHEMA: &str = "molten.service.failure.v1";
pub const SERVICE_RETRACTION_SCHEMA: &str = "molten.service.retraction.v1";
pub const SERVICE_RETENTION_INPUT_SCHEMA: &str = "molten.service.retention-input.v1";
pub const SERVICE_OWNED_STATE_SCHEMA: &str = "molten.service.owned-state.v1";
pub const SERVICE_RUNTIME_SUITE_SCHEMA: &str = "molten.service.runtime-suite.v1";
pub const SERVICE_RUNTIME_REPORT_SCHEMA: &str = "molten.service.runtime-report.v1";
pub const SERVICE_READINESS_ASSERTION_SCHEMA: &str = "molten.service.readiness.v1";
pub const SERVICE_REPLAY_IDENTITY_SCHEMA: &str = "molten.service.replay-identity.v1";
pub const SERVICE_TURN_CONTEXT_SCHEMA: &str = "molten.service.turn-context.v1";
pub const PROTOCOL_MANIFEST_SCHEMA: &str = "molten.protocol.manifest.v1";
pub const PROTOCOL_INSTALL_RECEIPT_SCHEMA: &str = "molten.protocol.install-receipt.v1";
pub const PROTOCOL_ENDPOINT_SCHEMA: &str = "molten.protocol.endpoint.v1";
pub const PROTOCOL_LOCAL_STATE_SCHEMA: &str = "molten.protocol.local-state.v1";
pub const PROTOCOL_SESSION_STATE_SCHEMA: &str = "molten.protocol.session-state.v1";
pub const PROTOCOL_MESSAGE_SCHEMA: &str = "molten.protocol.message.v1";
pub const PROTOCOL_OPERATION_RECEIPT_SCHEMA: &str = "molten.protocol.operation-receipt.v1";
pub const PROTOCOL_SESSION_GATE_RECEIPT_SCHEMA: &str = "molten.protocol.session-gate-receipt.v1";
pub const RAFT_GROUP_MANIFEST_SCHEMA: &str = "molten.raft.group-manifest.v1";
pub const RAFT_COMMAND_ENVELOPE_SCHEMA: &str = "molten.raft.command-envelope.v1";
pub const RAFT_LOG_ENTRY_SCHEMA: &str = "molten.raft.log-entry.v1";
pub const RAFT_COMMIT_RECEIPT_SCHEMA: &str = "molten.raft.commit-receipt.v1";
pub const RAFT_READ_RECEIPT_SCHEMA: &str = "molten.raft.read-receipt.v1";
pub const RAFT_SNAPSHOT_SCHEMA: &str = "molten.raft.snapshot.v1";
pub const RAFT_RECOVERY_RECEIPT_SCHEMA: &str = "molten.raft.recovery-receipt.v1";
pub const RAFT_PREDICATE_RECEIPT_SCHEMA: &str = "molten.raft.predicate-receipt.v1";
pub const CONTROL_REGISTRY_COMMAND_SCHEMA: &str = "molten.control-registry.command.v1";
pub const CONTROL_REGISTRY_STATE_SCHEMA: &str = "molten.control-registry.state.v1";
pub const CONTROL_REGISTRY_RECEIPT_SCHEMA: &str = "molten.control-registry.receipt.v1";
pub const TYPED_STORAGE_REF_SCHEMA: &str = "molten.storage.typed-ref.v1";
pub const TYPED_STORAGE_RECEIPT_SCHEMA: &str = "molten.storage.receipt.v1";
pub const TYPED_STORAGE_EFFECT_MANIFEST_SCHEMA: &str = "molten.storage.effect-manifest.v1";
pub const TYPED_STORAGE_SCHEMA_ARTIFACT_SCHEMA: &str = "molten.storage.schema-artifact.v1";
pub const TYPED_STORAGE_MIGRATION_RECIPE_SCHEMA: &str = "molten.storage.migration-recipe.v1";
pub const ARTIFACT_SCHEMA: &str = "molten.artifacts.artifact.v1";
pub const ARTIFACT_NAME_POINTER_SCHEMA: &str = "molten.artifacts.name-pointer.v1";
pub const ARTIFACT_NAME_VIEW_SCHEMA: &str = "molten.artifacts.name-view.v1";
pub const ARTIFACT_RECEIPT_SCHEMA: &str = "molten.artifacts.receipt.v1";
pub const ARTIFACT_IDENTITY_RECEIPT_SCHEMA: &str = "molten.artifacts.identity-receipt.v1";
pub const ARTIFACT_DEPENDENCY_EDGE_SCHEMA: &str = "molten.artifacts.dependency-edge.v1";
pub const ARTIFACT_CLOSURE_SCHEMA: &str = "molten.artifacts.closure.v1";
pub const ARTIFACT_RELEASE_SNAPSHOT_SCHEMA: &str = "molten.artifacts.release-snapshot.v1";
pub const SCHEMA_IDENTITY_SCHEMA: &str = "molten.schema.identity.v1";
pub const SCHEMA_ALIAS_SCHEMA: &str = "molten.schema.alias.v1";
pub const SCHEMA_COMPATIBILITY_SCHEMA: &str = "molten.schema.compatibility.v1";
pub const SCHEMA_COMPATIBILITY_RECEIPT_SCHEMA: &str = "molten.schema.compatibility-receipt.v1";
pub const SCHEMA_STRUCTURAL_FINGERPRINT_SCHEMA: &str = "molten.schema.structural-fingerprint.v1";
pub const EVAL_CACHE_KEY_SCHEMA: &str = "molten.eval-cache.key.v1";
pub const EVAL_CACHE_VALUE_SCHEMA: &str = "molten.eval-cache.value.v1";
pub const EVAL_CACHE_RECEIPT_SCHEMA: &str = "molten.eval-cache.receipt.v1";
pub const TRANSCRIPT_ARTIFACT_SCHEMA: &str = "molten.transcript.artifact.v1";
pub const TRANSCRIPT_STANZA_SCHEMA: &str = "molten.transcript.stanza.v1";
pub const TRANSCRIPT_STANZA_OUTCOME_SCHEMA: &str = "molten.transcript.stanza-outcome.v1";
pub const TRANSCRIPT_RUN_RECEIPT_SCHEMA: &str = "molten.transcript.run-receipt.v1";
pub const REWRITE_QUERY_SCHEMA: &str = "molten.rewrite.query.v1";
pub const REWRITE_MATCH_SCHEMA: &str = "molten.rewrite.match.v1";
pub const REWRITE_DIFF_SCHEMA: &str = "molten.rewrite.diff.v1";
pub const REWRITE_PLAN_SCHEMA: &str = "molten.rewrite.plan.v1";
pub const REWRITE_RECEIPT_SCHEMA: &str = "molten.rewrite.receipt.v1";
pub const CATALOG_SUMMARY_SCHEMA: &str = "molten.catalog.summary.v1";
pub const CATALOG_VIEW_SCHEMA: &str = "molten.catalog.view.v1";
pub const CATALOG_QUERY_SCHEMA: &str = "molten.catalog.query.v1";
pub const CATALOG_RESULT_SCHEMA: &str = "molten.catalog.result.v1";
pub const CATALOG_RECEIPT_SCHEMA: &str = "molten.catalog.receipt.v1";
pub const CATALOG_SHORT_ID_SCHEMA: &str = "molten.catalog.short-id-resolution.v1";
pub const CATALOG_MCP_REQUEST_SCHEMA: &str = "molten.catalog.mcp-request.v1";
pub const CATALOG_MCP_RESPONSE_SCHEMA: &str = "molten.catalog.mcp-response.v1";
pub const CATALOG_MCP_RECEIPT_SCHEMA: &str = "molten.catalog.mcp-receipt.v1";
pub const JOB_DAG_SCHEMA: &str = "molten.job-dag.dag.v1";
pub const JOB_DAG_NODE_SCHEMA: &str = "molten.job-dag.node.v1";
pub const JOB_DAG_EDGE_SCHEMA: &str = "molten.job-dag.edge.v1";
pub const JOB_DAG_OUTPUT_REQUEST_SCHEMA: &str = "molten.job-dag.output-request.v1";
pub const JOB_DAG_RECEIPT_SCHEMA: &str = "molten.job-dag.receipt.v1";
pub const JOB_STAGE_OPERATION_SCHEMA: &str = "molten.job-dag.stage-operation.v1";
pub const JOB_PLAN_SCHEMA: &str = "molten.job-dag.plan.v1";
pub const JOB_PROFILE_SCHEMA: &str = "molten.job-dag.profile.v1";
pub const JOB_FUSION_PLAN_SCHEMA: &str = "molten.job-dag.fusion-plan.v1";
pub const JOB_PLAN_RECEIPT_SCHEMA: &str = "molten.job-dag.plan-receipt.v1";
pub const JOB_PROFILE_RECEIPT_SCHEMA: &str = "molten.job-dag.profile-receipt.v1";
pub const JOB_FUSION_RECEIPT_SCHEMA: &str = "molten.job-dag.fusion-receipt.v1";
pub const JOB_SYNC_REQUEST_SCHEMA: &str = "molten.job-dag.sync-request.v1";
pub const JOB_SYNC_PLAN_SCHEMA: &str = "molten.job-dag.sync-plan.v1";
pub const JOB_SYNC_RECEIPT_SCHEMA: &str = "molten.job-dag.sync-receipt.v1";
pub const JOB_ADMISSION_REQUEST_SCHEMA: &str = "molten.job-dag.admission-request.v1";
pub const JOB_ADMISSION_PLAN_SCHEMA: &str = "molten.job-dag.admission-plan.v1";
pub const JOB_ADMISSION_RECEIPT_SCHEMA: &str = "molten.job-dag.admission-receipt.v1";
pub const JOB_EXECUTION_REQUEST_SCHEMA: &str = "molten.job-dag.execution-request.v1";
pub const JOB_EXECUTION_RECEIPT_SCHEMA: &str = "molten.job-dag.execution-receipt.v1";
pub const JOB_REF_SUBMISSION_SCHEMA: &str = "molten.job-dag.blob-ref-submission.v1";
pub const JOB_REF_STATUS_SCHEMA: &str = "molten.job-dag.blob-ref-status.v1";
pub const JOB_REF_RECEIPT_SCHEMA: &str = "molten.job-dag.blob-ref-receipt.v1";
pub const JOB_WORKER_REQUEST_SCHEMA: &str = "molten.job-dag.worker-request.v1";
pub const JOB_WORKER_ASSIGNMENT_SCHEMA: &str = "molten.job-dag.worker-assignment.v1";
pub const JOB_WORKER_STATUS_SCHEMA: &str = "molten.job-dag.worker-status.v1";
pub const JOB_WORKER_RESULT_SCHEMA: &str = "molten.job-dag.worker-result.v1";
pub const JOB_WORKER_RECEIPT_SCHEMA: &str = "molten.job-dag.worker-receipt.v1";
pub const JOB_WORKER_SCHEDULE_RECEIPT_SCHEMA: &str = "molten.job-dag.worker-schedule-receipt.v1";
pub const UPGRADE_PLAN_SCHEMA: &str = "molten.upgrade.plan.v1";
pub const UPGRADE_RECEIPT_SCHEMA: &str = "molten.upgrade.receipt.v1";
pub const UPGRADE_NAME_POINTER_SCHEMA: &str = "molten.upgrade.name-pointer.v1";
pub const CHUNK_MANIFEST_SCHEMA: &str = "molten.chunk-store.manifest.v1";
pub const CHUNK_REF_SCHEMA: &str = "molten.chunk-store.chunk-ref.v1";
pub const CHUNK_ROOT_SCHEMA: &str = "molten.chunk-store.chunk-root.v1";
pub const CHUNK_STORE_RECEIPT_SCHEMA: &str = "molten.chunk-store.receipt.v1";
pub const CHUNK_LINEAGE_SCHEMA: &str = "molten.chunk-store.lineage.v1";
pub const OCTET_GATE_POLICY_SCHEMA: &str = "molten.octet.gate-policy.v1";
pub const OCTET_GATE_RECEIPT_SCHEMA: &str = "molten.octet.gate-receipt.v1";
pub const OCTET_STRUCTURED_FINDINGS_SCHEMA: &str = "molten.octet.structured-findings.v1";
pub const OCTET_FINGERPRINT_EVIDENCE_SCHEMA: &str = "molten.octet.fingerprint-evidence.v1";
pub const OCTET_COMMAND_ARTIFACT_SCHEMA: &str = "molten.octet.command-artifact.v1";
pub const OCTET_STATUS_ARTIFACT_SCHEMA: &str = "molten.octet.status-artifact.v1";
pub const OCTET_SUMMARY_ARTIFACT_SCHEMA: &str = "molten.octet.summary-artifact.v1";
pub const OCTET_OBJECT_CORPUS_ARTIFACT_SCHEMA: &str = "molten.octet.object-corpus-artifact.v1";
pub const OCTET_ARTIFACT_LEDGER_RECEIPT_SCHEMA: &str = "molten.octet.artifact-ledger-receipt.v1";
pub const OCTET_WARNING_BASELINE_SCHEMA: &str = "molten.octet.warning-baseline.v1";
pub const OCTET_BASELINE_RECEIPT_SCHEMA: &str = "molten.octet.baseline-receipt.v1";
pub const OCTET_REVIEW_MANIFEST_SCHEMA: &str = "molten.octet.review-manifest.v1";
pub const OCTET_SOURCE_GATE_REQUIREMENT_SCHEMA: &str = "molten.octet.source-gate-requirement.v1";
pub const OCTET_SOURCE_GATE_VALIDATION_SCHEMA: &str = "molten.octet.source-gate-validation.v1";
pub const OCTET_REMEDIATION_PLAN_SCHEMA: &str = "molten.octet.remediation-plan.v1";
pub const HASH_ALGORITHM: &str = "blake3-preserves-packed-v1";
const BLAKE3_REF_PREFIX: &str = "blake3:";
const BLAKE3_HEX_LEN: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentRef(String);

impl ContentRef {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_content_ref(value)?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ContentRef {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for ContentRef {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::str::FromStr for ContentRef {
    type Err = MoltenError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for ContentRef {
    type Error = MoltenError;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl TryFrom<String> for ContentRef {
    type Error = MoltenError;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl serde::Serialize for ContentRef {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> serde::Deserialize<'de> for ContentRef {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

pub fn validate_content_ref(value: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must start with {BLAKE3_REF_PREFIX}, got {value}"
        )));
    };
    validate_content_ref_hex(value, hex)
}

pub fn content_ref_has_prefix(value: &str) -> bool {
    value.starts_with(BLAKE3_REF_PREFIX)
}

pub fn content_ref_hex(value: &str) -> Result<&str> {
    let Some(hex) = value.strip_prefix(BLAKE3_REF_PREFIX) else {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must start with {BLAKE3_REF_PREFIX}, got {value}"
        )));
    };
    validate_content_ref_hex(value, hex)?;
    Ok(hex)
}

pub fn content_ref_from_hex(hex: &str) -> Result<String> {
    let reference = format!("{BLAKE3_REF_PREFIX}{hex}");
    validate_content_ref_hex(&reference, hex)?;
    Ok(reference)
}

fn validate_content_ref_hex(value: &str, hex: &str) -> Result<()> {
    if hex.len() != BLAKE3_HEX_LEN {
        return Err(MoltenError::invalid_harness(format!(
            "content ref must be {BLAKE3_REF_PREFIX}<64 lowercase hex chars>, got {value}"
        )));
    }
    if !hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)) {
        return Err(MoltenError::invalid_harness(format!("content ref must use lowercase hex chars, got {value}")));
    }
    Ok(())
}

pub fn parse_text(source: &str) -> Result<IoValue> {
    preserves::read_iovalue_text(source, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn to_text(value: &IoValue) -> Result<String> {
    preserves::write_iovalue_text(value, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

pub fn canonical_bytes(value: &IoValue) -> Result<Vec<u8>> {
    preserves::write_iovalue_packed(value, false).map_err(|error| MoltenError::Preserves(error.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrictCanonicalDecode {
    pub value: IoValue,
    pub canonical_bytes: Vec<u8>,
    pub value_ref: ContentRef,
}

// r[impl molten.preserves_canonical_bytes.strict_decode]
// r[impl molten.preserves_canonical_bytes.noncanonical_denial]
pub fn strict_canonical_decode(bytes: &[u8]) -> Result<StrictCanonicalDecode> {
    let value = preserves::read_iovalue_packed(bytes, false).map_err(|error| MoltenError::Preserves(error.to_string()))?;
    let canonical = canonical_bytes(&value)?;
    if canonical.as_slice() != bytes {
        return Err(MoltenError::invalid_harness(
            "strict canonical Preserves decode failed: input bytes differ from canonical re-encoding",
        ));
    }
    let value_ref = ContentRef::parse(content_ref_from_bytes(&canonical))?;
    Ok(StrictCanonicalDecode {
        value,
        canonical_bytes: canonical,
        value_ref,
    })
}

pub fn strict_canonical_decode_with_ref(
    bytes: &[u8],
    expected_ref: &str,
    boundary: &str,
) -> Result<StrictCanonicalDecode> {
    let expected = ContentRef::parse(expected_ref).map_err(|error| {
        MoltenError::invalid_harness(format!("{boundary} expected content ref is invalid: {error}"))
    })?;
    let decoded = strict_canonical_decode(bytes)?;
    if decoded.value_ref != expected {
        return Err(MoltenError::invalid_harness(format!(
            "{boundary} strict canonical decode ref mismatch: expected {}, got {}",
            expected, decoded.value_ref
        )));
    }
    Ok(decoded)
}

pub fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    Ok(strict_canonical_decode(bytes)?.value)
}

pub fn canonical_hash(value: &IoValue) -> Result<String> {
    let bytes = canonical_bytes(value)?;
    Ok(content_ref_from_bytes(&bytes))
}

pub fn content_ref_from_bytes(bytes: &[u8]) -> String {
    content_ref_from_blake3_hash(blake3::hash(bytes))
}

pub fn content_ref_from_blake3_hash(hash: blake3::Hash) -> String {
    format!("{BLAKE3_REF_PREFIX}{}", hash.to_hex())
}

pub fn canonical_content_ref(value: &IoValue) -> Result<ContentRef> {
    ContentRef::parse(&canonical_hash(value)?)
}

const DECISION_PASS: &str = "pass";
const DECISION_DENY: &str = "deny";
const CHECK_STATUS_FAIL: &str = "fail";
const CHECK_STATUS_DIAGNOSTIC: &str = "diagnostic";
const REPLAY_CLASS_IDEMPOTENT: &str = "idempotent";
const REPLAY_CLASS_DETERMINISTIC: &str = "deterministic";
const REPLAY_CLASS_EFFECTFUL: &str = "effectful";
const OPERATION_FIRST_CHAR_LABEL: &str = "operation id first character";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableId(String);

impl StableId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "stable id")?;
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SchemaId(StableId);

impl SchemaId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "schema id")?;
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId(StableId);

impl OperationId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "operation id")?;
        let first = value
            .bytes()
            .next()
            .ok_or_else(|| MoltenError::invalid_harness("operation id cannot be empty"))?;
        if !first.is_ascii_lowercase() {
            return Err(MoltenError::invalid_harness(format!(
                "{OPERATION_FIRST_CHAR_LABEL} must be lowercase ascii, got {value}"
            )));
        }
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(StableId);

impl ProfileId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        validate_stable_id(value, "profile id")?;
        Ok(Self(StableId(value.to_string())))
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Deny,
}

impl Decision {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            DECISION_DENY => Ok(Self::Deny),
            _ => Err(MoltenError::invalid_harness(format!("unsupported decision {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => DECISION_PASS,
            Self::Deny => DECISION_DENY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Fail,
    Deny,
    Diagnostic,
}

impl CheckStatus {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            DECISION_PASS => Ok(Self::Pass),
            CHECK_STATUS_FAIL => Ok(Self::Fail),
            DECISION_DENY => Ok(Self::Deny),
            CHECK_STATUS_DIAGNOSTIC => Ok(Self::Diagnostic),
            _ => Err(MoltenError::invalid_harness(format!("unsupported check status {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => DECISION_PASS,
            Self::Fail => CHECK_STATUS_FAIL,
            Self::Deny => DECISION_DENY,
            Self::Diagnostic => CHECK_STATUS_DIAGNOSTIC,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayClass {
    Idempotent,
    Deterministic,
    Effectful,
}

impl ReplayClass {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            REPLAY_CLASS_IDEMPOTENT => Ok(Self::Idempotent),
            REPLAY_CLASS_DETERMINISTIC => Ok(Self::Deterministic),
            REPLAY_CLASS_EFFECTFUL => Ok(Self::Effectful),
            _ => Err(MoltenError::invalid_harness(format!("unsupported replay class {value}"))),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idempotent => REPLAY_CLASS_IDEMPOTENT,
            Self::Deterministic => REPLAY_CLASS_DETERMINISTIC,
            Self::Effectful => REPLAY_CLASS_EFFECTFUL,
        }
    }
}

pub fn validate_stable_id(value: &str, label: &str) -> Result<()> {
    if value.is_empty() {
        return Err(MoltenError::invalid_harness(format!("{label} cannot be empty")));
    }
    if value.bytes().all(is_stable_id_byte) {
        return Ok(());
    }
    Err(MoltenError::invalid_harness(format!(
        "{label} must contain only ASCII letters, digits, '_', '.', ':', or '-', got {value}"
    )))
}

fn is_stable_id_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-')
}

pub fn symbol(name: &'static str) -> IoValue {
    IoValue::symbol(name)
}

pub fn string(value: impl AsRef<str>) -> IoValue {
    IoValue::new(value.as_ref().to_owned())
}

pub fn u64_value(value: u64) -> IoValue {
    IoValue::new(value)
}

pub fn bool_value(value: bool) -> IoValue {
    IoValue::new(value)
}

pub fn sequence(values: Vec<IoValue>) -> IoValue {
    IoValue::new(values)
}

pub fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    IoValue::record(symbol(label), fields)
}

pub fn value_to_iovalue(value: &Value<IoValue>) -> IoValue {
    IoValue::from(value.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCheck {
    pub name: String,
    pub status: String,
}

// r[impl molten.preserves_rail_toolkit.parser_builders]
// r[impl molten.preserves_rail_toolkit.negative_shapes]
pub fn simple_record_fields<'a>(
    value: &'a IoValue,
    label: &str,
    arity: usize,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value
        .collect_simple_record(label, Some(arity))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...> with arity {arity}")))
}

pub fn required_string_field(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.to_string())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

pub fn required_content_ref(value: &Value<IoValue>, field: &str) -> Result<ContentRef> {
    let reference = required_string_field(value, field)?;
    ContentRef::parse(&reference).map_err(|error| {
        MoltenError::invalid_harness(format!("{field} must be a canonical content ref: {error}"))
    })
}

pub fn required_content_ref_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    Ok(required_content_ref(value, field)?.into_string())
}

pub fn optional_content_ref(value: &Value<IoValue>, field: &str) -> Result<Option<ContentRef>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_content_ref(&some[0], field).map(Some);
    }
    required_content_ref(value, field).map(Some)
}

pub fn optional_content_ref_string(value: &Value<IoValue>, field: &str) -> Result<Option<String>> {
    Ok(optional_content_ref(value, field)?.map(ContentRef::into_string))
}

pub fn required_sequence_field<'a>(
    value: &'a Value<IoValue>,
    field: &str,
) -> Result<std::borrow::Cow<'a, Vec<Value<IoValue>>>> {
    value
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {field}")))
}

pub fn record_string_field(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    required_string_field(&record[0], field)
}

pub fn record_content_ref(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<ContentRef> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    required_content_ref(&record[0], field)
}

pub fn record_content_ref_string(value: &Value<IoValue>, record_name: &str, field: &str) -> Result<String> {
    Ok(record_content_ref(value, record_name, field)?.into_string())
}

pub fn record_content_ref_sequence(
    value: &Value<IoValue>,
    record_name: &str,
    field: &str,
    maximum: usize,
) -> Result<Vec<ContentRef>> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, record_name, 1)?;
    let values = required_sequence_field(&record[0], field)?;
    ensure_toolkit_count_at_most(values.len(), maximum, field)?;
    let mut refs = Vec::with_capacity(values.len());
    for item in values.iter() {
        refs.push(required_content_ref(item, field)?);
    }
    Ok(refs)
}

pub fn record_content_ref_strings(
    value: &Value<IoValue>,
    record_name: &str,
    field: &str,
    maximum: usize,
) -> Result<Vec<String>> {
    Ok(record_content_ref_sequence(value, record_name, field, maximum)?
        .into_iter()
        .map(ContentRef::into_string)
        .collect())
}

pub fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

pub fn refs_sequence(refs: &[String]) -> IoValue {
    sequence(refs.iter().map(string).collect())
}

// r[impl molten.preserves_rail_toolkit.check_sets]
pub fn checks_value(checks: &[(&str, &str)]) -> IoValue {
    record("checks", vec![sequence(
        checks.iter().map(|(name, status)| record("check", vec![string(name), string(status)])).collect(),
    )])
}

pub fn parse_checks_record(
    value: &Value<IoValue>,
    maximum: usize,
    context: &str,
) -> Result<Vec<ParsedCheck>> {
    let value = value_to_iovalue(value);
    let record = simple_record_fields(&value, "checks", 1)?;
    let values = required_sequence_field(&record[0], "checks")?;
    ensure_toolkit_count_at_most(values.len(), maximum, "checks")?;
    let mut checks = Vec::with_capacity(values.len());
    let mut seen = std::collections::BTreeSet::new();
    for item in values.iter() {
        let item = value_to_iovalue(item);
        let check = simple_record_fields(&item, "check", 2)?;
        let name = required_string_field(&check[0], "check name")?;
        let status = required_string_field(&check[1], "check status")?;
        if !matches!(status.as_str(), "pass" | "fail" | "deny") {
            return Err(MoltenError::invalid_harness(format!("unsupported {context} check status {status}")));
        }
        if !seen.insert(name.clone()) {
            return Err(MoltenError::invalid_harness(format!("duplicate {context} check {name}")));
        }
        checks.push(ParsedCheck { name, status });
    }
    Ok(checks)
}

pub fn require_checks_present(checks: &[ParsedCheck], expected: &[&str], context: &str) -> Result<()> {
    for expected in expected {
        if !checks.iter().any(|check| check.name == *expected) {
            return Err(MoltenError::invalid_harness(format!("missing {context} check {expected}")));
        }
    }
    Ok(())
}

fn ensure_toolkit_count_at_most(count: usize, maximum: usize, label: &str) -> Result<()> {
    if count <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {count} exceeds maximum {maximum}")))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryFieldKind {
    SchemaId,
    AnyRecord,
    AnySequenceRecord,
    ChainRecord,
    ChecksRecord,
    ConformanceRecord,
    DecisionRecord,
    FileRefsRecord,
    HostcallDescriptorsRecord,
    NonEmptyRefSequenceRecord,
    NonEmptyStringRecord,
    ObjectRecord,
    OptionalRefRecord,
    RefAndStringRecord,
    RefRecord,
    RefSequenceRecord,
    StableIdRecord,
    StringAndRefRecord,
    StringRecord,
    StringSequenceRecord,
    UniqueRefSequenceRecord,
    UniqueStringSequenceRecord,
    TwoRefsRecord,
    U64Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryFieldSpec {
    pub label: &'static str,
    pub kind: BoundaryFieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundarySchemaSpec {
    pub family: &'static str,
    pub version: &'static str,
    pub record_label: &'static str,
    pub schema_id: &'static str,
    pub fields: &'static [BoundaryFieldSpec],
}

impl BoundarySchemaSpec {
    pub fn arity(&self) -> usize {
        self.fields.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundarySchemaValidation {
    pub family: String,
    pub schema_ref: ContentRef,
    pub value_ref: ContentRef,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryCodecReport {
    pub family: String,
    pub schema_ref: ContentRef,
    pub input_bytes_ref: ContentRef,
    pub decoded_value_ref: ContentRef,
    pub typed_value_ref: ContentRef,
    pub decision: String,
    pub diagnostics: Vec<String>,
}

const SCHEMA_FIELD: BoundaryFieldSpec = BoundaryFieldSpec {
    label: "schema-id",
    kind: BoundaryFieldKind::SchemaId,
};

const NODE_CONTROL_INGRESS_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "transport", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "topic", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "from-peer", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "to-node", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "sequence", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "operation", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "request-ref", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "request", kind: BoundaryFieldKind::AnyRecord },
    BoundaryFieldSpec { label: "peer-bootstrap", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "authority", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "resource", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "evidence", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "decision", kind: BoundaryFieldKind::DecisionRecord },
    BoundaryFieldSpec { label: "plugin", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "manifest", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "operation", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "hostcall", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "executor", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "effect", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "authority", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "capability-grants", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "resource", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "evaluation-turn", kind: BoundaryFieldKind::U64Record },
    BoundaryFieldSpec { label: "diagnostics", kind: BoundaryFieldKind::StringSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const PLUGIN_EXTENSION_CONTRACT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "extension-id", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "version", kind: BoundaryFieldKind::NonEmptyStringRecord },
    BoundaryFieldSpec { label: "host-abi", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "lifecycle", kind: BoundaryFieldKind::UniqueStringSequenceRecord },
    BoundaryFieldSpec { label: "hostcalls", kind: BoundaryFieldKind::HostcallDescriptorsRecord },
    BoundaryFieldSpec { label: "conformance", kind: BoundaryFieldKind::ConformanceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::NonEmptyRefSequenceRecord },
    BoundaryFieldSpec { label: "supply-chain", kind: BoundaryFieldKind::NonEmptyRefSequenceRecord },
    BoundaryFieldSpec { label: "profile", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const RETENTION_RECEIPT_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "decision", kind: BoundaryFieldKind::DecisionRecord },
    BoundaryFieldSpec { label: "action", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "object", kind: BoundaryFieldKind::ObjectRecord },
    BoundaryFieldSpec { label: "class", kind: BoundaryFieldKind::StableIdRecord },
    BoundaryFieldSpec { label: "requester", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "index", kind: BoundaryFieldKind::RefRecord },
    BoundaryFieldSpec { label: "pins", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "retained", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "remote", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "tombstone", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "diagnostics", kind: BoundaryFieldKind::StringSequenceRecord },
    BoundaryFieldSpec { label: "policy", kind: BoundaryFieldKind::RefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "chain", kind: BoundaryFieldKind::ChainRecord },
    BoundaryFieldSpec { label: "anchor", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "head", kind: BoundaryFieldKind::OptionalRefRecord },
    BoundaryFieldSpec { label: "artifacts", kind: BoundaryFieldKind::AnySequenceRecord },
    BoundaryFieldSpec { label: "verify-receipts", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "checkpoints", kind: BoundaryFieldKind::UniqueRefSequenceRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

const OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_FIELDS: &[BoundaryFieldSpec] = &[
    SCHEMA_FIELD,
    BoundaryFieldSpec { label: "output-path", kind: BoundaryFieldKind::StringAndRefRecord },
    BoundaryFieldSpec { label: "members", kind: BoundaryFieldKind::FileRefsRecord },
    BoundaryFieldSpec { label: "dogfood", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "replay", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "nix", kind: BoundaryFieldKind::TwoRefsRecord },
    BoundaryFieldSpec { label: "nextest", kind: BoundaryFieldKind::RefAndStringRecord },
    BoundaryFieldSpec { label: "checks", kind: BoundaryFieldKind::ChecksRecord },
];

pub const NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "node-control-ingress-envelope",
    version: "v1",
    record_label: "node-control-ingress-envelope-v1",
    schema_id: NODE_CONTROL_INGRESS_ENVELOPE_SCHEMA,
    fields: NODE_CONTROL_INGRESS_BOUNDARY_FIELDS,
};

pub const PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "plugin-hostcall-receipt",
    version: "v1",
    record_label: "plugin-hostcall-receipt-v1",
    schema_id: PLUGIN_HOSTCALL_RECEIPT_SCHEMA,
    fields: PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_FIELDS,
};

pub const PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "plugin-extension-contract",
    version: "v1",
    record_label: "plugin-extension-contract-v1",
    schema_id: PLUGIN_EXTENSION_CONTRACT_SCHEMA,
    fields: PLUGIN_EXTENSION_CONTRACT_BOUNDARY_FIELDS,
};

pub const RETENTION_RECEIPT_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "retention-receipt",
    version: "v1",
    record_label: "retention-receipt-v1",
    schema_id: RETENTION_RECEIPT_SCHEMA,
    fields: RETENTION_RECEIPT_BOUNDARY_FIELDS,
};

pub const EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "evidence-chain-segment-bundle",
    version: "v1",
    record_label: "chain-segment-bundle-v1",
    schema_id: EVIDENCE_CHAIN_SEGMENT_BUNDLE_SCHEMA,
    fields: EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_FIELDS,
};

pub const OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_SCHEMA: BoundarySchemaSpec = BoundarySchemaSpec {
    family: "operator-release-evidence-bundle",
    version: "v1",
    record_label: "release-evidence-bundle-v1",
    schema_id: OPERATOR_RELEASE_EVIDENCE_BUNDLE_SCHEMA,
    fields: OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_FIELDS,
};

// r[impl molten.preserves_schema_boundaries.schema_artifacts]
pub fn boundary_schema_artifact_value(spec: &BoundarySchemaSpec) -> Result<IoValue> {
    let arity = u64::try_from(spec.arity()).map_err(|error| {
        MoltenError::invalid_harness(format!("boundary schema arity cannot convert to u64: {error}"))
    })?;
    Ok(record("preserves-boundary-schema-artifact-v1", vec![
        record("family", vec![string(spec.family)]),
        record("version", vec![string(spec.version)]),
        record("preserves-schema-version", vec![string(preserves_schema::PRESERVES_SCHEMA_SPEC_VERSION)]),
        record("record-label", vec![string(spec.record_label)]),
        record("schema-id", vec![string(spec.schema_id)]),
        record("arity", vec![u64_value(arity)]),
        record("fields", vec![sequence(
            spec.fields.iter().map(boundary_field_contract_value).collect(),
        )]),
    ]))
}

pub fn boundary_schema_ref(spec: &BoundarySchemaSpec) -> Result<ContentRef> {
    canonical_content_ref(&boundary_schema_artifact_value(spec)?)
}

pub fn validate_boundary_claimed_schema_ref(spec: &BoundarySchemaSpec, claimed_ref: &str) -> Result<ContentRef> {
    let expected = boundary_schema_ref(spec)?;
    let claimed = ContentRef::parse(claimed_ref).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: claimed schema ref is invalid using current schema {}: {error}",
            spec.family, expected
        ))
    })?;
    if claimed == expected {
        return Ok(expected);
    }
    Err(MoltenError::invalid_harness(format!(
        "{} schema validation deny: stale schema ref {} expected {}",
        spec.family, claimed, expected
    )))
}

fn boundary_field_contract_value(field: &BoundaryFieldSpec) -> IoValue {
    record("field", vec![
        record("label", vec![string(field.label)]),
        record("kind", vec![string(boundary_field_kind_name(field.kind))]),
        record("constraints", vec![sequence(
            boundary_field_constraints(field.kind)
                .iter()
                .map(|constraint| string(*constraint))
                .collect(),
        )]),
    ])
}

fn boundary_field_kind_name(kind: BoundaryFieldKind) -> &'static str {
    match kind {
        BoundaryFieldKind::SchemaId => "schema-id",
        BoundaryFieldKind::AnyRecord => "any-record",
        BoundaryFieldKind::AnySequenceRecord => "any-sequence-record",
        BoundaryFieldKind::ChainRecord => "chain-record",
        BoundaryFieldKind::ChecksRecord => "checks-record",
        BoundaryFieldKind::ConformanceRecord => "conformance-record",
        BoundaryFieldKind::DecisionRecord => "decision-record",
        BoundaryFieldKind::FileRefsRecord => "file-refs-record",
        BoundaryFieldKind::HostcallDescriptorsRecord => "hostcall-descriptors-record",
        BoundaryFieldKind::NonEmptyRefSequenceRecord => "non-empty-ref-sequence-record",
        BoundaryFieldKind::NonEmptyStringRecord => "non-empty-string-record",
        BoundaryFieldKind::ObjectRecord => "object-record",
        BoundaryFieldKind::OptionalRefRecord => "optional-ref-record",
        BoundaryFieldKind::RefAndStringRecord => "ref-and-string-record",
        BoundaryFieldKind::RefRecord => "ref-record",
        BoundaryFieldKind::RefSequenceRecord => "ref-sequence-record",
        BoundaryFieldKind::StableIdRecord => "stable-id-record",
        BoundaryFieldKind::StringAndRefRecord => "string-and-ref-record",
        BoundaryFieldKind::StringRecord => "string-record",
        BoundaryFieldKind::StringSequenceRecord => "string-sequence-record",
        BoundaryFieldKind::UniqueRefSequenceRecord => "unique-ref-sequence-record",
        BoundaryFieldKind::UniqueStringSequenceRecord => "unique-string-sequence-record",
        BoundaryFieldKind::TwoRefsRecord => "two-refs-record",
        BoundaryFieldKind::U64Record => "u64-record",
    }
}

fn boundary_field_constraints(kind: BoundaryFieldKind) -> &'static [&'static str] {
    match kind {
        BoundaryFieldKind::SchemaId => &["schema-id", "exact-current-schema"],
        BoundaryFieldKind::AnyRecord => &["record", "arity-one", "embedded-record"],
        BoundaryFieldKind::AnySequenceRecord => &["record", "arity-one", "sequence"],
        BoundaryFieldKind::ChainRecord => &["record", "chain-fields"],
        BoundaryFieldKind::ChecksRecord => &["record", "checks", "unique-check-names", "known-check-status"],
        BoundaryFieldKind::ConformanceRecord => &["record", "positive-negative-property-refs"],
        BoundaryFieldKind::DecisionRecord => &["record", "string", "decision-pass-or-deny"],
        BoundaryFieldKind::FileRefsRecord => &["record", "sequence", "file-ref-items"],
        BoundaryFieldKind::HostcallDescriptorsRecord => &["record", "sequence", "unique-operation-descriptor", "typed-embedded-record"],
        BoundaryFieldKind::NonEmptyRefSequenceRecord => &["record", "sequence", "content-ref-items", "non-empty"],
        BoundaryFieldKind::NonEmptyStringRecord => &["record", "string", "non-empty"],
        BoundaryFieldKind::ObjectRecord => &["record", "object-ref-and-kind"],
        BoundaryFieldKind::OptionalRefRecord => &["record", "optional-content-ref"],
        BoundaryFieldKind::RefAndStringRecord => &["record", "content-ref", "string"],
        BoundaryFieldKind::RefRecord => &["record", "content-ref"],
        BoundaryFieldKind::RefSequenceRecord => &["record", "sequence", "content-ref-items"],
        BoundaryFieldKind::StableIdRecord => &["record", "string", "stable-id"],
        BoundaryFieldKind::StringAndRefRecord => &["record", "string", "content-ref"],
        BoundaryFieldKind::StringRecord => &["record", "string"],
        BoundaryFieldKind::StringSequenceRecord => &["record", "sequence", "string-items"],
        BoundaryFieldKind::UniqueRefSequenceRecord => &["record", "sequence", "content-ref-items", "unique"],
        BoundaryFieldKind::UniqueStringSequenceRecord => &["record", "sequence", "string-items", "unique"],
        BoundaryFieldKind::TwoRefsRecord => &["record", "two-content-refs"],
        BoundaryFieldKind::U64Record => &["record", "u64"],
    }
}

// r[impl molten.preserves_schema_boundaries.schema_adapter]
// r[impl molten.preserves_schema_boundaries.schema_denials]
pub fn validate_boundary_schema(value: &IoValue, spec: &BoundarySchemaSpec) -> Result<BoundarySchemaValidation> {
    let schema_ref = boundary_schema_ref(spec)?;
    let value_ref = canonical_content_ref(value)?;
    let arity = spec.arity();
    let fields = value.collect_simple_record(spec.record_label, Some(arity)).ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: expected <{} ...> with arity {} using schema {}",
            spec.family, spec.record_label, arity, schema_ref
        ))
    })?;
    for (index, field_spec) in spec.fields.iter().enumerate() {
        validate_boundary_field(&fields[index], field_spec, spec, &schema_ref)?;
    }
    Ok(BoundarySchemaValidation {
        family: spec.family.to_string(),
        schema_ref,
        value_ref,
        decision: "pass".to_string(),
        diagnostics: Vec::new(),
    })
}

pub fn boundary_schema_diagnostic_value(validation: &BoundarySchemaValidation) -> IoValue {
    record("preserves-boundary-schema-validation-v1", vec![
        record("family", vec![string(&validation.family)]),
        record("schema-ref", vec![string(validation.schema_ref.as_str())]),
        record("value-ref", vec![string(validation.value_ref.as_str())]),
        record("decision", vec![string(&validation.decision)]),
        record("diagnostics", vec![sequence(validation.diagnostics.iter().map(string).collect())]),
    ])
}

// r[impl molten.preserves_boundary_codegen.typed_codecs]
// r[impl molten.preserves_boundary_codegen.strict_decode]
// r[impl molten.preserves_boundary_codegen.schema_ref_evidence]
pub fn validate_boundary_bytes(bytes: &[u8], spec: &BoundarySchemaSpec) -> Result<BoundaryCodecReport> {
    let decoded = strict_canonical_decode(bytes)?;
    let schema_ref = boundary_schema_ref(spec)?;
    let input_bytes_ref = ContentRef::parse(content_ref_from_bytes(bytes))?;
    let typed_value_ref = boundary_typed_value_ref(spec, decoded.value_ref.as_str())?;
    match validate_boundary_schema(&decoded.value, spec) {
        Ok(validation) => Ok(BoundaryCodecReport {
            family: validation.family,
            schema_ref: validation.schema_ref,
            input_bytes_ref,
            decoded_value_ref: validation.value_ref,
            typed_value_ref,
            decision: "pass".to_string(),
            diagnostics: Vec::new(),
        }),
        Err(error) => Ok(BoundaryCodecReport {
            family: spec.family.to_string(),
            schema_ref,
            input_bytes_ref,
            decoded_value_ref: decoded.value_ref,
            typed_value_ref,
            decision: "deny".to_string(),
            diagnostics: vec![error.to_string()],
        }),
    }
}

pub fn boundary_codec_report_value(report: &BoundaryCodecReport) -> IoValue {
    record("preserves-boundary-codec-report-v1", vec![
        record("family", vec![string(&report.family)]),
        record("schema-ref", vec![string(report.schema_ref.as_str())]),
        record("input-bytes-ref", vec![string(report.input_bytes_ref.as_str())]),
        record("decoded-value-ref", vec![string(report.decoded_value_ref.as_str())]),
        record("typed-value-ref", vec![string(report.typed_value_ref.as_str())]),
        record("decision", vec![string(&report.decision)]),
        record("diagnostics", vec![sequence(report.diagnostics.iter().map(string).collect())]),
    ])
}

fn boundary_typed_value_ref(spec: &BoundarySchemaSpec, decoded_value_ref: &str) -> Result<ContentRef> {
    canonical_content_ref(&record("preserves-boundary-typed-codec-v1", vec![
        record("family", vec![string(spec.family)]),
        record("schema-ref", vec![string(boundary_schema_ref(spec)?.as_str())]),
        record("decoded-value-ref", vec![string(decoded_value_ref)]),
    ]))
}

// r[impl molten.preserves_boundary_field_contracts.field_contracts]
// r[impl molten.preserves_boundary_field_contracts.field_contract_denials]
fn validate_boundary_field(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    match field_spec.kind {
        BoundaryFieldKind::SchemaId => validate_boundary_schema_id(value, spec, schema_ref),
        BoundaryFieldKind::AnyRecord => {
            boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
            Ok(())
        }
        BoundaryFieldKind::AnySequenceRecord => validate_any_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::ChainRecord => validate_chain_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::ChecksRecord => validate_checks_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::ConformanceRecord => validate_conformance_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::DecisionRecord => validate_decision_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::FileRefsRecord => validate_file_refs_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::HostcallDescriptorsRecord => validate_hostcall_descriptors_boundary_record(
            value,
            field_spec,
            spec,
            schema_ref,
        ),
        BoundaryFieldKind::NonEmptyRefSequenceRecord => {
            validate_ref_sequence_record_with_contract(value, field_spec.label, spec, schema_ref, true, false)
        }
        BoundaryFieldKind::NonEmptyStringRecord => validate_non_empty_string_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::ObjectRecord => validate_object_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::OptionalRefRecord => validate_optional_ref_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::RefAndStringRecord => validate_ref_and_string_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::RefRecord => validate_ref_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::RefSequenceRecord => validate_ref_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StableIdRecord => validate_stable_id_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StringAndRefRecord => validate_string_and_ref_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::StringRecord => validate_string_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::StringSequenceRecord => validate_string_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::UniqueRefSequenceRecord => {
            validate_ref_sequence_record_with_contract(value, field_spec.label, spec, schema_ref, false, true)
        }
        BoundaryFieldKind::UniqueStringSequenceRecord => validate_unique_string_sequence_record(value, field_spec.label, spec, schema_ref),
        BoundaryFieldKind::TwoRefsRecord => validate_two_refs_boundary_record(value, field_spec, spec, schema_ref),
        BoundaryFieldKind::U64Record => validate_u64_record(value, field_spec.label, spec, schema_ref),
    }
}

const FIELD_ARITY_ZERO: usize = 0;
const FIELD_ARITY_ONE: usize = 1;
const FIELD_ARITY_TWO: usize = 2;
const FIELD_ARITY_THREE: usize = 3;

fn boundary_record<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    arity: usize,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, preserves::Record<Value<IoValue>>>> {
    value.collect_simple_record(label, Some(arity)).ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} must be <{label} ...> with arity {arity} using schema {}",
            spec.family, schema_ref
        ))
    })
}

fn validate_boundary_schema_id(
    value: &Value<IoValue>,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let actual_schema = value.as_string().ok_or_else(|| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: schema field must be a string for schema {}",
            spec.family, schema_ref
        ))
    })?;
    if actual_schema.as_ref() == spec.schema_id {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: unsupported schema {} expected {} using schema {}",
            spec.family,
            actual_schema.as_ref(),
            spec.schema_id,
            schema_ref
        )))
    }
}

fn validate_string_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_string(&record[0], label, spec, schema_ref).map(|_| ())
}

fn validate_non_empty_string_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    if text.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} requires a non-empty string using schema {}",
            spec.family, schema_ref
        )));
    }
    Ok(())
}

fn validate_stable_id_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    validate_stable_id(text.as_ref(), label).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected stable id using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn validate_decision_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let text = ensure_string(&record[0], label, spec, schema_ref)?;
    Decision::parse(text.as_ref()).map(|_| ()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected decision using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn validate_u64_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    record[0]
        .as_u64()
        .ok_or_else(|| boundary_field_error(spec, label, "u64", schema_ref))?
        .map(|_| ())
        .map_err(|error| MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} u64 out of range using schema {}: {error}",
            spec.family, schema_ref
        )))
}

fn validate_ref_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_content_ref(&record[0], label, spec, schema_ref)
}

fn validate_ref_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    validate_ref_sequence_record_with_contract(value, label, spec, schema_ref, false, false)
}

fn validate_ref_sequence_record_with_contract(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
    require_non_empty: bool,
    require_unique: bool,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    if require_non_empty && sequence.is_empty() {
        return Err(MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} requires a non-empty ref sequence using schema {}",
            spec.family, schema_ref
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    for item in sequence.iter() {
        let reference = ensure_content_ref_string(item, label, spec, schema_ref)?;
        if require_unique && !seen.insert(reference.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: field {label} duplicate ref {reference} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_string_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    for item in sequence.iter() {
        ensure_string(item, label, spec, schema_ref)?;
    }
    Ok(())
}

fn validate_unique_string_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let sequence = ensure_sequence(&record[0], label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for item in sequence.iter() {
        let text = ensure_string(item, label, spec, schema_ref)?;
        if !seen.insert(text.to_string()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: field {label} duplicate string {text} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_any_sequence_record(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, label, FIELD_ARITY_ONE, spec, schema_ref)?;
    ensure_sequence(&record[0], label, spec, schema_ref).map(|_| ())
}

fn validate_optional_ref_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let optional = value_to_iovalue(&record[0]);
    if optional.collect_simple_record("none", Some(FIELD_ARITY_ZERO)).is_some() {
        return Ok(());
    }
    if let Some(some) = optional.collect_simple_record("some", Some(FIELD_ARITY_ONE)) {
        return ensure_content_ref(&some[0], field_spec.label, spec, schema_ref);
    }
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)
}

fn validate_checks_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let checks = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for item in checks.iter() {
        let item = value_to_iovalue(item);
        let check = item.collect_simple_record("check", Some(FIELD_ARITY_TWO)).ok_or_else(|| {
            boundary_field_error(spec, field_spec.label, "<check string string>", schema_ref)
        })?;
        let name = ensure_string(&check[0], "check name", spec, schema_ref)?;
        let status = ensure_string(&check[1], "check status", spec, schema_ref)?;
        if !seen.insert(name.to_string()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: duplicate check {name} using schema {}",
                spec.family, schema_ref
            )));
        }
        CheckStatus::parse(status.as_ref()).map_err(|error| {
            MoltenError::invalid_harness(format!(
                "{} schema validation deny: unsupported check status using schema {}: {error}",
                spec.family, schema_ref
            ))
        })?;
    }
    Ok(())
}

fn validate_chain_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let value = value_to_iovalue(value);
    let chain = value.collect_simple_record(field_spec.label, Some(FIELD_ARITY_THREE)).ok_or_else(|| {
        boundary_field_error(spec, field_spec.label, "chain record", schema_ref)
    })?;
    validate_string_record(&chain[0], "scope", spec, schema_ref)?;
    validate_string_record(&chain[1], "id", spec, schema_ref)?;
    validate_string_record(&chain[2], "epoch", spec, schema_ref)
}

fn validate_object_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_string(&record[1], field_spec.label, spec, schema_ref).map(|_| ())
}

fn validate_string_and_ref_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_string(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_content_ref(&record[1], field_spec.label, spec, schema_ref)
}

fn validate_ref_and_string_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_string(&record[1], field_spec.label, spec, schema_ref).map(|_| ())
}

fn validate_two_refs_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_TWO, spec, schema_ref)?;
    ensure_content_ref(&record[0], field_spec.label, spec, schema_ref)?;
    ensure_content_ref(&record[1], field_spec.label, spec, schema_ref)
}

fn validate_file_refs_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let files = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    for file in files.iter() {
        let file = value_to_iovalue(file);
        let fields = file.collect_simple_record("file", Some(FIELD_ARITY_TWO)).ok_or_else(|| {
            boundary_field_error(spec, field_spec.label, "<file string ref>", schema_ref)
        })?;
        ensure_string(&fields[0], "file name", spec, schema_ref)?;
        ensure_content_ref(&fields[1], "file ref", spec, schema_ref)?;
    }
    Ok(())
}

fn validate_conformance_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let value = value_to_iovalue(value);
    let fields = value.collect_simple_record(field_spec.label, Some(FIELD_ARITY_THREE)).ok_or_else(|| {
        boundary_field_error(spec, field_spec.label, "conformance record", schema_ref)
    })?;
    validate_ref_record(&fields[0], "positive", spec, schema_ref)?;
    validate_ref_record(&fields[1], "negative", spec, schema_ref)?;
    validate_ref_record(&fields[2], "property", spec, schema_ref)
}

fn validate_hostcall_descriptors_boundary_record(
    value: &Value<IoValue>,
    field_spec: &BoundaryFieldSpec,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    let record = boundary_record(value, field_spec.label, FIELD_ARITY_ONE, spec, schema_ref)?;
    let descriptors = ensure_sequence(&record[0], field_spec.label, spec, schema_ref)?;
    let mut seen = std::collections::BTreeSet::new();
    for descriptor in descriptors.iter() {
        let identity = validate_hostcall_descriptor_boundary_record(descriptor, spec, schema_ref)?;
        if !seen.insert(identity.clone()) {
            return Err(MoltenError::invalid_harness(format!(
                "{} schema validation deny: duplicate hostcall descriptor {identity} using schema {}",
                spec.family, schema_ref
            )));
        }
    }
    Ok(())
}

fn validate_hostcall_descriptor_boundary_record(
    value: &Value<IoValue>,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value.collect_simple_record("hostcall-descriptor", Some(HOSTCALL_DESCRIPTOR_ARITY)).ok_or_else(|| {
        boundary_field_error(spec, "hostcall-descriptor", "hostcall descriptor", schema_ref)
    })?;
    let operation_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_OPERATION_INDEX], "operation", FIELD_ARITY_ONE, spec, schema_ref)?;
    let operation = ensure_string(&operation_record[0], "operation", spec, schema_ref)?;
    OperationId::parse(operation.as_ref()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: hostcall operation expected operation id using schema {}: {error}",
            spec.family, schema_ref
        ))
    })?;
    let descriptor_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_DESCRIPTOR_INDEX], "descriptor", FIELD_ARITY_ONE, spec, schema_ref)?;
    let descriptor_ref = ensure_content_ref_string(&descriptor_record[0], "descriptor", spec, schema_ref)?;
    validate_ref_record(&fields[HOSTCALL_DESCRIPTOR_INPUT_SCHEMA_INDEX], "input-schema", spec, schema_ref)?;
    validate_ref_record(&fields[HOSTCALL_DESCRIPTOR_OUTPUT_SCHEMA_INDEX], "output-schema", spec, schema_ref)?;
    validate_ref_sequence_record_with_contract(&fields[HOSTCALL_DESCRIPTOR_AUTHORITY_INDEX], "authority", spec, schema_ref, true, true)?;
    validate_ref_sequence_record_with_contract(&fields[HOSTCALL_DESCRIPTOR_RESOURCE_INDEX], "resource", spec, schema_ref, true, true)?;
    validate_ref_sequence_record_with_contract(&fields[HOSTCALL_DESCRIPTOR_EFFECTS_INDEX], "effects", spec, schema_ref, true, true)?;
    let replay_record = boundary_record(&fields[HOSTCALL_DESCRIPTOR_REPLAY_INDEX], "replay", FIELD_ARITY_ONE, spec, schema_ref)?;
    let replay = ensure_string(&replay_record[0], "replay", spec, schema_ref)?;
    ReplayClass::parse(replay.as_ref()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: hostcall replay class unsupported using schema {}: {error}",
            spec.family, schema_ref
        ))
    })?;
    validate_ref_sequence_record_with_contract(&fields[HOSTCALL_DESCRIPTOR_ERRORS_INDEX], "errors", spec, schema_ref, true, true)?;
    Ok(format!("{}:{}", operation.as_ref(), descriptor_ref))
}

const HOSTCALL_DESCRIPTOR_ARITY: usize = 9;
const HOSTCALL_DESCRIPTOR_OPERATION_INDEX: usize = 0;
const HOSTCALL_DESCRIPTOR_DESCRIPTOR_INDEX: usize = 1;
const HOSTCALL_DESCRIPTOR_INPUT_SCHEMA_INDEX: usize = 2;
const HOSTCALL_DESCRIPTOR_OUTPUT_SCHEMA_INDEX: usize = 3;
const HOSTCALL_DESCRIPTOR_AUTHORITY_INDEX: usize = 4;
const HOSTCALL_DESCRIPTOR_RESOURCE_INDEX: usize = 5;
const HOSTCALL_DESCRIPTOR_EFFECTS_INDEX: usize = 6;
const HOSTCALL_DESCRIPTOR_REPLAY_INDEX: usize = 7;
const HOSTCALL_DESCRIPTOR_ERRORS_INDEX: usize = 8;

fn ensure_string<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, str>> {
    value.as_string().ok_or_else(|| boundary_field_error(spec, label, "string", schema_ref))
}

fn ensure_content_ref(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<()> {
    ensure_content_ref_string(value, label, spec, schema_ref).map(|_| ())
}

fn ensure_content_ref_string(
    value: &Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<String> {
    let reference = value
        .as_string()
        .ok_or_else(|| boundary_field_error(spec, label, "canonical content ref string", schema_ref))?;
    ContentRef::parse(reference.as_ref()).map(|_| reference.to_string()).map_err(|error| {
        MoltenError::invalid_harness(format!(
            "{} schema validation deny: field {label} expected canonical content ref string using schema {}: {error}",
            spec.family, schema_ref
        ))
    })
}

fn ensure_sequence<'a>(
    value: &'a Value<IoValue>,
    label: &str,
    spec: &BoundarySchemaSpec,
    schema_ref: &ContentRef,
) -> Result<std::borrow::Cow<'a, [Value<IoValue>]>> {
    value
        .collect_sequence()
        .map(|sequence| match sequence {
            std::borrow::Cow::Borrowed(values) => std::borrow::Cow::Borrowed(values.as_slice()),
            std::borrow::Cow::Owned(values) => std::borrow::Cow::Owned(values),
        })
        .ok_or_else(|| boundary_field_error(spec, label, "sequence", schema_ref))
}

fn boundary_field_error(
    spec: &BoundarySchemaSpec,
    label: &str,
    expected: &str,
    schema_ref: &ContentRef,
) -> MoltenError {
    MoltenError::invalid_harness(format!(
        "{} schema validation deny: field {label} expected {expected} using schema {}",
        spec.family, schema_ref
    ))
}

pub const DEFAULT_STRUCTURAL_SCAN_MAX_NODES: usize = 8_192;
pub const DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH: usize = 128;
pub const SENSITIVE_STRUCTURAL_MARKERS: &[&str] = &["secret", "confidential", "credential", "private", "encrypted-ref"];
pub const AMBIENT_JOB_TOKENS: &[&str] = &[
    "mobile-code",
    "raw-closure",
    "closure",
    "host-path",
    "source-path",
    "source-registry",
    "process-command",
    "command",
    "env",
    "environment",
    "source-text",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuralTokenKind {
    RecordLabel,
    Symbol,
    String,
    ByteString,
    ContentRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuralMatch {
    pub kind: StructuralTokenKind,
    pub token: String,
    pub path: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralInspectionScope {
    pub record_labels: bool,
    pub symbols: bool,
    pub strings: bool,
    pub byte_strings: bool,
    pub content_refs: bool,
}

impl StructuralInspectionScope {
    pub const fn structural_markers() -> Self {
        Self {
            record_labels: true,
            symbols: true,
            strings: false,
            byte_strings: false,
            content_refs: false,
        }
    }

    pub const fn content_refs() -> Self {
        Self {
            record_labels: false,
            symbols: false,
            strings: false,
            byte_strings: false,
            content_refs: true,
        }
    }

    pub const fn all() -> Self {
        Self {
            record_labels: true,
            symbols: true,
            strings: true,
            byte_strings: true,
            content_refs: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructuralInspectionLimits {
    pub max_nodes: usize,
    pub max_depth: usize,
}

impl Default for StructuralInspectionLimits {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_STRUCTURAL_SCAN_MAX_NODES,
            max_depth: DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH,
        }
    }
}

struct StructuralScanState {
    visited_nodes: usize,
    limits: StructuralInspectionLimits,
}

// r[impl molten.preserves_value_inspection.structural_scan]
pub fn find_structural_match<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    find_structural_match_with_limits(value, scope, StructuralInspectionLimits::default(), predicate)
}

pub fn find_structural_match_with_limits<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    limits: StructuralInspectionLimits,
    mut predicate: F,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    let mut state = StructuralScanState {
        visited_nodes: 0,
        limits,
    };
    let mut path = vec!["$".to_string()];
    visit_structural_value(value, scope, &mut predicate, &mut state, &mut path)
}

pub fn find_named_structural_marker(value: &IoValue, markers: &[&str]) -> Result<Option<StructuralMatch>> {
    find_structural_match(value, StructuralInspectionScope::structural_markers(), |kind, token| {
        matches!(kind, StructuralTokenKind::RecordLabel | StructuralTokenKind::Symbol) && markers.contains(&token)
    })
}

// r[impl molten.preserves_value_inspection.marker_detection]
pub fn find_sensitive_structural_marker(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, SENSITIVE_STRUCTURAL_MARKERS)
}

// r[impl molten.preserves_value_inspection.ambient_token_denial]
pub fn find_ambient_job_token(value: &IoValue) -> Result<Option<StructuralMatch>> {
    find_named_structural_marker(value, AMBIENT_JOB_TOKENS)
}

// r[impl molten.preserves_value_inspection.ref_retention]
pub fn find_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<Option<StructuralMatch>> {
    let target = ContentRef::parse(target_ref)?;
    find_structural_match(value, StructuralInspectionScope::content_refs(), |kind, token| {
        kind == StructuralTokenKind::ContentRef && token == target.as_str()
    })
}

pub fn contains_structural_content_ref(value: &IoValue, target_ref: &str) -> Result<bool> {
    Ok(find_structural_content_ref(value, target_ref)?.is_some())
}

fn visit_structural_value<F>(
    value: &IoValue,
    scope: StructuralInspectionScope,
    predicate: &mut F,
    state: &mut StructuralScanState,
    path: &mut Vec<String>,
) -> Result<Option<StructuralMatch>>
where
    F: FnMut(StructuralTokenKind, &str) -> bool,
{
    state.visited_nodes = state
        .visited_nodes
        .checked_add(1)
        .ok_or_else(|| MoltenError::invalid_harness("structural Preserves scan node count overflow"))?;
    if state.visited_nodes > state.limits.max_nodes {
        return Err(MoltenError::invalid_harness(format!(
            "structural Preserves scan exceeded {} nodes",
            state.limits.max_nodes
        )));
    }
    if path.len() > state.limits.max_depth {
        return Err(MoltenError::invalid_harness(format!(
            "structural Preserves scan exceeded depth {}",
            state.limits.max_depth
        )));
    }

    if value.is_record() {
        let label = value.label();
        if let Some(name) = label.as_symbol()
            && scope.record_labels
            && predicate(StructuralTokenKind::RecordLabel, name.as_ref())
        {
            return Ok(Some(structural_match(StructuralTokenKind::RecordLabel, name.as_ref(), path)));
        }
        path.push("label".to_string());
        if let Some(found) = visit_structural_value(&value_to_iovalue(&label), scope, predicate, state, path)? {
            return Ok(Some(found));
        }
        path.pop();
        for (index, child) in value.iter().enumerate() {
            path.push(format!("field[{index}]"));
            if let Some(found) = visit_structural_value(&value_to_iovalue(&child), scope, predicate, state, path)? {
                return Ok(Some(found));
            }
            path.pop();
        }
        return Ok(None);
    }

    if let Some(symbol) = value.as_symbol()
        && scope.symbols
        && predicate(StructuralTokenKind::Symbol, symbol.as_ref())
    {
        return Ok(Some(structural_match(StructuralTokenKind::Symbol, symbol.as_ref(), path)));
    }
    if let Some(text) = value.as_string() {
        if scope.strings && predicate(StructuralTokenKind::String, text.as_ref()) {
            return Ok(Some(structural_match(StructuralTokenKind::String, text.as_ref(), path)));
        }
        if scope.content_refs && ContentRef::parse(text.as_ref()).is_ok()
            && predicate(StructuralTokenKind::ContentRef, text.as_ref())
        {
            return Ok(Some(structural_match(StructuralTokenKind::ContentRef, text.as_ref(), path)));
        }
    }
    if let Some(bytes) = value.as_bytestring()
        && scope.byte_strings
    {
        let token = content_ref_from_bytes(bytes.as_ref());
        if predicate(StructuralTokenKind::ByteString, &token) {
            return Ok(Some(structural_match(StructuralTokenKind::ByteString, &token, path)));
        }
    }

    if value.is_sequence() || value.is_set() {
        for (index, child) in value.iter().enumerate() {
            path.push(format!("item[{index}]"));
            if let Some(found) = visit_structural_value(&value_to_iovalue(&child), scope, predicate, state, path)? {
                return Ok(Some(found));
            }
            path.pop();
        }
        return Ok(None);
    }

    if value.is_dictionary() {
        for (index, (key, child)) in value.entries().enumerate() {
            path.push(format!("entry[{index}].key"));
            if let Some(found) = visit_structural_value(&value_to_iovalue(&key), scope, predicate, state, path)? {
                return Ok(Some(found));
            }
            path.pop();
            path.push(format!("entry[{index}].value"));
            if let Some(found) = visit_structural_value(&value_to_iovalue(&child), scope, predicate, state, path)? {
                return Ok(Some(found));
            }
            path.pop();
        }
    }

    Ok(None)
}

fn structural_match(kind: StructuralTokenKind, token: &str, path: &[String]) -> StructuralMatch {
    StructuralMatch {
        kind,
        token: token.to_string(),
        path: path.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    const ANNOTATED_ONE_PACKED: &[u8] = b"\x85\xb0\x01\x02\xb0\x01\x01";
    const TRAILING_SENTINEL_BYTE: u8 = 0;
    const TAMPER_MASK: u8 = 1;

    #[test]
    fn preserves_text_roundtrip_keeps_hash() {
        let value = super::parse_text("<example \"a\" [1 2 3]>").expect("parse initial text");
        let hash = super::canonical_hash(&value).expect("hash initial value");
        let rendered = super::to_text(&value).expect("render preserves text");
        let reparsed = super::parse_text(&rendered).expect("parse rendered text");
        assert_eq!(hash, super::canonical_hash(&reparsed).expect("hash reparsed value"));
    }

    #[test]
    fn strict_canonical_decode_accepts_molten_canonical_bytes() {
        // r[verify molten.preserves_canonical_bytes.strict_decode]
        let value = super::parse_text("<strict-decode-fixture [#t 42]>").expect("parse fixture");
        let bytes = super::canonical_bytes(&value).expect("canonical bytes");
        let decoded = super::strict_canonical_decode(&bytes).expect("strict decode");
        assert_eq!(decoded.value, value);
        assert_eq!(decoded.canonical_bytes, bytes);
        assert_eq!(decoded.value_ref.as_str(), super::canonical_hash(&decoded.value).expect("decoded hash"));
    }

    #[test]
    fn strict_canonical_decode_rejects_annotated_trailing_truncated_and_tampered_bytes() {
        // r[verify molten.preserves_canonical_bytes.noncanonical_denial]
        let annotated_error = super::strict_canonical_decode(ANNOTATED_ONE_PACKED)
            .expect_err("annotations are parseable but not canonical without annotations");
        assert!(annotated_error.to_string().contains("strict canonical Preserves decode failed"));

        let value = super::parse_text("<strict-decode-fixture \"payload\">").expect("parse fixture");
        let mut trailing = super::canonical_bytes(&value).expect("canonical bytes");
        trailing.push(TRAILING_SENTINEL_BYTE);
        assert!(super::strict_canonical_decode(&trailing).is_err());

        let mut truncated = super::canonical_bytes(&value).expect("canonical bytes");
        truncated.pop();
        assert!(super::strict_canonical_decode(&truncated).is_err());

        let original = super::canonical_bytes(&value).expect("canonical bytes");
        let original_ref = super::content_ref_from_bytes(&original);
        let mut tampered = original.clone();
        let first = tampered.first_mut().expect("non-empty canonical bytes");
        *first ^= TAMPER_MASK;
        assert!(super::strict_canonical_decode_with_ref(&tampered, &original_ref, "tampered-fixture").is_err());
    }

    fn boundary_schema_specs() -> Vec<&'static super::BoundarySchemaSpec> {
        vec![
            &super::NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA,
            &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA,
            &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA,
            &super::RETENTION_RECEIPT_BOUNDARY_SCHEMA,
            &super::EVIDENCE_CHAIN_SEGMENT_BUNDLE_BOUNDARY_SCHEMA,
            &super::OPERATOR_RELEASE_EVIDENCE_BUNDLE_BOUNDARY_SCHEMA,
        ]
    }

    fn boundary_test_ref(label: &str) -> String {
        super::content_ref_from_bytes(format!("boundary-schema-test-{label}").as_bytes())
    }

    fn boundary_schema_fixture(spec: &super::BoundarySchemaSpec) -> preserves::IOValue {
        super::record(
            spec.record_label,
            spec.fields
                .iter()
                .map(|field| boundary_field_fixture(spec, field))
                .collect(),
        )
    }

    fn boundary_field_fixture(
        spec: &super::BoundarySchemaSpec,
        field: &super::BoundaryFieldSpec,
    ) -> preserves::IOValue {
        match field.kind {
            super::BoundaryFieldKind::SchemaId => super::string(spec.schema_id),
            super::BoundaryFieldKind::AnyRecord => {
                super::record(field.label, vec![super::record("payload", vec![super::string("value")])])
            }
            super::BoundaryFieldKind::AnySequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("artifact", vec![super::string(boundary_test_ref(field.label))]),
            ])]),
            super::BoundaryFieldKind::ChainRecord => super::record("chain", vec![
                super::record("scope", vec![super::string("test-scope")]),
                super::record("id", vec![super::string("test-id")]),
                super::record("epoch", vec![super::string("test-epoch")]),
            ]),
            super::BoundaryFieldKind::ChecksRecord => super::checks_value(&[("schema-bound", "pass")]),
            super::BoundaryFieldKind::ConformanceRecord => super::record(field.label, vec![
                super::record("positive", vec![super::string(boundary_test_ref("positive"))]),
                super::record("negative", vec![super::string(boundary_test_ref("negative"))]),
                super::record("property", vec![super::string(boundary_test_ref("property"))]),
            ]),
            super::BoundaryFieldKind::DecisionRecord => super::record(field.label, vec![super::string("pass")]),
            super::BoundaryFieldKind::FileRefsRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("file", vec![
                    super::string("member.txt"),
                    super::string(boundary_test_ref("member")),
                ]),
            ])]),
            super::BoundaryFieldKind::HostcallDescriptorsRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("hostcall-descriptor", vec![
                    super::record("operation", vec![super::string("storage.read")]),
                    super::record("descriptor", vec![super::string(boundary_test_ref("descriptor"))]),
                    super::record("input-schema", vec![super::string(boundary_test_ref("input-schema"))]),
                    super::record("output-schema", vec![super::string(boundary_test_ref("output-schema"))]),
                    super::record("authority", vec![super::sequence(vec![super::string(boundary_test_ref("authority"))])]),
                    super::record("resource", vec![super::sequence(vec![super::string(boundary_test_ref("resource"))])]),
                    super::record("effects", vec![super::sequence(vec![super::string(boundary_test_ref("effects"))])]),
                    super::record("replay", vec![super::string("deterministic")]),
                    super::record("errors", vec![super::sequence(vec![super::string(boundary_test_ref("errors"))])]),
                ]),
            ])]),
            super::BoundaryFieldKind::NonEmptyRefSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(boundary_test_ref(field.label)),
            ])]),
            super::BoundaryFieldKind::NonEmptyStringRecord => {
                super::record(field.label, vec![super::string(format!("{}-value", field.label))])
            }
            super::BoundaryFieldKind::ObjectRecord => super::record(field.label, vec![
                super::string(boundary_test_ref("object")),
                super::string("artifact"),
            ]),
            super::BoundaryFieldKind::OptionalRefRecord => super::record(field.label, vec![super::record(
                "some",
                vec![super::string(boundary_test_ref(field.label))],
            )]),
            super::BoundaryFieldKind::RefAndStringRecord => super::record(field.label, vec![
                super::string(boundary_test_ref(field.label)),
                super::string(format!("{}-path", field.label)),
            ]),
            super::BoundaryFieldKind::RefRecord => {
                super::record(field.label, vec![super::string(boundary_test_ref(field.label))])
            }
            super::BoundaryFieldKind::RefSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(boundary_test_ref(field.label)),
            ])]),
            super::BoundaryFieldKind::StableIdRecord => {
                super::record(field.label, vec![super::string(format!("{}-value", field.label))])
            }
            super::BoundaryFieldKind::StringAndRefRecord => super::record(field.label, vec![
                super::string(format!("{}-value", field.label)),
                super::string(boundary_test_ref(field.label)),
            ]),
            super::BoundaryFieldKind::StringRecord => {
                super::record(field.label, vec![super::string(format!("{}-value", field.label))])
            }
            super::BoundaryFieldKind::StringSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(format!("{}-item", field.label)),
            ])]),
            super::BoundaryFieldKind::UniqueRefSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(boundary_test_ref(field.label)),
            ])]),
            super::BoundaryFieldKind::UniqueStringSequenceRecord => super::record(field.label, vec![super::sequence(vec![
                super::string(format!("{}-item", field.label)),
            ])]),
            super::BoundaryFieldKind::TwoRefsRecord => super::record(field.label, vec![
                super::string(boundary_test_ref(&format!("{}-a", field.label))),
                super::string(boundary_test_ref(&format!("{}-b", field.label))),
            ]),
            super::BoundaryFieldKind::U64Record => super::record(field.label, vec![super::u64_value(1)]),
        }
    }

    fn boundary_fixture_fields(
        value: &preserves::IOValue,
        spec: &super::BoundarySchemaSpec,
    ) -> Vec<preserves::IOValue> {
        let record = value
            .collect_simple_record(spec.record_label, Some(spec.arity()))
            .expect("boundary fixture record");
        let mut fields = Vec::with_capacity(spec.arity());
        for index in 0..spec.arity() {
            fields.push(super::value_to_iovalue(&record[index]));
        }
        fields
    }

    fn boundary_fixture_with_field(
        spec: &super::BoundarySchemaSpec,
        field_index: usize,
        replacement: preserves::IOValue,
    ) -> preserves::IOValue {
        let fixture = boundary_schema_fixture(spec);
        let mut fields = boundary_fixture_fields(&fixture, spec);
        fields[field_index] = replacement;
        super::record(spec.record_label, fields)
    }

    fn boundary_field_index(
        spec: &super::BoundarySchemaSpec,
        predicate: impl Fn(&super::BoundaryFieldSpec) -> bool,
    ) -> usize {
        spec.fields.iter().position(predicate).expect("matching boundary field")
    }

    fn malformed_ref_field(field: &super::BoundaryFieldSpec) -> preserves::IOValue {
        match field.kind {
            super::BoundaryFieldKind::RefRecord => super::record(field.label, vec![super::sequence(Vec::new())]),
            super::BoundaryFieldKind::RefSequenceRecord
            | super::BoundaryFieldKind::NonEmptyRefSequenceRecord
            | super::BoundaryFieldKind::UniqueRefSequenceRecord => {
                super::record(field.label, vec![super::sequence(vec![super::sequence(Vec::new())])])
            }
            super::BoundaryFieldKind::OptionalRefRecord => {
                super::record(field.label, vec![super::record("some", vec![super::sequence(Vec::new())])])
            }
            super::BoundaryFieldKind::StringAndRefRecord => super::record(field.label, vec![
                super::string("name"),
                super::sequence(Vec::new()),
            ]),
            super::BoundaryFieldKind::RefAndStringRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string("name"),
            ]),
            super::BoundaryFieldKind::TwoRefsRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string(boundary_test_ref(field.label)),
            ]),
            super::BoundaryFieldKind::FileRefsRecord => super::record(field.label, vec![super::sequence(vec![
                super::record("file", vec![super::string("member.txt"), super::sequence(Vec::new())]),
            ])]),
            super::BoundaryFieldKind::ObjectRecord => super::record(field.label, vec![
                super::sequence(Vec::new()),
                super::string("artifact"),
            ]),
            _ => super::sequence(Vec::new()),
        }
    }

    fn is_ref_bearing_field(field: &super::BoundaryFieldSpec) -> bool {
        matches!(
            field.kind,
            super::BoundaryFieldKind::FileRefsRecord
                | super::BoundaryFieldKind::ObjectRecord
                | super::BoundaryFieldKind::OptionalRefRecord
                | super::BoundaryFieldKind::RefAndStringRecord
                | super::BoundaryFieldKind::NonEmptyRefSequenceRecord
                | super::BoundaryFieldKind::RefRecord
                | super::BoundaryFieldKind::RefSequenceRecord
                | super::BoundaryFieldKind::StringAndRefRecord
                | super::BoundaryFieldKind::UniqueRefSequenceRecord
                | super::BoundaryFieldKind::TwoRefsRecord
        )
    }

    #[test]
    fn boundary_schema_adapter_accepts_valid_versioned_records_for_all_adopted_families() {
        // r[verify molten.preserves_schema_boundaries.schema_adapter]
        // r[verify molten.preserves_boundary_field_contracts.field_contracts]
        for spec in boundary_schema_specs() {
            let value = boundary_schema_fixture(spec);
            let validation = super::validate_boundary_schema(&value, spec).expect("schema validation");
            assert_eq!(validation.decision, "pass");
            assert!(validation.schema_ref.as_str().starts_with("blake3:"));
            assert_eq!(validation.value_ref.as_str(), super::canonical_hash(&value).expect("value ref"));
            let diagnostic = super::boundary_schema_diagnostic_value(&validation);
            let diagnostic_text = super::to_text(&diagnostic).expect("diagnostic text");
            assert!(diagnostic_text.contains(spec.family));
            assert!(diagnostic_text.contains("schema-ref"));
        }
    }

    #[test]
    fn boundary_codec_report_binds_strict_decode_schema_and_typed_refs() {
        // r[verify molten.preserves_schema_boundaries.schema_artifacts]
        // r[verify molten.preserves_boundary_codegen.typed_codecs]
        // r[verify molten.preserves_boundary_codegen.strict_decode]
        // r[verify molten.preserves_boundary_codegen.schema_ref_evidence]
        // r[verify molten.preserves_boundary_codegen.fixture_corpus]
        let spec = &super::NODE_CONTROL_INGRESS_BOUNDARY_SCHEMA;
        let value = boundary_schema_fixture(spec);
        let bytes = super::canonical_bytes(&value).expect("canonical boundary fixture bytes");
        let report = super::validate_boundary_bytes(&bytes, spec).expect("boundary codec report");
        assert_eq!(report.decision, "pass");
        assert_eq!(report.schema_ref, super::boundary_schema_ref(spec).expect("schema ref"));
        assert_eq!(report.input_bytes_ref.as_str(), super::content_ref_from_bytes(&bytes));
        assert_eq!(report.decoded_value_ref.as_str(), super::canonical_hash(&value).expect("value ref"));
        assert!(report.typed_value_ref.as_str().starts_with("blake3:"));
        let rendered = super::to_text(&super::boundary_codec_report_value(&report)).expect("report text");
        assert!(rendered.contains("typed-value-ref"));
        assert!(rendered.contains(spec.family));
    }

    #[test]
    fn boundary_codec_report_denies_malformed_canonical_records_before_side_effects() {
        // r[verify molten.preserves_boundary_codegen.fixture_corpus]
        let spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let malformed = boundary_fixture_with_field(spec, SCHEMA_FIELD_INDEX, super::string("unsupported.schema.v0"));
        let bytes = super::canonical_bytes(&malformed).expect("canonical malformed fixture bytes");
        let report = super::validate_boundary_bytes(&bytes, spec).expect("canonical malformed report");
        assert_eq!(report.decision, "deny");
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("unsupported schema")));

        let mut non_canonical = bytes.clone();
        non_canonical.push(TRAILING_SENTINEL_BYTE);
        assert!(super::validate_boundary_bytes(&non_canonical, spec).is_err());
    }

    #[test]
    fn boundary_schema_adapter_denies_malformed_records_for_all_adopted_families() {
        // r[verify molten.preserves_schema_boundaries.schema_denials]
        for spec in boundary_schema_specs() {
            let fixture = boundary_schema_fixture(spec);
            let fields = boundary_fixture_fields(&fixture, spec);
            let wrong_label = super::record("wrong-label", fields.clone());
            assert!(super::validate_boundary_schema(&wrong_label, spec).is_err());

            let mut missing_fields = fields.clone();
            missing_fields.pop();
            let missing = super::record(spec.record_label, missing_fields);
            assert!(super::validate_boundary_schema(&missing, spec).is_err());

            let wrong_schema_type = boundary_fixture_with_field(spec, SCHEMA_FIELD_INDEX, super::u64_value(1));
            assert!(super::validate_boundary_schema(&wrong_schema_type, spec).is_err());

            let wrong_version = boundary_fixture_with_field(
                spec,
                SCHEMA_FIELD_INDEX,
                super::string("unsupported.schema.v0"),
            );
            assert!(super::validate_boundary_schema(&wrong_version, spec).is_err());

            let mut extra_fields = fields.clone();
            extra_fields.push(super::record("extra-critical", vec![super::string("deny")]));
            let extra = super::record(spec.record_label, extra_fields);
            assert!(super::validate_boundary_schema(&extra, spec).is_err());

            let checks_index = boundary_field_index(spec, |field| {
                matches!(field.kind, super::BoundaryFieldKind::ChecksRecord)
            });
            let malformed_checks = boundary_fixture_with_field(spec, checks_index, super::record(
                "checks",
                vec![super::sequence(vec![super::record("check", vec![super::string("missing-status")])])],
            ));
            assert!(super::validate_boundary_schema(&malformed_checks, spec).is_err());

            let ref_index = boundary_field_index(spec, is_ref_bearing_field);
            let malformed_ref = boundary_fixture_with_field(spec, ref_index, malformed_ref_field(&spec.fields[ref_index]));
            assert!(super::validate_boundary_schema(&malformed_ref, spec).is_err());
        }
    }

    #[test]
    fn boundary_schema_ref_binds_field_labels_kinds_and_constraints() {
        const TEST_SCHEMA_FIELD_COUNT: usize = 2;
        const BASE_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::StringRecord },
        ];
        const LABEL_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload-renamed", kind: super::BoundaryFieldKind::StringRecord },
        ];
        const KIND_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::RefRecord },
        ];
        const CONSTRAINT_DRIFT_FIELDS: [super::BoundaryFieldSpec; TEST_SCHEMA_FIELD_COUNT] = [
            super::BoundaryFieldSpec { label: "schema-id", kind: super::BoundaryFieldKind::SchemaId },
            super::BoundaryFieldSpec { label: "payload", kind: super::BoundaryFieldKind::NonEmptyStringRecord },
        ];
        let base = test_schema_spec("contract-ref-base", &BASE_FIELDS);
        let label_drift = test_schema_spec("contract-ref-base", &LABEL_DRIFT_FIELDS);
        let kind_drift = test_schema_spec("contract-ref-base", &KIND_DRIFT_FIELDS);
        let constraint_drift = test_schema_spec("contract-ref-base", &CONSTRAINT_DRIFT_FIELDS);
        let base_ref = super::boundary_schema_ref(&base).expect("base schema ref");
        assert_ne!(base_ref, super::boundary_schema_ref(&label_drift).expect("label drift ref"));
        assert_ne!(base_ref, super::boundary_schema_ref(&kind_drift).expect("kind drift ref"));
        assert_ne!(base_ref, super::boundary_schema_ref(&constraint_drift).expect("constraint drift ref"));
        let artifact_text = super::to_text(&super::boundary_schema_artifact_value(&base).expect("artifact"))
            .expect("artifact text");
        assert!(artifact_text.contains("fields"));
        assert!(artifact_text.contains("constraints"));
    }

    #[test]
    fn boundary_validation_reports_stale_claimed_schema_ref() {
        let spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let stale = boundary_test_ref("old-schema-ref");
        let error = super::validate_boundary_claimed_schema_ref(spec, &stale)
            .expect_err("stale schema ref denies");
        assert!(error.to_string().contains(spec.family));
        assert!(error.to_string().contains("stale schema ref"));
    }

    #[test]
    fn boundary_field_contracts_reject_invalid_domains_and_duplicates() {
        // r[verify molten.preserves_boundary_field_contracts.field_contract_denials]
        let plugin_spec = &super::PLUGIN_HOSTCALL_RECEIPT_BOUNDARY_SCHEMA;
        let decision_index = boundary_field_index(plugin_spec, |field| field.label == "decision");
        let bad_decision = boundary_fixture_with_field(
            plugin_spec,
            decision_index,
            super::record("decision", vec![super::string("maybe")]),
        );
        assert!(super::validate_boundary_schema(&bad_decision, plugin_spec).is_err());

        let extension_spec = &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA;
        let policy_index = boundary_field_index(extension_spec, |field| field.label == "policy");
        let empty_policy = boundary_fixture_with_field(
            extension_spec,
            policy_index,
            super::record("policy", vec![super::sequence(Vec::new())]),
        );
        assert!(super::validate_boundary_schema(&empty_policy, extension_spec).is_err());

        let retention_spec = &super::RETENTION_RECEIPT_BOUNDARY_SCHEMA;
        let pins_index = boundary_field_index(retention_spec, |field| field.label == "pins");
        let duplicated_ref = boundary_test_ref("duplicate-pin");
        let duplicate_pins = boundary_fixture_with_field(
            retention_spec,
            pins_index,
            super::record("pins", vec![super::sequence(vec![
                super::string(&duplicated_ref),
                super::string(&duplicated_ref),
            ])]),
        );
        assert!(super::validate_boundary_schema(&duplicate_pins, retention_spec).is_err());

        let extension_spec = &super::PLUGIN_EXTENSION_CONTRACT_BOUNDARY_SCHEMA;
        let hostcalls_index = boundary_field_index(extension_spec, |field| field.label == "hostcalls");
        let hostcalls = boundary_field_fixture(extension_spec, &extension_spec.fields[hostcalls_index]);
        let hostcall_record = hostcalls
            .collect_simple_record("hostcalls", Some(1))
            .expect("hostcalls record");
        let descriptors = hostcall_record[0].collect_sequence().expect("descriptor sequence");
        let first = super::value_to_iovalue(&descriptors[0]);
        let duplicate_hostcalls = boundary_fixture_with_field(
            extension_spec,
            hostcalls_index,
            super::record("hostcalls", vec![super::sequence(vec![first.clone(), first])]),
        );
        assert!(super::validate_boundary_schema(&duplicate_hostcalls, extension_spec).is_err());
    }

    fn test_schema_spec(
        family: &'static str,
        fields: &'static [super::BoundaryFieldSpec],
    ) -> super::BoundarySchemaSpec {
        super::BoundarySchemaSpec {
            family,
            version: "v1",
            record_label: "test-boundary-v1",
            schema_id: "molten.test-boundary.v1",
            fields,
        }
    }

    const SCHEMA_FIELD_INDEX: usize = 0;

    #[test]
    fn structural_scan_detects_nested_markers_without_scanning_rendered_strings() {
        // r[verify molten.preserves_value_inspection.structural_scan]
        // r[verify molten.preserves_value_inspection.marker_detection]
        let nested = super::record("outer", vec![
            super::sequence(vec![super::record("secret", vec![super::string("payload")])]),
            super::string("<credential \"looks rendered but is inert\">"),
        ]);
        let marker = super::find_sensitive_structural_marker(&nested)
            .expect("scan nested")
            .expect("sensitive marker");
        assert_eq!(marker.kind, super::StructuralTokenKind::RecordLabel);
        assert_eq!(marker.token, "secret");

        let inert = super::record("outer", vec![super::string("<secret \"looks rendered but is inert\">")]);
        assert!(
            super::find_sensitive_structural_marker(&inert)
                .expect("scan inert")
                .is_none(),
            "rendered-looking strings are diagnostics, not structural markers"
        );
    }

    #[test]
    fn structural_scan_finds_nested_content_refs() {
        // r[verify molten.preserves_value_inspection.ref_retention]
        let target = super::content_ref_from_bytes(b"structural-content-ref-target");
        let value = super::record("outer", vec![
            super::sequence(vec![super::record("metadata", vec![super::string(&target)])]),
            super::string("blake3:not-a-valid-ref"),
        ]);
        let found = super::find_structural_content_ref(&value, &target)
            .expect("scan content ref")
            .expect("content ref match");
        assert_eq!(found.kind, super::StructuralTokenKind::ContentRef);
        assert_eq!(found.token, target);
    }

    #[test]
    fn structural_scan_reports_bounds() {
        let value = super::record("outer", vec![super::record("inner", Vec::new())]);
        let error = super::find_structural_match_with_limits(
            &value,
            super::StructuralInspectionScope::all(),
            super::StructuralInspectionLimits {
                max_nodes: 1,
                max_depth: super::DEFAULT_STRUCTURAL_SCAN_MAX_DEPTH,
            },
            |_, _| false,
        )
        .expect_err("bounded scan should fail");
        assert!(error.to_string().contains("structural Preserves scan exceeded"));
    }

    #[test]
    fn parser_toolkit_builds_and_parses_common_ref_shapes() {
        const MAX_REFS: usize = 4;
        let first = super::content_ref_from_bytes(b"toolkit-first-ref");
        let second = super::content_ref_from_bytes(b"toolkit-second-ref");
        let record = super::record("refs", vec![super::refs_sequence(&[first.clone(), second.clone()])]);
        let refs = super::record_content_ref_strings(&preserves::Value::from(record), "refs", "toolkit refs", MAX_REFS)
            .expect("parse refs");
        assert_eq!(refs, vec![first, second]);

        let some = super::optional_ref_value(refs.first().map(String::as_str));
        assert_eq!(
            super::optional_content_ref_string(&preserves::Value::from(some), "optional ref")
                .expect("optional ref"),
            refs.first().cloned()
        );
    }

    #[test]
    fn parser_toolkit_rejects_malformed_shapes_and_checks() {
        // r[verify molten.preserves_rail_toolkit.parser_builders]
        // r[verify molten.preserves_rail_toolkit.check_sets]
        // r[verify molten.preserves_rail_toolkit.negative_shapes]
        const MAX_REFS: usize = 4;
        let wrong_label = super::record("wrong", Vec::new());
        assert!(super::simple_record_fields(&wrong_label, "expected", 0).is_err());

        let wrong_arity = super::record("expected", vec![super::string("extra")]);
        assert!(super::simple_record_fields(&wrong_arity, "expected", 0).is_err());

        let wrong_type = preserves::Value::from(super::u64_value(1));
        assert!(super::required_string_field(&wrong_type, "string field").is_err());

        let invalid_ref_record = super::record("refs", vec![super::sequence(vec![super::string("blake3:not-valid")])]);
        assert!(
            super::record_content_ref_strings(
                &preserves::Value::from(invalid_ref_record),
                "refs",
                "toolkit refs",
                MAX_REFS,
            )
            .is_err()
        );

        let checks = super::checks_value(&[("shape", "pass")]);
        let parsed = super::parse_checks_record(&preserves::Value::from(checks), MAX_REFS, "toolkit")
            .expect("parse checks");
        assert!(super::require_checks_present(&parsed, &["missing"], "toolkit").is_err());

        let duplicate = super::checks_value(&[("shape", "pass"), ("shape", "pass")]);
        assert!(super::parse_checks_record(&preserves::Value::from(duplicate), MAX_REFS, "toolkit").is_err());

        let unsupported = super::checks_value(&[("shape", "unknown")]);
        assert!(super::parse_checks_record(&preserves::Value::from(unsupported), MAX_REFS, "toolkit").is_err());
    }

    #[test]
    fn content_ref_parser_rejects_non_canonical_shapes() {
        let valid = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        super::validate_content_ref(valid).expect("valid ref");
        let parsed = super::ContentRef::parse(valid).expect("parsed ref");
        assert_eq!(parsed.as_str(), valid);
        assert_eq!(parsed.into_string(), valid);
        assert_eq!(
            super::content_ref_from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
                .expect("ref from hex"),
            valid
        );

        for invalid in [
            "",
            "blake3:",
            "blake3:fixture",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef0",
            "blake3:0123456789ABCDEF0123456789abcdef0123456789abcdef0123456789abcdef",
            "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
            "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde/",
        ] {
            assert!(super::validate_content_ref(invalid).is_err(), "invalid ref accepted: {invalid}");
        }
    }

    #[test]
    fn typed_domain_newtypes_parse_format_and_reject_invalid_values() {
        let stable = super::StableId::parse("node:alpha-1").expect("stable id");
        assert_eq!(stable.as_str(), "node:alpha-1");
        assert_eq!(stable.clone().into_string(), "node:alpha-1");
        assert_eq!(super::SchemaId::parse("molten.schema.v1").expect("schema id").as_str(), "molten.schema.v1");
        assert_eq!(super::OperationId::parse("storage.read").expect("operation id").as_str(), "storage.read");
        assert_eq!(super::ProfileId::parse("production").expect("profile id").as_str(), "production");
        assert_eq!(super::Decision::parse("pass").expect("decision").as_str(), "pass");
        assert_eq!(super::CheckStatus::parse("diagnostic").expect("check status").as_str(), "diagnostic");
        assert_eq!(super::ReplayClass::parse("deterministic").expect("replay class").as_str(), "deterministic");

        assert!(super::StableId::parse("").is_err());
        assert!(super::StableId::parse("bad/id").is_err());
        assert!(super::OperationId::parse("Storage.read").is_err());
        assert!(super::Decision::parse("maybe").is_err());
        assert!(super::CheckStatus::parse("unknown").is_err());
        assert!(super::ReplayClass::parse("nondeterministic").is_err());
    }

    #[test]
    fn content_ref_serde_preserves_wire_string_and_rejects_invalid_input() {
        let valid = "blake3:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        let parsed = super::ContentRef::parse(valid).expect("valid content ref");
        let rendered = serde_json::to_string(&parsed).expect("serialized ref");
        assert_eq!(rendered, format!("\"{valid}\""));
        let decoded: super::ContentRef = serde_json::from_str(&rendered).expect("decoded ref");
        assert_eq!(decoded, parsed);
        assert_eq!(decoded.to_string(), valid);
        assert!(serde_json::from_str::<super::ContentRef>("\"blake3:not-hex\"").is_err());
        assert!(serde_json::from_str::<super::ContentRef>("\"/tmp/not-a-ref\"").is_err());
    }

    #[test]
    fn canonical_content_ref_matches_canonical_hash() {
        let value = super::parse_text("<content-ref-fixture [#t 42]>").expect("parse fixture");
        let reference = super::canonical_content_ref(&value).expect("canonical content ref");
        assert_eq!(reference.as_str(), super::canonical_hash(&value).expect("canonical hash"));
    }
}
