type OrderedSet<T> = std::collections::BTreeSet<T>;
type IoValue = preserves::IOValue;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;
type Value<T> = preserves::Value<T>;

const MAX_SUPERVISION_ITEMS: usize = 4096;

const _: () = assert!(MAX_SUPERVISION_ITEMS <= 100_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionEvidenceInput {
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub retention_policy_refs: Vec<String>,
    pub prior_lifecycle_refs: Vec<String>,
    pub effect_log_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOwnedStateInput {
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub foreign_ref_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceOwnedState {
    pub state_ref: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub foreign_ref_claims: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionSuiteInput {
    pub manifest: IoValue,
    pub links: Vec<IoValue>,
    pub monitors: Vec<IoValue>,
    pub restart_policy: IoValue,
    pub owned_state: IoValue,
    pub restart_attempt: u64,
    pub logical_step: u64,
    pub evidence: ServiceSupervisionEvidenceInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionSuite {
    pub suite_ref: String,
    pub manifest: crate::service_records::ServiceManifest,
    pub links: Vec<crate::service_records::ServiceLink>,
    pub monitors: Vec<crate::service_records::ServiceMonitor>,
    pub restart_policy: crate::service_records::ServiceRestartPolicy,
    pub owned_state: ServiceOwnedState,
    pub restart_attempt: u64,
    pub logical_step: u64,
    pub evidence: ServiceSupervisionEvidenceInput,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionRun {
    pub suite_ref: String,
    pub suite_value: IoValue,
    pub report_ref: String,
    pub failure_markers: Vec<IoValue>,
    pub statuses: Vec<IoValue>,
    pub lifecycle_receipts: Vec<IoValue>,
    pub monitor_notifications: Vec<IoValue>,
    pub restart_decisions: Vec<IoValue>,
    pub scheduled_demands: Vec<IoValue>,
    pub cleanup_receipts: Vec<IoValue>,
    pub retractions: Vec<IoValue>,
    pub retention_inputs: Vec<IoValue>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionReplay {
    pub expected_report_ref: String,
    pub actual_report_ref: String,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionGate {
    pub receipt_ref: String,
    pub report_ref: String,
    pub suite_ref: String,
    pub decision: String,
    pub restart_decision: Option<String>,
    pub status_count: usize,
    pub monitor_count: usize,
    pub cleanup_count: usize,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisionGateReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub report_ref: String,
    pub suite_ref: String,
    pub restart_decision: Option<String>,
    pub status_count: u64,
    pub monitor_count: u64,
    pub cleanup_count: u64,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct GateReceiptValueInput<'a> {
    decision: &'a str,
    report_ref: &'a str,
    suite_ref: &'a str,
    restart_decision: Option<&'a str>,
    status_count: usize,
    monitor_count: usize,
    cleanup_count: usize,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RestartEvaluation {
    decision: String,
    attempt: u64,
    backoff_slot: u64,
    diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CleanupEvaluation {
    cleanup_receipt: Option<IoValue>,
    retractions: Vec<IoValue>,
    retention_input: Option<IoValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CleanupTarget {
    kind: String,
    target_ref: String,
}

pub fn service_owned_state_value(input: &ServiceOwnedStateInput) -> Result<IoValue> {
    validate_owned_state_input(input)?;
    Ok(crate::preserves_rail::record("service-owned-state-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_OWNED_STATE_SCHEMA),
        crate::preserves_rail::record("service-id", vec![crate::preserves_rail::string(&input.service_id)]),
        crate::preserves_rail::record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        crate::preserves_rail::record("owned-assertions", vec![refs_sequence(&input.owned_assertion_refs)]),
        crate::preserves_rail::record("observers", vec![refs_sequence(&input.observer_refs)]),
        crate::preserves_rail::record("live-refs", vec![refs_sequence(&input.live_ref_refs)]),
        crate::preserves_rail::record("exposed-refs", vec![refs_sequence(&input.exposed_ref_refs)]),
        crate::preserves_rail::record("pending-effects", vec![refs_sequence(&input.pending_effect_refs)]),
        crate::preserves_rail::record("foreign-claims", vec![refs_sequence(&input.foreign_ref_claims)]),
        checks_value(&["service-owned-state", "cleanup-index", "foreign-claims-explicit"]),
    ]))
}

pub fn parse_service_owned_state(value: &IoValue) -> Result<ServiceOwnedState> {
    let fields = value
        .collect_simple_record("service-owned-state-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-owned-state-v1 ...>"))?;
    require_schema(&fields[0], crate::preserves_rail::SERVICE_OWNED_STATE_SCHEMA, "service owned-state schema")?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "cleanup-index", "service owned state")?;
    let owned_state = ServiceOwnedState {
        state_ref: crate::preserves_rail::canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        manifest_ref: record_optional_ref(&fields[2], "manifest")?,
        owned_assertion_refs: parse_ref_sequence(&fields[3], "owned-assertions")?,
        observer_refs: parse_ref_sequence(&fields[4], "observers")?,
        live_ref_refs: parse_ref_sequence(&fields[5], "live-refs")?,
        exposed_ref_refs: parse_ref_sequence(&fields[6], "exposed-refs")?,
        pending_effect_refs: parse_ref_sequence(&fields[7], "pending-effects")?,
        foreign_ref_claims: parse_ref_sequence(&fields[8], "foreign-claims")?,
        value: value.clone(),
    };
    validate_owned_state_parsed(&owned_state)?;
    Ok(owned_state)
}

pub fn service_supervision_suite_value(input: &ServiceSupervisionSuiteInput) -> Result<IoValue> {
    validate_suite_input(input)?;
    Ok(crate::preserves_rail::record("service-supervision-suite-v1", vec![
        crate::preserves_rail::string(crate::preserves_rail::SERVICE_SUPERVISION_SUITE_SCHEMA),
        crate::preserves_rail::record("manifest", vec![input.manifest.clone()]),
        crate::preserves_rail::record("links", vec![crate::preserves_rail::sequence(input.links.clone())]),
        crate::preserves_rail::record("monitors", vec![crate::preserves_rail::sequence(input.monitors.clone())]),
        crate::preserves_rail::record("restart-policy", vec![input.restart_policy.clone()]),
        crate::preserves_rail::record("owned-state", vec![input.owned_state.clone()]),
        crate::preserves_rail::record("restart-attempt", vec![crate::preserves_rail::u64_value(input.restart_attempt)]),
        crate::preserves_rail::record("logical-step", vec![crate::preserves_rail::u64_value(input.logical_step)]),
        evidence_value(&input.evidence),
        checks_value(&[
            "canonical-service-supervision-suite",
            "logical-supervision-only",
            "bounded-restart-cleanup",
        ]),
    ]))
}

pub fn parse_service_supervision_suite(value: &IoValue) -> Result<ServiceSupervisionSuite> {
    let fields = value
        .collect_simple_record("service-supervision-suite-v1", Some(10))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervision-suite-v1 ...>"))?;
    require_schema(
        &fields[0],
        crate::preserves_rail::SERVICE_SUPERVISION_SUITE_SCHEMA,
        "service supervision suite schema",
    )?;
    let checks = parse_checks(&fields[9])?;
    require_check(&checks, "logical-supervision-only", "service supervision suite")?;
    let manifest_value = record_iovalue(&fields[1], "manifest")?;
    let restart_policy_value = record_iovalue(&fields[4], "restart-policy")?;
    let owned_state_value = record_iovalue(&fields[5], "owned-state")?;
    let suite = ServiceSupervisionSuite {
        suite_ref: crate::preserves_rail::canonical_hash(value)?,
        manifest: crate::service_records::parse_service_manifest(&manifest_value)?,
        links: parse_link_sequence(&fields[2])?,
        monitors: parse_monitor_sequence(&fields[3])?,
        restart_policy: crate::service_records::parse_service_restart_policy(&restart_policy_value)?,
        owned_state: parse_service_owned_state(&owned_state_value)?,
        restart_attempt: record_u64(&fields[6], "restart-attempt")?,
        logical_step: record_u64(&fields[7], "logical-step")?,
        evidence: parse_evidence(&fields[8])?,
        value: value.clone(),
    };
    validate_suite_parsed(&suite)?;
    Ok(suite)
}

pub fn run_service_supervision_suite_value(value: &IoValue) -> Result<ServiceSupervisionRun> {
    let suite = parse_service_supervision_suite(value)?;
    run_service_supervision_suite(&suite)
}
