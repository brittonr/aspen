use preserves::IOValue;

type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const PROD_SOAK_DURABILITY_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_DURABILITY_SCHEMA;
const PROD_SOAK_EVIDENCE_EXPORT_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_EVIDENCE_EXPORT_SCHEMA;
const PROD_SOAK_FAULT_CASE_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_FAULT_CASE_SCHEMA;
const PROD_SOAK_FAULT_MATRIX_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_FAULT_MATRIX_SCHEMA;
const PROD_SOAK_RESOURCE_ENVELOPE_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_RESOURCE_ENVELOPE_SCHEMA;
const PROD_SOAK_RUN_SCHEMA: &str = crate::preserves_rail::PROD_SOAK_RUN_SCHEMA;

fn record(label: &'static str, fields: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IOValue>) -> IOValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: impl AsRef<str>) -> IOValue {
    crate::preserves_rail::string(value)
}

fn u64_value(value: u64) -> IOValue {
    crate::preserves_rail::u64_value(value)
}

fn validate_content_ref(value: &str) -> Result<()> {
    crate::preserves_rail::validate_content_ref(value)
}

const MAX_SOAK_REFS: usize = 512;
const MAX_SOAK_TEXT_FIELDS: usize = 128;
const _: () = assert!(MAX_SOAK_REFS <= 100_000);
const _: () = assert!(MAX_SOAK_TEXT_FIELDS <= 100_000);

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

pub struct ProdSoakEvidenceExportInput<'a> {
    pub node: &'a str,
    pub node_evidence_ref: &'a str,
    pub artifact_refs: &'a [String],
    pub log_refs: &'a [String],
}

pub struct ProdSoakRunInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub topology_ref: &'a str,
    pub fault_profile: &'a str,
    pub node_evidence_refs: &'a [String],
    pub peer_ticket_refs: &'a [String],
    pub node_control_refs: &'a [String],
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

pub struct ProdSoakDurabilityInput<'a> {
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

pub struct ProdSoakFaultCaseInput<'a> {
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

pub struct ProdSoakResourceEnvelopeInput<'a> {
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

pub struct ProdSoakFaultMatrixInput<'a> {
    pub decision: &'a str,
    pub scenario: &'a str,
    pub fault_case_refs: &'a [String],
    pub fault_kinds: &'a [String],
    pub diagnostics: &'a [String],
    pub caveats: &'a [String],
}

