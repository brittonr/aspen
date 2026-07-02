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
pub const ARTIFACT_RECEIPT_SCHEMA: &str = "molten.artifacts.receipt.v1";
pub const ARTIFACT_CLOSURE_SCHEMA: &str = "molten.artifacts.closure.v1";
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
    pub fn parse(value: &str) -> Result<Self> {
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

pub fn parse_canonical_bytes(bytes: &[u8]) -> Result<IoValue> {
    preserves::read_iovalue_packed(bytes, false).map_err(|error| MoltenError::Preserves(error.to_string()))
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

#[cfg(test)]
mod tests {

    #[test]
    fn preserves_text_roundtrip_keeps_hash() {
        let value = super::parse_text("<example \"a\" [1 2 3]>").expect("parse initial text");
        let hash = super::canonical_hash(&value).expect("hash initial value");
        let rendered = super::to_text(&value).expect("render preserves text");
        let reparsed = super::parse_text(&rendered).expect("parse rendered text");
        assert_eq!(hash, super::canonical_hash(&reparsed).expect("hash reparsed value"));
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
    fn canonical_content_ref_matches_canonical_hash() {
        let value = super::parse_text("<content-ref-fixture [#t 42]>").expect("parse fixture");
        let reference = super::canonical_content_ref(&value).expect("canonical content ref");
        assert_eq!(reference.as_str(), super::canonical_hash(&value).expect("canonical hash"));
    }
}
