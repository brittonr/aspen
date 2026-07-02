type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const DURABILITY_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_DURABILITY_SCHEMA;
const EVIDENCE_EXPORT_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_EVIDENCE_EXPORT_SCHEMA;
const FAULT_CASE_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_FAULT_CASE_SCHEMA;
const FAULT_MATRIX_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_FAULT_MATRIX_SCHEMA;
const RESOURCE_ENVELOPE_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_RESOURCE_ENVELOPE_SCHEMA;
const RUN_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_RUN_SCHEMA;

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

const MAX_REFS: usize = 512;
const MAX_TEXT_FIELDS: usize = 128;
const _: () = assert!(MAX_REFS <= 100_000);
const _: () = assert!(MAX_TEXT_FIELDS <= 100_000);

const REQUIRED_NETWORK_FAULTS: &[&str] = &[
    "delay",
    "drop",
    "partition",
    "rejoin",
    "stale-ticket",
    "wrong-authority",
    "duplicate-operation",
    "conflicting-operation-id",
    "corrupted-transport-receipt",
];

pub struct EvidenceExportInput<'a> {
    pub node: &'a str,
    pub node_evidence_ref: &'a str,
    pub artifact_refs: &'a [String],
    pub log_refs: &'a [String],
}

pub struct RunInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub topology_ref: &'a str,
    pub fault_profile: &'a str,
    pub node_evidence_refs: &'a [String],
    pub peer_ticket_refs: &'a [String],
    pub control_refs: &'a [String],
    pub remote_service_refs: &'a [String],
    pub job_refs: &'a [String],
    pub coordination_refs: &'a [String],
    pub evidence_export_refs: &'a [String],
    pub fault_refs: &'a [String],
    pub durability_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub log_refs: &'a [String],
    pub caveats: &'a [String],
}