pub fn evidence_export_value(input: &ProdSoakEvidenceExportInput<'_>) -> Result<IOValue> {
    validate_text_field("node", input.node)?;
    validate_content_ref(input.node_evidence_ref)?;
    validate_ref_slice("evidence export artifact", input.artifact_refs)?;
    validate_ref_slice("evidence export log", input.log_refs)?;
    Ok(record("prod-soak-evidence-export-v1", vec![
        string(PROD_SOAK_EVIDENCE_EXPORT_SCHEMA),
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

pub fn run_value(input: &ProdSoakRunInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_content_ref(input.topology_ref)?;
    validate_text_field("fault profile", input.fault_profile)?;
    validate_ref_slice("node evidence", input.node_evidence_refs)?;
    validate_ref_slice("peer ticket", input.peer_ticket_refs)?;
    validate_ref_slice("node control", input.node_control_refs)?;
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
    validate_pass_category("node control", input.node_control_refs, input.decision)?;
    validate_pass_category("remote service", input.remote_service_refs, input.decision)?;
    validate_pass_category("job", input.job_refs, input.decision)?;
    validate_pass_category("coordination", input.coordination_refs, input.decision)?;
    validate_pass_category("evidence export", input.evidence_export_refs, input.decision)?;
    validate_fault_profile_refs(input.fault_profile, input.fault_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-run-v1", vec![
        string(PROD_SOAK_RUN_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("topology", vec![string(input.topology_ref)]),
        record("fault-profile", vec![string(input.fault_profile)]),
        record("node-evidence", vec![sequence(ref_values(input.node_evidence_refs)?)]),
        record("peer-tickets", vec![sequence(ref_values(input.peer_ticket_refs)?)]),
        record("node-control-workflows", vec![sequence(ref_values(input.node_control_refs)?)]),
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
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("logs", vec![sequence(ref_values(input.log_refs)?)]),
        record("caveats", vec![sequence(string_values(
            "soak caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("production-shaped-child-evidence", "pass"),
            check_value("replay-boundary-explicit", "pass"),
            check_value("live-caveats-explicit", status(input.caveats.is_empty())),
            check_value("soak-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn durability_value(input: &ProdSoakDurabilityInput<'_>) -> Result<IOValue> {
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
        string(PROD_SOAK_DURABILITY_SCHEMA),
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
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "durability caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
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

pub fn resource_envelope_value(input: &ProdSoakResourceEnvelopeInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_metric_bound("queue depth", input.queue_depth, input.max_queue_depth)?;
    validate_metric_bound("receipt bytes", input.receipt_bytes, input.max_receipt_bytes)?;
    validate_metric_bound("store bytes", input.store_bytes, input.max_store_bytes)?;
    validate_metric_bound("delivery latency ms", input.delivery_latency_ms, input.max_delivery_latency_ms)?;
    validate_metric_bound("recovery time ms", input.recovery_time_ms, input.max_recovery_time_ms)?;
    validate_ref_slice("resource pressure", input.pressure_refs)?;
    validate_ref_slice("resource denial", input.denial_refs)?;
    validate_pass_category("resource pressure", input.pressure_refs, input.decision)?;
    validate_pass_category("resource denial", input.denial_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-resource-envelope-v1", vec![
        string(PROD_SOAK_RESOURCE_ENVELOPE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("queue-depth", vec![u64_value(input.queue_depth)]),
        record("max-queue-depth", vec![u64_value(input.max_queue_depth)]),
        record("receipt-bytes", vec![u64_value(input.receipt_bytes)]),
        record("max-receipt-bytes", vec![u64_value(input.max_receipt_bytes)]),
        record("store-bytes", vec![u64_value(input.store_bytes)]),
        record("max-store-bytes", vec![u64_value(input.max_store_bytes)]),
        record("delivery-latency-ms", vec![u64_value(input.delivery_latency_ms)]),
        record("max-delivery-latency-ms", vec![u64_value(input.max_delivery_latency_ms)]),
        record("recovery-time-ms", vec![u64_value(input.recovery_time_ms)]),
        record("max-recovery-time-ms", vec![u64_value(input.max_recovery_time_ms)]),
        record("pressure", vec![sequence(ref_values(input.pressure_refs)?)]),
        record("denials", vec![sequence(ref_values(input.denial_refs)?)]),
        record("diagnostics", vec![sequence(string_values(
            "resource diagnostic",
            input.diagnostics,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "resource caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("queue-depth-bound", "pass"),
            check_value("receipt-growth-bound", "pass"),
            check_value("store-growth-bound", "pass"),
            check_value("delivery-latency-bound", "pass"),
            check_value("recovery-time-bound", "pass"),
            check_value("resource-pressure-denial-bound", "pass"),
        ])]),
    ]))
}

pub fn fault_case_value(input: &ProdSoakFaultCaseInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_fault_kind(input.fault_kind)?;
    validate_text_field("injection", input.injection)?;
    validate_text_field("expected outcome", input.expected_outcome)?;
    validate_ref_slice("fault evidence", input.evidence_refs)?;
    validate_ref_slice("fault denial", input.denial_refs)?;
    validate_text_field("replay status", input.replay_status)?;
    validate_pass_category("fault evidence", input.evidence_refs, input.decision)?;
    validate_pass_fault_denials(input.expected_outcome, input.denial_refs, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-fault-case-v1", vec![
        string(PROD_SOAK_FAULT_CASE_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-kind", vec![string(input.fault_kind)]),
        record("injection", vec![string(input.injection)]),
        record("expected-outcome", vec![string(input.expected_outcome)]),
        record("evidence", vec![sequence(ref_values(input.evidence_refs)?)]),
        record("denials", vec![sequence(ref_values(input.denial_refs)?)]),
        record("replay-status", vec![string(input.replay_status)]),
        record("diagnostics", vec![sequence(string_values(
            "fault diagnostic",
            input.diagnostics,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "fault caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("fault-kind-covered", "pass"),
            check_value("fault-evidence-bound", "pass"),
            check_value(
                "deny-before-side-effects-bound",
                status(denial_required(input.expected_outcome) && input.denial_refs.is_empty()),
            ),
            check_value("fault-evidence-does-not-grant-authority", "pass"),
        ])]),
    ]))
}

pub fn fault_matrix_value(input: &ProdSoakFaultMatrixInput<'_>) -> Result<IOValue> {
    validate_decision(input.decision)?;
    validate_text_field("scenario", input.scenario)?;
    validate_ref_slice("fault case", input.fault_case_refs)?;
    validate_fault_kinds(input.fault_kinds)?;
    validate_pass_category("fault case", input.fault_case_refs, input.decision)?;
    validate_fault_matrix_coverage(input.fault_kinds, input.decision)?;
    validate_pass_caveats(input.caveats, input.decision)?;
    Ok(record("prod-soak-fault-matrix-v1", vec![
        string(PROD_SOAK_FAULT_MATRIX_SCHEMA),
        record("decision", vec![string(input.decision)]),
        record("scenario", vec![string(input.scenario)]),
        record("fault-cases", vec![sequence(ref_values(input.fault_case_refs)?)]),
        record("fault-kinds", vec![sequence(input.fault_kinds.iter().map(string).collect())]),
        record("required-faults", vec![sequence(REQUIRED_NETWORK_FAULTS.iter().map(string).collect())]),
        record("diagnostics", vec![sequence(string_values(
            "fault matrix diagnostic",
            input.diagnostics,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("caveats", vec![sequence(string_values(
            "fault matrix caveat",
            input.caveats,
            MAX_SOAK_TEXT_FIELDS,
        )?)]),
        record("checks", vec![sequence(vec![
            check_value("network-transport-fault-matrix", "pass"),
            check_value("required-fault-kinds-covered", status(missing_required_faults(input.fault_kinds).is_some())),
            check_value("fault-cases-bound", status(input.fault_case_refs.is_empty())),
            check_value("simulated-faults-marked-diagnostic", "pass"),
        ])]),
    ]))
}

fn validate_metric_bound(label: &str, actual: u64, maximum: u64) -> Result<()> {
    if actual > maximum {
        Err(MoltenError::invalid_harness(format!("prod soak {label} {actual} exceeds bound {maximum}")))
    } else {
        Ok(())
    }
}

fn validate_pass_category(label: &str, refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && refs.is_empty() {
        Err(MoltenError::invalid_harness(format!("passing prod soak run requires at least one {label} ref")))
    } else {
        Ok(())
    }
}

fn validate_pass_caveats(caveats: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && caveats.is_empty() {
        Err(MoltenError::invalid_harness("passing prod soak run requires explicit evidence-only caveats"))
    } else {
        Ok(())
    }
}

fn validate_fault_profile_refs(fault_profile: &str, fault_refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && fault_profile != "none" && fault_refs.is_empty() {
        Err(MoltenError::invalid_harness(
            "passing prod soak run with non-none fault profile requires fault refs",
        ))
    } else {
        Ok(())
    }
}

fn validate_pass_fault_denials(expected_outcome: &str, denial_refs: &[String], decision: &str) -> Result<()> {
    if decision == "pass" && denial_required(expected_outcome) && denial_refs.is_empty() {
        Err(MoltenError::invalid_harness(
            "passing prod soak deny-before-side-effects fault requires denial refs",
        ))
    } else {
        Ok(())
    }
}

fn denial_required(expected_outcome: &str) -> bool {
    expected_outcome.contains("deny") || expected_outcome.contains("fail-closed")
}

fn validate_fault_kind(kind: &str) -> Result<()> {
    if REQUIRED_NETWORK_FAULTS.contains(&kind) {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!(
            "unsupported prod soak fault kind {kind}; expected one of {}",
            REQUIRED_NETWORK_FAULTS.join(", ")
        )))
    }
}

fn validate_fault_kinds(kinds: &[String]) -> Result<()> {
    if kinds.len() > MAX_SOAK_TEXT_FIELDS {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak fault kind count {} exceeds bound {MAX_SOAK_TEXT_FIELDS}",
            kinds.len()
        )));
    }
    for kind in kinds {
        validate_fault_kind(kind)?;
    }
    Ok(())
}

fn validate_fault_matrix_coverage(kinds: &[String], decision: &str) -> Result<()> {
    if decision == "pass"
        && let Some(missing) = missing_required_faults(kinds)
    {
        Err(MoltenError::invalid_harness(format!(
            "passing prod soak fault matrix missing fault kinds: {}",
            missing.join(", ")
        )))
    } else {
        Ok(())
    }
}

fn missing_required_faults(kinds: &[String]) -> Option<Vec<String>> {
    let present = kinds.iter().map(String::as_str).collect::<std::collections::BTreeSet<_>>();
    let missing = REQUIRED_NETWORK_FAULTS
        .iter()
        .filter(|kind| !present.contains(**kind))
        .map(|kind| (*kind).to_string())
        .collect::<Vec<_>>();
    if missing.is_empty() { None } else { Some(missing) }
}

fn validate_text_field(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("prod soak {label} must not be empty")))
    } else {
        Ok(())
    }
}

fn validate_ref_slice(label: &str, refs: &[String]) -> Result<()> {
    if refs.len() > MAX_SOAK_REFS {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} ref count {} exceeds bound {MAX_SOAK_REFS}",
            refs.len()
        )));
    }
    for reference in refs {
        validate_content_ref(reference).map_err(|error| {
            MoltenError::invalid_harness(format!("invalid prod soak {label} ref {reference}: {error}"))
        })?;
    }
    Ok(())
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "unavailable" | "skipped" => Ok(()),
        other => Err(MoltenError::invalid_harness(format!(
            "unsupported prod soak decision {other}; expected pass, deny, unavailable, or skipped"
        ))),
    }
}

fn ref_values(refs: &[String]) -> Result<Vec<IOValue>> {
    validate_ref_slice("artifact", refs)?;
    Ok(refs.iter().map(string).collect())
}

fn string_values(label: &str, values: &[String], maximum: usize) -> Result<Vec<IOValue>> {
    if values.len() > maximum {
        return Err(MoltenError::invalid_harness(format!(
            "prod soak {label} count {} exceeds bound {maximum}",
            values.len()
        )));
    }
    let mut output = Vec::with_capacity(values.len());
    for value in values {
        validate_text_field(label, value)?;
        output.push(string(value));
    }
    Ok(output)
}

fn check_value(name: &'static str, status: &'static str) -> IOValue {
    record("check", vec![string(name), string(status)])
}

fn status(is_problem: bool) -> &'static str {
    if is_problem { "deny" } else { "pass" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content_ref_from_bytes(bytes: &[u8]) -> String {
        crate::preserves_rail::content_ref_from_bytes(bytes)
    }

    fn to_text(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::to_text(value)
    }

    fn canonical_hash(value: &preserves::IOValue) -> crate::error::Result<String> {
        crate::preserves_rail::canonical_hash(value)
    }

    fn local_ref(name: &str) -> String {
        content_ref_from_bytes(name.as_bytes())
    }

    #[test]
    fn evidence_export_binds_node_and_artifacts() {
        let artifact_ref = local_ref("artifact");
        let value = evidence_export_value(&ProdSoakEvidenceExportInput {
            node: "node-a",
            node_evidence_ref: &local_ref("node-evidence"),
            artifact_refs: std::slice::from_ref(&artifact_ref),
            log_refs: &[local_ref("log")],
        })
        .expect("evidence export");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-evidence-export-v1"));
        assert!(text.contains(&artifact_ref));
    }

    #[test]
    fn run_receipt_binds_phase_one_child_categories() {
        let node_evidence = vec![local_ref("node-a"), local_ref("node-b")];
        let peer_ticket = vec![local_ref("ticket")];
        let node_control = vec![local_ref("protocol-gate")];
        let remote_service = vec![local_ref("remote-deliver")];
        let job = vec![local_ref("job-worker")];
        let coordination = vec![local_ref("coordination")];
        let evidence_export = vec![local_ref("export-a"), local_ref("export-b")];
        let value = run_value(&ProdSoakRunInput {
            decision: "pass",
            scenario: "phase1-soak",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &node_evidence,
            peer_ticket_refs: &peer_ticket,
            node_control_refs: &node_control,
            remote_service_refs: &remote_service,
            job_refs: &job,
            coordination_refs: &coordination,
            evidence_export_refs: &evidence_export,
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &[],
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[local_ref("log")],
            caveats: &["soak evidence is pilot-scoped".to_string()],
        })
        .expect("soak run");
        let reference = canonical_hash(&value).expect("ref");
        let text = to_text(&value).expect("text");
        assert!(reference.starts_with("blake3:"));
        assert!(text.contains("prod-soak-run-v1"));
        assert!(text.contains("phase1-soak"));
        assert!(text.contains(&remote_service[0]));
    }

    #[test]
    fn run_receipt_binds_network_diagnostics_and_metrics_refs() {
        let node_evidence = vec![local_ref("node-a"), local_ref("node-b")];
        let peer_ticket = vec![local_ref("ticket")];
        let node_control = vec![local_ref("framed-stream")];
        let remote_service = vec![local_ref("remote-service")];
        let job = vec![local_ref("job-worker")];
        let coordination = vec![local_ref("coordination")];
        let evidence_export = vec![local_ref("export-a")];
        let network_diagnostic = local_ref("network-diagnostics");
        let metrics_snapshot = local_ref("metrics-snapshot");
        let resource_refs = vec![network_diagnostic.clone(), metrics_snapshot.clone()];
        let value = run_value(&ProdSoakRunInput {
            decision: "pass",
            scenario: "phase1-network-diagnostics",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &node_evidence,
            peer_ticket_refs: &peer_ticket,
            node_control_refs: &node_control,
            remote_service_refs: &remote_service,
            job_refs: &job,
            coordination_refs: &coordination,
            evidence_export_refs: &evidence_export,
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &resource_refs,
            replay_status: "non-replayable-live-observations",
            diagnostics: &["network diagnostics are observability evidence only".to_string()],
            log_refs: &[local_ref("log")],
            caveats: &["network diagnostics do not grant side-effect authority".to_string()],
        })
        .expect("soak run");
        let text = to_text(&value).expect("text");
        assert!(text.contains(&network_diagnostic));
        assert!(text.contains(&metrics_snapshot));
        assert!(text.contains("soak-evidence-does-not-grant-authority"));
    }

    #[test]
    fn passing_run_requires_all_phase_one_categories() {
        let error = run_value(&ProdSoakRunInput {
            decision: "pass",
            scenario: "missing-remote",
            topology_ref: &local_ref("topology"),
            fault_profile: "none",
            node_evidence_refs: &[local_ref("node")],
            peer_ticket_refs: &[local_ref("ticket")],
            node_control_refs: &[local_ref("control")],
            remote_service_refs: &[],
            job_refs: &[local_ref("job")],
            coordination_refs: &[local_ref("coordination")],
            evidence_export_refs: &[local_ref("export")],
            fault_refs: &[],
            durability_refs: &[],
            resource_refs: &[],
            replay_status: "non-replayable-live-observations",
            diagnostics: &[],
            log_refs: &[],
            caveats: &["diagnostic only".to_string()],
        })
        .expect_err("missing remote should deny pass");
        assert!(error.to_string().contains("remote service"));
    }

    #[test]
    fn durability_receipt_requires_restart_and_state_refs() {
        let value = durability_value(&ProdSoakDurabilityInput {
            decision: "pass",
            scenario: "restart-durability",
            queued_control_refs: &[local_ref("restart-queue")],
            recovery_refs: &[local_ref("control-loop")],
            ledger_refs: &[local_ref("ledger-readback")],
            chunk_refs: &[local_ref("chunk-put")],
            retention_refs: &[local_ref("retention-pin")],
            diagnostics: &[],
            caveats: &["durability evidence is pilot scoped".to_string()],
        })
        .expect("durability");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-durability-v1"));
        assert!(text.contains("restart-durability"));
    }

    #[test]
    fn resource_envelope_receipt_binds_bounds_and_denials() {
        let value = resource_envelope_value(&ProdSoakResourceEnvelopeInput {
            decision: "pass",
            scenario: "pilot-resource-envelope",
            queue_depth: 1,
            max_queue_depth: 8,
            receipt_bytes: 4096,
            max_receipt_bytes: 1_000_000,
            store_bytes: 65_536,
            max_store_bytes: 10_000_000,
            delivery_latency_ms: 50,
            max_delivery_latency_ms: 5_000,
            recovery_time_ms: 100,
            max_recovery_time_ms: 10_000,
            pressure_refs: &[local_ref("pressure")],
            denial_refs: &[local_ref("denial")],
            diagnostics: &[],
            caveats: &["resource envelope evidence is pilot scoped".to_string()],
        })
        .expect("resource envelope");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-resource-envelope-v1"));
        assert!(text.contains("queue-depth-bound"));
    }

    #[test]
    fn fault_case_binds_denial_for_stale_ticket() {
        let denial = vec![local_ref("stale-ticket-denial")];
        let evidence = vec![local_ref("ticket")];
        let value = fault_case_value(&ProdSoakFaultCaseInput {
            decision: "pass",
            scenario: "network-faults",
            fault_kind: "stale-ticket",
            injection: "simulated-live-gate",
            expected_outcome: "deny-before-side-effects",
            evidence_refs: &evidence,
            denial_refs: &denial,
            replay_status: "simulated-fault",
            diagnostics: &["stale ticket denied before control side effects".to_string()],
            caveats: &["simulated fault evidence is diagnostic".to_string()],
        })
        .expect("fault case");
        let text = to_text(&value).expect("text");
        assert!(text.contains("prod-soak-fault-case-v1"));
        assert!(text.contains(&denial[0]));
    }

    #[test]
    fn fault_matrix_requires_all_network_faults_for_pass() {
        let fault_cases = vec![local_ref("case")];
        let incomplete = vec!["delay".to_string()];
        let error = fault_matrix_value(&ProdSoakFaultMatrixInput {
            decision: "pass",
            scenario: "network-faults",
            fault_case_refs: &fault_cases,
            fault_kinds: &incomplete,
            diagnostics: &[],
            caveats: &["simulated faults are diagnostic".to_string()],
        })
        .expect_err("missing faults deny pass");
        assert!(error.to_string().contains("drop"));

        let complete = REQUIRED_NETWORK_FAULTS.iter().map(|kind| (*kind).to_string()).collect::<Vec<_>>();
        let value = fault_matrix_value(&ProdSoakFaultMatrixInput {
            decision: "pass",
            scenario: "network-faults",
            fault_case_refs: &fault_cases,
            fault_kinds: &complete,
            diagnostics: &[],
            caveats: &["simulated faults are diagnostic".to_string()],
        })
        .expect("complete matrix");
        assert!(to_text(&value).expect("text").contains("prod-soak-fault-matrix-v1"));
    }
}