pub struct DurabilityInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub queued_control_refs: &'a [String],
    pub recovery_refs: &'a [String],
    pub ledger_refs: &'a [String],
    pub chunk_refs: &'a [String],
    pub retention_refs: &'a [String],
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub struct FaultCaseInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub fault_kind: &'a str,
    pub injection: &'a str,
    pub expected_outcome: &'a str,
    pub evidence_refs: &'a [String],
    pub denial_refs: &'a [String],
    pub replay_status: &'a str,
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub struct ResourceEnvelopeInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub queue_depth: u64,
    pub max_queue_depth: u64,
    pub receipt_bytes: u64,
    pub max_receipt_bytes: u64,
    pub store_bytes: u64,
    pub max_store_bytes: u64,
    pub delivery_latency_ms: u64,
    pub max_delivery_latency_ms: u64,
    pub recovery_time_ms: u64,
    pub max_recovery_time_ms: u64,
    pub pressure_refs: &'a [String],
    pub denial_refs: &'a [String],
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub struct FaultMatrixInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub fault_case_refs: &'a [String],
    pub fault_kinds: &'a [String],
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub fn evidence_export_value(input: &EvidenceExportInput<'_>) -> Result<IoValue> {
    validate_text_field("node", input.node)?;
    validate_content_ref(input.node_evidence_ref)?;
    validate_ref_slice("evidence export artifact", input.artifact_refs)?;
    validate_ref_slice("evidence export log", input.log_refs)?;
    Ok(record("prod-soak-evidence-export-v1", vec![
        string(EVIDENCE_EXPORT_SCHEMA),
        record("node", vec![string(input.node)]),
        record("node-evidence", vec![string(input.node_evidence_ref)]),
        record("artifacts", vec![sequence(ref_values(input.artifact_refs)?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("checks", vec![sequence(vec![
            check_value("node-evidence-bound", "pass"),
            check_value("artifacts-exported", status(input.artifact_refs.is_empty())),
            check_value("logs-diagnostic-only", "pass"),
            check_value("export-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn run_value(input: &RunInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("fault profile", input.fault_profile)?;
    validate_ref_slice("node evidence", input.node_evidence_refs)?;
    validate_ref_slice("peer ticket", input.peer_ticket_refs)?;
    validate_ref_slice("node control", input.control_refs)?;
    validate_ref_slice("remote service", input.remote_service_refs)?;
    validate_ref_slice("job", input.job_refs)?;
    validate_ref_slice("coordination", input.coordination_refs)?;
    validate_ref_slice("evidence export", input.evidence_export_refs)?;
    validate_ref_slice("fault", input.fault_refs)?;
    validate_ref_slice("durability", input.durability_refs)?;
    validate_ref_slice("resource", input.resource_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_ref_slice("log", input.log_refs)?;
    validate_pass_category("node evidence", input.node_evidence_refs, input.decision)?;
    validate_pass_category("peer ticket", input.peer_ticket_refs, input.decision)?;
    validate_pass_category("node control", input.control_refs, input.decision)?;
    validate_pass_category("remote service", input.remote_service_refs, input.decision)?;
    validate_pass_category("job", input.job_refs, input.decision)?;
    validate_pass_category("coordination", input.coordination_refs, input.decision)?;
    validate_pass_category("evidence export", input.evidence_export_refs, input.decision)?;
    validate_fault_profile_refs(input.fault_profile, input.fault_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-run-v1", vec![
        string(RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("topology", vec![string(input.topology_ref)]),
        record("fault-profile", vec![string(input.fault_profile)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("peer-tickets", vec![sequence(ref_values(input.peer_ticket_refs)?)]),
        record("node-control-workflows", vec![sequence(ref_values(input.control_refs)?)]),
        record("remote-service", vec![sequence(ref_values(input.remote_service_refs)?)]),
        record("job-workers", vec![sequence(ref_values(input.job_refs)?)]),
        record("coordination", vec![sequence(ref_values(input.coordination_refs)?)]),
        record("evidence-exports", vec![sequence(ref_values(input.evidence_export_refs)?)]),
        record("faults", vec![sequence(ref_values(input.fault_refs)?)]),
        record("durability", vec![sequence(ref_values(input.durability_refs)?)]),
        record("resources", vec![sequence(ref_values(input.resource_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "diagnostic",
            input.diagnostics,
            MAX_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values("soak caveat", input.caveats, MAX_TEXT_FIELDS)?)]),
        record("checks", vec![sequence(vec![
            check_value("production-shaped-child-evidence", "pass"),
            check_value("replay-boundary-explicit", "pass"),
            check_value("live-caveats-explicit", status(input.caveats.is_empty())),
            check_value("soak-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn durability_value(input: &DurabilityInput<'_>) -> Result<IoValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_ref_slice("durability queued control", input.queued_control_refs)?;
    validate_ref_slice("durability recovery", input.recovery_refs)?;
    validate_ref_slice("durability ledger", input.ledger_refs)?;
    validate_ref_slice("durability chunk", input.chunk_refs)?;
    validate_ref_slice("durability retention", input.retention_refs)?;
    validate_pass_category("queued control", input.queued_control_refs, input.decision)?;
    validate_pass_category("recovery", input.recovery_refs, input.decision)?;
    validate_pass_category("ledger", input.ledger_refs, input.decision)?;
    validate_pass_category("chunk", input.chunk_refs, input.decision)?;
    validate_pass_category("retention", input.retention_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-durability-v1", vec![
        string(DURABILITY_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("queued-control", vec![sequence(ref_values(input.queued_control_refs)?)]),
        record("recovery", vec![sequence(ref_values(input.recovery_refs)?)]),
        record("ledger-readback", vec![sequence(ref_values(input.ledger_refs)?)]),
        record("chunk-artifacts", vec![sequence(ref_values(input.chunk_refs)?)]),
        record("retention-state", vec![sequence(ref_values(input.retention_refs)?)]),
        record("diagnostics", vec![sequence(string_values(
            "durability diagnostic",
            input.diagnostics,
            MAX_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "durability caveat",
            input.caveats,
            MAX_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("queued-control-bound", "pass"),
            check_value("recovery-receipts-bound", "pass"),
            check_value("ledger-readback-bound", "pass"),
            check_value("chunk-artifacts-bound", "pass"),
            check_value("retention-state-bound", "pass"),
            check_value("durability-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}
