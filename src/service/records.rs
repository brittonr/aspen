type IoValue = preserves::IOValue;

type Value<T> = preserves::Value<T>;
type MoltenError = crate::error::MoltenError;
type Result<T> = crate::error::Result<T>;

const SERVICE_CLEANUP_RECEIPT_SCHEMA: &str = crate::preserves_rail::SERVICE_CLEANUP_RECEIPT_SCHEMA;
const SERVICE_DEMAND_SCHEMA: &str = crate::preserves_rail::SERVICE_DEMAND_SCHEMA;
const SERVICE_LIFECYCLE_RECEIPT_SCHEMA: &str = crate::preserves_rail::SERVICE_LIFECYCLE_RECEIPT_SCHEMA;
const SERVICE_LINK_SCHEMA: &str = crate::preserves_rail::SERVICE_LINK_SCHEMA;
const SERVICE_MANIFEST_SCHEMA: &str = crate::preserves_rail::SERVICE_MANIFEST_SCHEMA;
const SERVICE_MONITOR_SCHEMA: &str = crate::preserves_rail::SERVICE_MONITOR_SCHEMA;
const SERVICE_RESTART_DECISION_SCHEMA: &str = crate::preserves_rail::SERVICE_RESTART_DECISION_SCHEMA;
const SERVICE_RESTART_POLICY_SCHEMA: &str = crate::preserves_rail::SERVICE_RESTART_POLICY_SCHEMA;
const SERVICE_STATUS_SCHEMA: &str = crate::preserves_rail::SERVICE_STATUS_SCHEMA;
const SERVICE_SUPERVISOR_SCHEMA: &str = crate::preserves_rail::SERVICE_SUPERVISOR_SCHEMA;

fn canonical_hash(value: &IoValue) -> Result<String> {
    crate::preserves_rail::canonical_hash(value)
}

fn record(label: &'static str, fields: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::record(label, fields)
}

fn sequence(values: Vec<IoValue>) -> IoValue {
    crate::preserves_rail::sequence(values)
}

fn string(value: &str) -> IoValue {
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

const MAX_SERVICE_IDS: usize = 512;
const MAX_SERVICE_REFS: usize = 4096;
const MAX_SERVICE_DIAGNOSTICS: usize = 256;
const MAX_SERVICE_CHECKS: usize = 256;

const _: () = assert!(MAX_SERVICE_IDS <= 10_000);
const _: () = assert!(MAX_SERVICE_REFS <= 100_000);
const _: () = assert!(MAX_SERVICE_DIAGNOSTICS <= 10_000);
const _: () = assert!(MAX_SERVICE_CHECKS <= 10_000);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManifestInput {
    pub service_id: String,
    pub owner_authority_ref: String,
    pub target_ref: String,
    pub dependencies: Vec<String>,
    pub provided_assertion_refs: Vec<String>,
    pub restart_policy_ref: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceManifest {
    pub manifest_ref: String,
    pub service_id: String,
    pub owner_authority_ref: String,
    pub target_ref: String,
    pub dependencies: Vec<String>,
    pub provided_assertion_refs: Vec<String>,
    pub restart_policy_ref: String,
    pub policy_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDemandInput {
    pub demand_id: String,
    pub service_id: String,
    pub requester_ref: String,
    pub manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceDemand {
    pub demand_ref: String,
    pub demand_id: String,
    pub service_id: String,
    pub requester_ref: String,
    pub manifest_ref: Option<String>,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatusInput {
    pub service_id: String,
    pub state: String,
    pub manifest_ref: Option<String>,
    pub demand_refs: Vec<String>,
    pub dependency_status_refs: Vec<String>,
    pub readiness_assertion_refs: Vec<String>,
    pub failure_refs: Vec<String>,
    pub restart_count: u64,
    pub monitor_refs: Vec<String>,
    pub replay_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceStatus {
    pub status_ref: String,
    pub service_id: String,
    pub state: String,
    pub manifest_ref: Option<String>,
    pub demand_refs: Vec<String>,
    pub dependency_status_refs: Vec<String>,
    pub readiness_assertion_refs: Vec<String>,
    pub failure_refs: Vec<String>,
    pub restart_count: u64,
    pub monitor_refs: Vec<String>,
    pub replay_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisorInput {
    pub supervisor_id: String,
    pub service_ids: Vec<String>,
    pub link_refs: Vec<String>,
    pub monitor_refs: Vec<String>,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceSupervisor {
    pub supervisor_ref: String,
    pub supervisor_id: String,
    pub service_ids: Vec<String>,
    pub link_refs: Vec<String>,
    pub monitor_refs: Vec<String>,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceLinkInput {
    pub supervisor_id: String,
    pub parent_service_id: String,
    pub child_service_id: String,
    pub propagation: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceLink {
    pub link_ref: String,
    pub supervisor_id: String,
    pub parent_service_id: String,
    pub child_service_id: String,
    pub propagation: String,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceMonitorInput {
    pub monitor_id: String,
    pub service_id: String,
    pub observer_ref: String,
    pub notification_policy: String,
    pub policy_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceMonitor {
    pub monitor_ref: String,
    pub monitor_id: String,
    pub service_id: String,
    pub observer_ref: String,
    pub notification_policy: String,
    pub policy_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRestartPolicyInput {
    pub policy_id: String,
    pub max_attempts: u64,
    pub window_steps: u64,
    pub backoff_steps: u64,
    pub resource_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRestartPolicy {
    pub policy_ref: String,
    pub policy_id: String,
    pub max_attempts: u64,
    pub window_steps: u64,
    pub backoff_steps: u64,
    pub resource_refs: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRestartDecisionInput {
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub policy_ref: String,
    pub attempt: u64,
    pub max_attempts: u64,
    pub window_step: u64,
    pub backoff_slot: u64,
    pub prior_lifecycle_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceRestartDecision {
    pub decision_ref: String,
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub policy_ref: String,
    pub attempt: u64,
    pub max_attempts: u64,
    pub window_step: u64,
    pub backoff_slot: u64,
    pub prior_lifecycle_refs: Vec<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceLifecycleReceiptInput {
    pub operation: String,
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub status_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub supervision_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceLifecycleReceipt {
    pub receipt_ref: String,
    pub operation: String,
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub status_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub resource_refs: Vec<String>,
    pub effect_profile_refs: Vec<String>,
    pub supervision_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCleanupReceiptInput {
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub retraction_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub retention_refs: Vec<String>,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCleanupReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub service_id: String,
    pub manifest_ref: Option<String>,
    pub authority_refs: Vec<String>,
    pub owned_assertion_refs: Vec<String>,
    pub observer_refs: Vec<String>,
    pub live_ref_refs: Vec<String>,
    pub exposed_ref_refs: Vec<String>,
    pub pending_effect_refs: Vec<String>,
    pub retraction_refs: Vec<String>,
    pub revocation_refs: Vec<String>,
    pub retention_refs: Vec<String>,
    pub diagnostics: Vec<String>,
    pub value: IoValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceRecord {
    Manifest(ServiceManifest),
    Demand(ServiceDemand),
    Status(ServiceStatus),
    Supervisor(ServiceSupervisor),
    Link(ServiceLink),
    Monitor(ServiceMonitor),
    RestartPolicy(ServiceRestartPolicy),
    RestartDecision(ServiceRestartDecision),
    LifecycleReceipt(ServiceLifecycleReceipt),
    CleanupReceipt(ServiceCleanupReceipt),
}

pub fn service_manifest_value(input: &ServiceManifestInput) -> Result<IoValue> {
    validate_manifest_input(input)?;
    Ok(record("service-manifest-v1", vec![
        string(SERVICE_MANIFEST_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("owner", vec![string(&input.owner_authority_ref)]),
        record("target", vec![string(&input.target_ref)]),
        record("requires", vec![service_id_sequence(&input.dependencies)]),
        record("provides", vec![refs_sequence(&input.provided_assertion_refs)]),
        record("restart-policy", vec![string(&input.restart_policy_ref)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&input.effect_profile_refs)]),
        checks_value(&[
            "schema-known",
            "explicit-authority",
            "target-ref-bound",
            "policy-resource-effect-declared",
            "canonical-service-record",
        ]),
    ]))
}

pub fn parse_service_manifest(value: &IoValue) -> Result<ServiceManifest> {
    let fields = value
        .collect_simple_record("service-manifest-v1", Some(11))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-manifest-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_MANIFEST_SCHEMA, "service manifest schema")?;
    let checks = parse_checks(&fields[10])?;
    require_check(&checks, "explicit-authority", "service manifest")?;
    require_check(&checks, "policy-resource-effect-declared", "service manifest")?;
    let manifest = ServiceManifest {
        manifest_ref: canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        owner_authority_ref: record_ref(&fields[2], "owner")?,
        target_ref: record_ref(&fields[3], "target")?,
        dependencies: parse_service_id_sequence(&fields[4], "requires")?,
        provided_assertion_refs: parse_ref_sequence(&fields[5], "provides")?,
        restart_policy_ref: record_ref(&fields[6], "restart-policy")?,
        policy_refs: parse_ref_sequence(&fields[7], "policy")?,
        resource_refs: parse_ref_sequence(&fields[8], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[9], "effect-profile")?,
        value: value.clone(),
    };
    validate_manifest_parsed(&manifest)?;
    Ok(manifest)
}

pub fn service_demand_value(input: &ServiceDemandInput) -> Result<IoValue> {
    validate_demand_input(input)?;
    Ok(record("service-demand-v1", vec![
        string(SERVICE_DEMAND_SCHEMA),
        record("demand-id", vec![string(&input.demand_id)]),
        record("service-id", vec![string(&input.service_id)]),
        record("requester", vec![string(&input.requester_ref)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["canonical-demand", "explicit-requester", "startup-admission-required"]),
    ]))
}

pub fn parse_service_demand(value: &IoValue) -> Result<ServiceDemand> {
    let fields = value
        .collect_simple_record("service-demand-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-demand-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_DEMAND_SCHEMA, "service demand schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "startup-admission-required", "service demand")?;
    let demand = ServiceDemand {
        demand_ref: canonical_hash(value)?,
        demand_id: record_string(&fields[1], "demand-id")?,
        service_id: record_string(&fields[2], "service-id")?,
        requester_ref: record_ref(&fields[3], "requester")?,
        manifest_ref: record_optional_ref(&fields[4], "manifest")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_service_id(&demand.service_id, "service demand service id")?;
    validate_non_empty(&demand.demand_id, "service demand id")?;
    Ok(demand)
}

pub fn service_status_value(input: &ServiceStatusInput) -> Result<IoValue> {
    validate_status_input(input)?;
    Ok(record("service-status-v1", vec![
        string(SERVICE_STATUS_SCHEMA),
        record("service-id", vec![string(&input.service_id)]),
        record("state", vec![string(&input.state)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("demands", vec![refs_sequence(&input.demand_refs)]),
        record("dependencies", vec![refs_sequence(&input.dependency_status_refs)]),
        record("readiness", vec![refs_sequence(&input.readiness_assertion_refs)]),
        record("failures", vec![refs_sequence(&input.failure_refs)]),
        record("restart-count", vec![u64_value(input.restart_count)]),
        record("monitors", vec![refs_sequence(&input.monitor_refs)]),
        record("replay", vec![refs_sequence(&input.replay_refs)]),
        checks_value(&["canonical-status", "owned-assertion-refs", "replay-identity-bound"]),
    ]))
}

pub fn parse_service_status(value: &IoValue) -> Result<ServiceStatus> {
    let fields = value
        .collect_simple_record("service-status-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-status-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_STATUS_SCHEMA, "service status schema")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "replay-identity-bound", "service status")?;
    let status = ServiceStatus {
        status_ref: canonical_hash(value)?,
        service_id: record_string(&fields[1], "service-id")?,
        state: record_string(&fields[2], "state")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        demand_refs: parse_ref_sequence(&fields[4], "demands")?,
        dependency_status_refs: parse_ref_sequence(&fields[5], "dependencies")?,
        readiness_assertion_refs: parse_ref_sequence(&fields[6], "readiness")?,
        failure_refs: parse_ref_sequence(&fields[7], "failures")?,
        restart_count: record_u64(&fields[8], "restart-count")?,
        monitor_refs: parse_ref_sequence(&fields[9], "monitors")?,
        replay_refs: parse_ref_sequence(&fields[10], "replay")?,
        value: value.clone(),
    };
    validate_service_id(&status.service_id, "service status service id")?;
    validate_state(&status.state)?;
    Ok(status)
}

pub fn service_supervisor_value(input: &ServiceSupervisorInput) -> Result<IoValue> {
    validate_supervisor_input(input)?;
    Ok(record("service-supervisor-v1", vec![
        string(SERVICE_SUPERVISOR_SCHEMA),
        record("supervisor-id", vec![string(&input.supervisor_id)]),
        record("services", vec![service_id_sequence(&input.service_ids)]),
        record("links", vec![refs_sequence(&input.link_refs)]),
        record("monitors", vec![refs_sequence(&input.monitor_refs)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-supervision", "no-os-parentage", "policy-declared"]),
    ]))
}

pub fn parse_service_supervisor(value: &IoValue) -> Result<ServiceSupervisor> {
    let fields = value
        .collect_simple_record("service-supervisor-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-supervisor-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_SUPERVISOR_SCHEMA, "service supervisor schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "logical-supervision", "service supervisor")?;
    let supervisor = ServiceSupervisor {
        supervisor_ref: canonical_hash(value)?,
        supervisor_id: record_string(&fields[1], "supervisor-id")?,
        service_ids: parse_service_id_sequence(&fields[2], "services")?,
        link_refs: parse_ref_sequence(&fields[3], "links")?,
        monitor_refs: parse_ref_sequence(&fields[4], "monitors")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_non_empty(&supervisor.supervisor_id, "service supervisor id")?;
    Ok(supervisor)
}

pub fn service_link_value(input: &ServiceLinkInput) -> Result<IoValue> {
    validate_link_input(input)?;
    Ok(record("service-link-v1", vec![
        string(SERVICE_LINK_SCHEMA),
        record("supervisor-id", vec![string(&input.supervisor_id)]),
        record("parent-service", vec![string(&input.parent_service_id)]),
        record("child-service", vec![string(&input.child_service_id)]),
        record("propagation", vec![string(&input.propagation)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-supervision", "no-os-parentage", "failure-propagation-declared"]),
    ]))
}

pub fn parse_service_link(value: &IoValue) -> Result<ServiceLink> {
    let fields = value
        .collect_simple_record("service-link-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-link-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_LINK_SCHEMA, "service link schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "no-os-parentage", "service link")?;
    let link = ServiceLink {
        link_ref: canonical_hash(value)?,
        supervisor_id: record_string(&fields[1], "supervisor-id")?,
        parent_service_id: record_string(&fields[2], "parent-service")?,
        child_service_id: record_string(&fields[3], "child-service")?,
        propagation: record_string(&fields[4], "propagation")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_link_parsed(&link)?;
    Ok(link)
}

pub fn service_monitor_value(input: &ServiceMonitorInput) -> Result<IoValue> {
    validate_monitor_input(input)?;
    Ok(record("service-monitor-v1", vec![
        string(SERVICE_MONITOR_SCHEMA),
        record("monitor-id", vec![string(&input.monitor_id)]),
        record("service-id", vec![string(&input.service_id)]),
        record("observer", vec![string(&input.observer_ref)]),
        record("notification-policy", vec![string(&input.notification_policy)]),
        record("policy", vec![refs_sequence(&input.policy_refs)]),
        checks_value(&["logical-monitor", "observer-ref-bound", "no-os-parentage"]),
    ]))
}

pub fn parse_service_monitor(value: &IoValue) -> Result<ServiceMonitor> {
    let fields = value
        .collect_simple_record("service-monitor-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-monitor-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_MONITOR_SCHEMA, "service monitor schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "observer-ref-bound", "service monitor")?;
    let monitor = ServiceMonitor {
        monitor_ref: canonical_hash(value)?,
        monitor_id: record_string(&fields[1], "monitor-id")?,
        service_id: record_string(&fields[2], "service-id")?,
        observer_ref: record_ref(&fields[3], "observer")?,
        notification_policy: record_string(&fields[4], "notification-policy")?,
        policy_refs: parse_ref_sequence(&fields[5], "policy")?,
        value: value.clone(),
    };
    validate_monitor_parsed(&monitor)?;
    Ok(monitor)
}

pub fn service_restart_policy_value(input: &ServiceRestartPolicyInput) -> Result<IoValue> {
    validate_restart_policy_input(input)?;
    Ok(record("service-restart-policy-v1", vec![
        string(SERVICE_RESTART_POLICY_SCHEMA),
        record("policy-id", vec![string(&input.policy_id)]),
        record("max-attempts", vec![u64_value(input.max_attempts)]),
        record("window-steps", vec![u64_value(input.window_steps)]),
        record("backoff-steps", vec![u64_value(input.backoff_steps)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        checks_value(&["bounded-restart", "logical-time", "resource-declared"]),
    ]))
}

pub fn parse_service_restart_policy(value: &IoValue) -> Result<ServiceRestartPolicy> {
    let fields = value
        .collect_simple_record("service-restart-policy-v1", Some(7))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-restart-policy-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RESTART_POLICY_SCHEMA, "service restart policy schema")?;
    let checks = parse_checks(&fields[6])?;
    require_check(&checks, "bounded-restart", "service restart policy")?;
    let policy = ServiceRestartPolicy {
        policy_ref: canonical_hash(value)?,
        policy_id: record_string(&fields[1], "policy-id")?,
        max_attempts: record_u64(&fields[2], "max-attempts")?,
        window_steps: record_u64(&fields[3], "window-steps")?,
        backoff_steps: record_u64(&fields[4], "backoff-steps")?,
        resource_refs: parse_ref_sequence(&fields[5], "resource")?,
        value: value.clone(),
    };
    validate_restart_policy_parsed(&policy)?;
    Ok(policy)
}

pub fn service_restart_decision_value(input: &ServiceRestartDecisionInput) -> Result<IoValue> {
    validate_restart_decision_input(input)?;
    Ok(record("service-restart-decision-v1", vec![
        string(SERVICE_RESTART_DECISION_SCHEMA),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("policy", vec![string(&input.policy_ref)]),
        record("attempt", vec![u64_value(input.attempt)]),
        record("max-attempts", vec![u64_value(input.max_attempts)]),
        record("window-step", vec![u64_value(input.window_step)]),
        record("backoff-slot", vec![u64_value(input.backoff_slot)]),
        record("prior-lifecycle", vec![refs_sequence(&input.prior_lifecycle_refs)]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["bounded-restart", "logical-window", "replay-identity-bound"]),
    ]))
}

pub fn parse_service_restart_decision(value: &IoValue) -> Result<ServiceRestartDecision> {
    let fields = value
        .collect_simple_record("service-restart-decision-v1", Some(14))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-restart-decision-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_RESTART_DECISION_SCHEMA, "service restart decision schema")?;
    let checks = parse_checks(&fields[13])?;
    require_check(&checks, "bounded-restart", "service restart decision")?;
    let decision = ServiceRestartDecision {
        decision_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        service_id: record_string(&fields[2], "service-id")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        policy_ref: record_ref(&fields[4], "policy")?,
        attempt: record_u64(&fields[5], "attempt")?,
        max_attempts: record_u64(&fields[6], "max-attempts")?,
        window_step: record_u64(&fields[7], "window-step")?,
        backoff_slot: record_u64(&fields[8], "backoff-slot")?,
        prior_lifecycle_refs: parse_ref_sequence(&fields[9], "prior-lifecycle")?,
        authority_refs: parse_ref_sequence(&fields[10], "authority")?,
        resource_refs: parse_ref_sequence(&fields[11], "resource")?,
        diagnostics: parse_string_sequence(&fields[12], "diagnostics")?,
        value: value.clone(),
    };
    validate_restart_decision_parsed(&decision)?;
    Ok(decision)
}

pub fn service_lifecycle_receipt_value(input: &ServiceLifecycleReceiptInput) -> Result<IoValue> {
    validate_lifecycle_input(input)?;
    Ok(record("service-lifecycle-receipt-v1", vec![
        string(SERVICE_LIFECYCLE_RECEIPT_SCHEMA),
        record("operation", vec![string(&input.operation)]),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("status", vec![optional_ref_value(input.status_ref.as_deref())]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("resource", vec![refs_sequence(&input.resource_refs)]),
        record("effect-profile", vec![refs_sequence(&input.effect_profile_refs)]),
        record("supervision", vec![refs_sequence(&input.supervision_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["canonical-receipt", "decision-before-side-effects", "text-not-evidence"]),
    ]))
}

pub fn parse_service_lifecycle_receipt(value: &IoValue) -> Result<ServiceLifecycleReceipt> {
    let fields = value
        .collect_simple_record("service-lifecycle-receipt-v1", Some(12))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-lifecycle-receipt-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_LIFECYCLE_RECEIPT_SCHEMA, "service lifecycle receipt schema")?;
    let checks = parse_checks(&fields[11])?;
    require_check(&checks, "canonical-receipt", "service lifecycle receipt")?;
    let receipt = ServiceLifecycleReceipt {
        receipt_ref: canonical_hash(value)?,
        operation: record_string(&fields[1], "operation")?,
        decision: record_string(&fields[2], "decision")?,
        service_id: record_string(&fields[3], "service-id")?,
        manifest_ref: record_optional_ref(&fields[4], "manifest")?,
        status_ref: record_optional_ref(&fields[5], "status")?,
        authority_refs: parse_ref_sequence(&fields[6], "authority")?,
        resource_refs: parse_ref_sequence(&fields[7], "resource")?,
        effect_profile_refs: parse_ref_sequence(&fields[8], "effect-profile")?,
        supervision_refs: parse_ref_sequence(&fields[9], "supervision")?,
        diagnostics: parse_string_sequence(&fields[10], "diagnostics")?,
        value: value.clone(),
    };
    validate_lifecycle_parsed(&receipt)?;
    Ok(receipt)
}

pub fn service_cleanup_receipt_value(input: &ServiceCleanupReceiptInput) -> Result<IoValue> {
    validate_cleanup_input(input)?;
    Ok(record("service-cleanup-receipt-v1", vec![
        string(SERVICE_CLEANUP_RECEIPT_SCHEMA),
        record("decision", vec![string(&input.decision)]),
        record("service-id", vec![string(&input.service_id)]),
        record("manifest", vec![optional_ref_value(input.manifest_ref.as_deref())]),
        record("authority", vec![refs_sequence(&input.authority_refs)]),
        record("owned-assertions", vec![refs_sequence(&input.owned_assertion_refs)]),
        record("observers", vec![refs_sequence(&input.observer_refs)]),
        record("live-refs", vec![refs_sequence(&input.live_ref_refs)]),
        record("exposed-refs", vec![refs_sequence(&input.exposed_ref_refs)]),
        record("pending-effects", vec![refs_sequence(&input.pending_effect_refs)]),
        record("retractions", vec![refs_sequence(&input.retraction_refs)]),
        record("revocations", vec![refs_sequence(&input.revocation_refs)]),
        record("retention", vec![refs_sequence(&input.retention_refs)]),
        record("diagnostics", vec![strings_sequence(&input.diagnostics)]),
        checks_value(&["canonical-cleanup", "owned-state-only", "retention-still-gates"]),
    ]))
}

pub fn parse_service_cleanup_receipt(value: &IoValue) -> Result<ServiceCleanupReceipt> {
    let fields = value
        .collect_simple_record("service-cleanup-receipt-v1", Some(15))
        .ok_or_else(|| MoltenError::invalid_harness("expected <service-cleanup-receipt-v1 ...>"))?;
    require_schema(&fields[0], SERVICE_CLEANUP_RECEIPT_SCHEMA, "service cleanup receipt schema")?;
    let checks = parse_checks(&fields[14])?;
    require_check(&checks, "owned-state-only", "service cleanup receipt")?;
    let receipt = ServiceCleanupReceipt {
        receipt_ref: canonical_hash(value)?,
        decision: record_string(&fields[1], "decision")?,
        service_id: record_string(&fields[2], "service-id")?,
        manifest_ref: record_optional_ref(&fields[3], "manifest")?,
        authority_refs: parse_ref_sequence(&fields[4], "authority")?,
        owned_assertion_refs: parse_ref_sequence(&fields[5], "owned-assertions")?,
        observer_refs: parse_ref_sequence(&fields[6], "observers")?,
        live_ref_refs: parse_ref_sequence(&fields[7], "live-refs")?,
        exposed_ref_refs: parse_ref_sequence(&fields[8], "exposed-refs")?,
        pending_effect_refs: parse_ref_sequence(&fields[9], "pending-effects")?,
        retraction_refs: parse_ref_sequence(&fields[10], "retractions")?,
        revocation_refs: parse_ref_sequence(&fields[11], "revocations")?,
        retention_refs: parse_ref_sequence(&fields[12], "retention")?,
        diagnostics: parse_string_sequence(&fields[13], "diagnostics")?,
        value: value.clone(),
    };
    validate_cleanup_parsed(&receipt)?;
    Ok(receipt)
}

pub fn parse_service_record(value: &IoValue) -> Result<ServiceRecord> {
    if value.collect_simple_record("service-manifest-v1", Some(11)).is_some() {
        return parse_service_manifest(value).map(ServiceRecord::Manifest);
    }
    if value.collect_simple_record("service-demand-v1", Some(7)).is_some() {
        return parse_service_demand(value).map(ServiceRecord::Demand);
    }
    if value.collect_simple_record("service-status-v1", Some(12)).is_some() {
        return parse_service_status(value).map(ServiceRecord::Status);
    }
    if value.collect_simple_record("service-supervisor-v1", Some(7)).is_some() {
        return parse_service_supervisor(value).map(ServiceRecord::Supervisor);
    }
    if value.collect_simple_record("service-link-v1", Some(7)).is_some() {
        return parse_service_link(value).map(ServiceRecord::Link);
    }
    if value.collect_simple_record("service-monitor-v1", Some(7)).is_some() {
        return parse_service_monitor(value).map(ServiceRecord::Monitor);
    }
    if value.collect_simple_record("service-restart-policy-v1", Some(7)).is_some() {
        return parse_service_restart_policy(value).map(ServiceRecord::RestartPolicy);
    }
    if value.collect_simple_record("service-restart-decision-v1", Some(14)).is_some() {
        return parse_service_restart_decision(value).map(ServiceRecord::RestartDecision);
    }
    if value.collect_simple_record("service-lifecycle-receipt-v1", Some(12)).is_some() {
        return parse_service_lifecycle_receipt(value).map(ServiceRecord::LifecycleReceipt);
    }
    if value.collect_simple_record("service-cleanup-receipt-v1", Some(15)).is_some() {
        return parse_service_cleanup_receipt(value).map(ServiceRecord::CleanupReceipt);
    }
    Err(MoltenError::invalid_harness("unknown service record schema"))
}

pub fn service_summary(value: &IoValue) -> Result<String> {
    let has_sensitive_marker = is_sensitive_marker_present(value)?;
    let redaction = if has_sensitive_marker { " redacted=true" } else { "" };
    Ok(summary_text(parse_service_record(value)?, redaction))
}

fn summary_text(record: ServiceRecord, redaction: &str) -> String {
    match record {
        ServiceRecord::Manifest(manifest) => manifest_text(&manifest, redaction),
        ServiceRecord::Demand(demand) => demand_text(&demand, redaction),
        ServiceRecord::Status(status) => status_text(&status, redaction),
        ServiceRecord::Supervisor(supervisor) => supervisor_text(&supervisor, redaction),
        ServiceRecord::Link(link) => link_text(&link, redaction),
        ServiceRecord::Monitor(monitor) => monitor_text(&monitor, redaction),
        ServiceRecord::RestartPolicy(policy) => restart_policy_text(&policy, redaction),
        ServiceRecord::RestartDecision(decision) => restart_decision_text(&decision, redaction),
        ServiceRecord::LifecycleReceipt(receipt) => lifecycle_text(&receipt, redaction),
        ServiceRecord::CleanupReceipt(receipt) => cleanup_text(&receipt, redaction),
    }
}

fn manifest_text(manifest: &ServiceManifest, redaction: &str) -> String {
    format!(
        "service manifest id={} target={} deps={} ref={}{}",
        manifest.service_id,
        manifest.target_ref,
        manifest.dependencies.len(),
        manifest.manifest_ref,
        redaction
    )
}

fn demand_text(demand: &ServiceDemand, redaction: &str) -> String {
    format!(
        "service demand id={} service={} requester={} ref={}{}",
        demand.demand_id, demand.service_id, demand.requester_ref, demand.demand_ref, redaction
    )
}

fn status_text(status: &ServiceStatus, redaction: &str) -> String {
    format!(
        "service status service={} state={} readiness={} ref={}{}",
        status.service_id,
        status.state,
        status.readiness_assertion_refs.len(),
        status.status_ref,
        redaction
    )
}

fn supervisor_text(supervisor: &ServiceSupervisor, redaction: &str) -> String {
    format!(
        "service supervisor id={} services={} ref={}{}",
        supervisor.supervisor_id,
        supervisor.service_ids.len(),
        supervisor.supervisor_ref,
        redaction
    )
}

fn link_text(link: &ServiceLink, redaction: &str) -> String {
    format!(
        "service link supervisor={} parent={} child={} propagation={} ref={}{}",
        link.supervisor_id, link.parent_service_id, link.child_service_id, link.propagation, link.link_ref, redaction
    )
}

fn monitor_text(monitor: &ServiceMonitor, redaction: &str) -> String {
    format!(
        "service monitor id={} service={} observer={} ref={}{}",
        monitor.monitor_id, monitor.service_id, monitor.observer_ref, monitor.monitor_ref, redaction
    )
}

fn restart_policy_text(policy: &ServiceRestartPolicy, redaction: &str) -> String {
    format!(
        "service restart-policy id={} max-attempts={} ref={}{}",
        policy.policy_id, policy.max_attempts, policy.policy_ref, redaction
    )
}

fn restart_decision_text(decision: &ServiceRestartDecision, redaction: &str) -> String {
    format!(
        "service restart decision={} service={} attempt={}/{} ref={}{}",
        decision.decision,
        decision.service_id,
        decision.attempt,
        decision.max_attempts,
        decision.decision_ref,
        redaction
    )
}

fn lifecycle_text(receipt: &ServiceLifecycleReceipt, redaction: &str) -> String {
    format!(
        "service lifecycle operation={} decision={} service={} ref={}{}",
        receipt.operation, receipt.decision, receipt.service_id, receipt.receipt_ref, redaction
    )
}

fn cleanup_text(receipt: &ServiceCleanupReceipt, redaction: &str) -> String {
    format!(
        "service cleanup decision={} service={} retractions={} ref={}{}",
        receipt.decision,
        receipt.service_id,
        receipt.retraction_refs.len(),
        receipt.receipt_ref,
        redaction
    )
}

fn validate_manifest_input(input: &ServiceManifestInput) -> Result<()> {
    validate_service_id(&input.service_id, "service manifest service id")?;
    require_ref(&input.owner_authority_ref, "service manifest owner authority ref")?;
    require_ref(&input.target_ref, "service manifest target ref")?;
    validate_service_ids(&input.dependencies, "service dependency")?;
    validate_refs(&input.provided_assertion_refs, "provided assertion ref")?;
    require_ref(&input.restart_policy_ref, "service restart policy ref")?;
    validate_refs(&input.policy_refs, "service policy ref")?;
    validate_refs(&input.resource_refs, "service resource ref")?;
    validate_refs(&input.effect_profile_refs, "service effect profile ref")?;
    require_non_empty_refs(&input.policy_refs, "service policy refs")?;
    require_non_empty_refs(&input.resource_refs, "service resource refs")?;
    require_non_empty_refs(&input.effect_profile_refs, "service effect profile refs")
}

fn validate_manifest_parsed(manifest: &ServiceManifest) -> Result<()> {
    validate_service_id(&manifest.service_id, "service manifest service id")?;
    require_ref(&manifest.owner_authority_ref, "service manifest owner authority ref")?;
    require_ref(&manifest.target_ref, "service manifest target ref")?;
    validate_service_ids(&manifest.dependencies, "service dependency")?;
    require_ref(&manifest.restart_policy_ref, "service restart policy ref")?;
    require_non_empty_refs(&manifest.policy_refs, "service policy refs")?;
    require_non_empty_refs(&manifest.resource_refs, "service resource refs")?;
    require_non_empty_refs(&manifest.effect_profile_refs, "service effect profile refs")
}

fn validate_demand_input(input: &ServiceDemandInput) -> Result<()> {
    validate_non_empty(&input.demand_id, "service demand id")?;
    validate_service_id(&input.service_id, "service demand service id")?;
    require_ref(&input.requester_ref, "service demand requester ref")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service demand manifest ref")?;
    validate_refs(&input.policy_refs, "service demand policy ref")
}

fn validate_status_input(input: &ServiceStatusInput) -> Result<()> {
    validate_service_id(&input.service_id, "service status service id")?;
    validate_state(&input.state)?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service status manifest ref")?;
    validate_refs(&input.demand_refs, "service status demand ref")?;
    validate_refs(&input.dependency_status_refs, "service dependency status ref")?;
    validate_refs(&input.readiness_assertion_refs, "service readiness assertion ref")?;
    validate_refs(&input.failure_refs, "service failure ref")?;
    validate_refs(&input.monitor_refs, "service monitor ref")?;
    validate_refs(&input.replay_refs, "service replay ref")
}

fn validate_supervisor_input(input: &ServiceSupervisorInput) -> Result<()> {
    validate_non_empty(&input.supervisor_id, "service supervisor id")?;
    validate_service_ids(&input.service_ids, "supervised service")?;
    validate_refs(&input.link_refs, "service link ref")?;
    validate_refs(&input.monitor_refs, "service monitor ref")?;
    validate_refs(&input.policy_refs, "service supervisor policy ref")
}

fn validate_link_input(input: &ServiceLinkInput) -> Result<()> {
    validate_non_empty(&input.supervisor_id, "service link supervisor id")?;
    validate_service_id(&input.parent_service_id, "service link parent service id")?;
    validate_service_id(&input.child_service_id, "service link child service id")?;
    validate_propagation(&input.propagation)?;
    validate_refs(&input.policy_refs, "service link policy ref")
}

fn validate_link_parsed(link: &ServiceLink) -> Result<()> {
    validate_non_empty(&link.supervisor_id, "service link supervisor id")?;
    validate_service_id(&link.parent_service_id, "service link parent service id")?;
    validate_service_id(&link.child_service_id, "service link child service id")?;
    validate_propagation(&link.propagation)?;
    validate_refs(&link.policy_refs, "service link policy ref")
}

fn validate_monitor_input(input: &ServiceMonitorInput) -> Result<()> {
    validate_non_empty(&input.monitor_id, "service monitor id")?;
    validate_service_id(&input.service_id, "service monitor service id")?;
    require_ref(&input.observer_ref, "service monitor observer ref")?;
    validate_notification_policy(&input.notification_policy)?;
    validate_refs(&input.policy_refs, "service monitor policy ref")
}

fn validate_monitor_parsed(monitor: &ServiceMonitor) -> Result<()> {
    validate_non_empty(&monitor.monitor_id, "service monitor id")?;
    validate_service_id(&monitor.service_id, "service monitor service id")?;
    require_ref(&monitor.observer_ref, "service monitor observer ref")?;
    validate_notification_policy(&monitor.notification_policy)?;
    validate_refs(&monitor.policy_refs, "service monitor policy ref")
}

fn validate_restart_policy_input(input: &ServiceRestartPolicyInput) -> Result<()> {
    validate_non_empty(&input.policy_id, "service restart policy id")?;
    if input.window_steps == 0 {
        return Err(MoltenError::invalid_harness("service restart policy window must be positive"));
    }
    validate_refs(&input.resource_refs, "service restart resource ref")?;
    require_non_empty_refs(&input.resource_refs, "service restart resource refs")
}

fn validate_restart_policy_parsed(policy: &ServiceRestartPolicy) -> Result<()> {
    validate_non_empty(&policy.policy_id, "service restart policy id")?;
    if policy.window_steps == 0 {
        return Err(MoltenError::invalid_harness("service restart policy window must be positive"));
    }
    require_non_empty_refs(&policy.resource_refs, "service restart resource refs")
}

fn validate_restart_decision_input(input: &ServiceRestartDecisionInput) -> Result<()> {
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service restart decision service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service restart decision manifest ref")?;
    require_ref(&input.policy_ref, "service restart decision policy ref")?;
    validate_refs(&input.prior_lifecycle_refs, "service restart decision lifecycle ref")?;
    validate_refs(&input.authority_refs, "service restart decision authority ref")?;
    validate_refs(&input.resource_refs, "service restart decision resource ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_restart_decision_parsed(decision: &ServiceRestartDecision) -> Result<()> {
    validate_decision(&decision.decision)?;
    validate_service_id(&decision.service_id, "service restart decision service id")?;
    require_ref(&decision.policy_ref, "service restart decision policy ref")?;
    validate_diagnostics(&decision.diagnostics)
}

fn validate_lifecycle_input(input: &ServiceLifecycleReceiptInput) -> Result<()> {
    validate_operation(&input.operation)?;
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service lifecycle service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service lifecycle manifest ref")?;
    validate_optional_ref(input.status_ref.as_deref(), "service lifecycle status ref")?;
    validate_refs(&input.authority_refs, "service lifecycle authority ref")?;
    validate_refs(&input.resource_refs, "service lifecycle resource ref")?;
    validate_refs(&input.effect_profile_refs, "service lifecycle effect profile ref")?;
    validate_refs(&input.supervision_refs, "service lifecycle supervision ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_lifecycle_parsed(receipt: &ServiceLifecycleReceipt) -> Result<()> {
    validate_operation(&receipt.operation)?;
    validate_decision(&receipt.decision)?;
    validate_service_id(&receipt.service_id, "service lifecycle service id")?;
    validate_refs(&receipt.supervision_refs, "service lifecycle supervision ref")?;
    validate_diagnostics(&receipt.diagnostics)
}

fn validate_cleanup_input(input: &ServiceCleanupReceiptInput) -> Result<()> {
    validate_decision(&input.decision)?;
    validate_service_id(&input.service_id, "service cleanup service id")?;
    validate_optional_ref(input.manifest_ref.as_deref(), "service cleanup manifest ref")?;
    validate_refs(&input.authority_refs, "service cleanup authority ref")?;
    validate_refs(&input.owned_assertion_refs, "service cleanup owned assertion ref")?;
    validate_refs(&input.observer_refs, "service cleanup observer ref")?;
    validate_refs(&input.live_ref_refs, "service cleanup live ref")?;
    validate_refs(&input.exposed_ref_refs, "service cleanup exposed ref")?;
    validate_refs(&input.pending_effect_refs, "service cleanup pending effect ref")?;
    validate_refs(&input.retraction_refs, "service cleanup retraction ref")?;
    validate_refs(&input.revocation_refs, "service cleanup revocation ref")?;
    validate_refs(&input.retention_refs, "service cleanup retention ref")?;
    validate_diagnostics(&input.diagnostics)
}

fn validate_cleanup_parsed(receipt: &ServiceCleanupReceipt) -> Result<()> {
    validate_decision(&receipt.decision)?;
    validate_service_id(&receipt.service_id, "service cleanup service id")?;
    validate_refs(&receipt.retraction_refs, "service cleanup retraction ref")?;
    validate_refs(&receipt.revocation_refs, "service cleanup revocation ref")?;
    validate_refs(&receipt.retention_refs, "service cleanup retention ref")?;
    validate_diagnostics(&receipt.diagnostics)
}

fn validate_service_ids(ids: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(ids.len(), MAX_SERVICE_IDS, field)?;
    for service_id in ids {
        validate_service_id(service_id, field)?;
    }
    Ok(())
}

fn validate_service_id(value: &str, field: &str) -> Result<()> {
    validate_non_empty(value, field)?;
    if value.starts_with("svc:") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected svc: service id for {field}, got {value}")))
    }
}

fn validate_state(state: &str) -> Result<()> {
    match state {
        "demanded" | "waiting" | "starting" | "ready" | "degraded" | "failed" | "stopped" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service state {state}"))),
    }
}

fn validate_operation(operation: &str) -> Result<()> {
    match operation {
        "declare" | "demand" | "status" | "start" | "ready" | "fail" | "restart" | "stop" | "cleanup"
        | "dependency-wait" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service lifecycle operation {operation}"))),
    }
}

fn validate_decision(decision: &str) -> Result<()> {
    match decision {
        "pass" | "deny" | "diagnostic" | "backoff" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service decision {decision}"))),
    }
}

fn validate_propagation(propagation: &str) -> Result<()> {
    match propagation {
        "restart" | "stop" | "notify" | "ignore" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service failure propagation {propagation}"))),
    }
}

fn validate_notification_policy(policy: &str) -> Result<()> {
    match policy {
        "failure" | "status" | "all" => Ok(()),
        _ => Err(MoltenError::invalid_harness(format!("unsupported service monitor notification policy {policy}"))),
    }
}

fn validate_diagnostics(diagnostics: &[String]) -> Result<()> {
    ensure_count_at_most(diagnostics.len(), MAX_SERVICE_DIAGNOSTICS, "service diagnostics")?;
    for diagnostic in diagnostics {
        validate_non_empty(diagnostic, "service diagnostic")?;
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn require_non_empty_refs(refs: &[String], field: &str) -> Result<()> {
    if refs.is_empty() {
        Err(MoltenError::invalid_harness(format!("{field} must not be empty")))
    } else {
        validate_refs(refs, field)
    }
}

fn validate_refs(refs: &[String], field: &str) -> Result<()> {
    ensure_count_at_most(refs.len(), MAX_SERVICE_REFS, field)?;
    for reference in refs {
        require_ref(reference, field)?;
    }
    Ok(())
}

fn validate_optional_ref(reference: Option<&str>, field: &str) -> Result<()> {
    if let Some(reference) = reference {
        require_ref(reference, field)
    } else {
        Ok(())
    }
}

fn require_ref(reference: &str, field: &str) -> Result<()> {
    validate_content_ref(reference).map_err(|error| {
        MoltenError::invalid_harness(format!("expected canonical content ref for {field}, got {reference}: {error}"))
    })
}

fn ensure_count_at_most(actual: usize, maximum: usize, label: &str) -> Result<()> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{label} count {actual} exceeds bound {maximum}")))
    }
}

fn service_id_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn refs_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn strings_sequence(values: &[String]) -> IoValue {
    sequence(values.iter().map(|value| string(value)).collect())
}

fn optional_ref_value(value: Option<&str>) -> IoValue {
    value.map_or_else(|| record("none", Vec::new()), |value| record("some", vec![string(value)]))
}

fn checks_value(names: &[&str]) -> IoValue {
    record("checks", vec![sequence(
        names.iter().map(|name| record("check", vec![string(name), string("pass")])).collect(),
    )])
}

fn parse_service_id_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_SERVICE_IDS, label)?;
    values
        .iter()
        .map(|value| {
            let service_id = required_string(value, label)?;
            validate_service_id(&service_id, label)?;
            Ok(service_id)
        })
        .collect()
}

fn parse_ref_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_SERVICE_REFS, label)?;
    values
        .iter()
        .map(|value| {
            let reference = required_string(value, label)?;
            require_ref(&reference, label)?;
            Ok(reference)
        })
        .collect()
}

fn parse_string_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<String>> {
    let values = field_sequence(value, label)?;
    ensure_count_at_most(values.len(), MAX_SERVICE_DIAGNOSTICS, label)?;
    values.iter().map(|value| required_string(value, label)).collect()
}

fn field_sequence(value: &Value<IoValue>, label: &str) -> Result<Vec<Value<IoValue>>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    let values = fields[0]
        .collect_sequence()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected sequence for {label}")))?;
    Ok(values.iter().cloned().collect())
}

fn parse_checks(value: &Value<IoValue>) -> Result<Vec<(String, String)>> {
    let values = field_sequence(value, "checks")?;
    ensure_count_at_most(values.len(), MAX_SERVICE_CHECKS, "service checks")?;
    values
        .iter()
        .map(|check| {
            let check = value_to_iovalue(check);
            let fields = check
                .collect_simple_record("check", Some(2))
                .ok_or_else(|| MoltenError::invalid_harness("expected service check"))?;
            Ok((required_string(&fields[0], "check name")?, required_string(&fields[1], "check status")?))
        })
        .collect()
}

fn require_check(checks: &[(String, String)], name: &str, context: &str) -> Result<()> {
    if checks.iter().any(|(check, status)| check == name && status == "pass") {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("{context} missing passing {name} check")))
    }
}

fn record_string(value: &Value<IoValue>, label: &str) -> Result<String> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_string(&fields[0], label)
}

fn record_ref(value: &Value<IoValue>, label: &str) -> Result<String> {
    let reference = record_string(value, label)?;
    require_ref(&reference, label)?;
    Ok(reference)
}

fn record_optional_ref(value: &Value<IoValue>, label: &str) -> Result<Option<String>> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    parse_optional_ref_value(&fields[0])
}

fn parse_optional_ref_value(value: &Value<IoValue>) -> Result<Option<String>> {
    if value.collect_simple_record("none", Some(0)).is_some() {
        return Ok(None);
    }
    if let Some(some) = value.collect_simple_record("some", Some(1)) {
        return required_ref(&some[0], "optional service ref").map(Some);
    }
    required_ref(value, "optional service ref").map(Some)
}

fn record_u64(value: &Value<IoValue>, label: &str) -> Result<u64> {
    let value = value_to_iovalue(value);
    let fields = value
        .collect_simple_record(label, Some(1))
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected <{label} ...>")))?;
    required_u64(&fields[0], label)
}

fn require_schema(value: &Value<IoValue>, expected: &str, field: &str) -> Result<()> {
    let actual = required_string(value, field)?;
    if actual == expected {
        Ok(())
    } else {
        Err(MoltenError::invalid_harness(format!("expected {field} {expected}, got {actual}")))
    }
}

fn required_ref(value: &Value<IoValue>, field: &str) -> Result<String> {
    let reference = required_string(value, field)?;
    require_ref(&reference, field)?;
    Ok(reference)
}

fn required_string(value: &Value<IoValue>, field: &str) -> Result<String> {
    value
        .as_string()
        .map(|value| value.into_owned())
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected string for {field}")))
}

fn required_u64(value: &Value<IoValue>, field: &str) -> Result<u64> {
    value
        .as_u64()
        .ok_or_else(|| MoltenError::invalid_harness(format!("expected u64 for {field}")))?
        .map_err(|error| MoltenError::invalid_harness(format!("u64 out of range for {field}: {error}")))
}

fn is_sensitive_marker_present(value: &IoValue) -> Result<bool> {
    let text = crate::preserves_rail::to_text(value)?;
    Ok(["<secret", "<confidential", "<credential", "<private", "<encrypted-ref"]
        .iter()
        .any(|marker| text.contains(marker)))
}

#[cfg(test)]
mod tests {
    use hegel::TestCase;
    use hegel::generators;

    use super::*;
    use crate::catalog;
    use crate::catalog::CatalogListInput;
    use crate::catalog::CatalogVisibilityInput;
    use crate::catalog_mcp;
    use crate::ledger;
    use crate::preserves_rail::content_ref_from_bytes;
    use crate::preserves_rail::parse_text;
    use crate::preserves_rail::to_text;

    fn test_ref(label: &str) -> String {
        content_ref_from_bytes(label.as_bytes())
    }

    fn manifest_input() -> ServiceManifestInput {
        ServiceManifestInput {
            service_id: "svc:web".to_string(),
            owner_authority_ref: test_ref("authority"),
            target_ref: test_ref("target"),
            dependencies: vec!["svc:db".to_string()],
            provided_assertion_refs: vec![test_ref("provided")],
            restart_policy_ref: test_ref("restart"),
            policy_refs: vec![test_ref("policy")],
            resource_refs: vec![test_ref("resource")],
            effect_profile_refs: vec![test_ref("effect")],
        }
    }

    #[test]
    fn service_manifest_roundtrips_with_stable_ref() {
        let value = service_manifest_value(&manifest_input()).expect("manifest value");
        let parsed = parse_service_manifest(&value).expect("parse manifest");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(parsed.service_id, "svc:web");
        assert_eq!(parsed.dependencies, vec!["svc:db".to_string()]);
        assert_eq!(parsed.manifest_ref, canonical_hash(&reparsed).expect("hash reparsed manifest"));
    }

    #[test]
    fn service_manifest_requires_explicit_boundaries() {
        let mut input = manifest_input();
        input.policy_refs.clear();
        let error = service_manifest_value(&input).expect_err("missing policy denied");
        assert!(error.to_string().contains("service policy refs"));

        let malformed = parse_text(
            "<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:web\"> \
             <owner \"not-a-ref\"> <target \"not-a-ref\"> <requires []> <provides []> \
             <restart-policy \"not-a-ref\"> <policy []> <resource []> <effect-profile []> \
             <checks [<check \"explicit-authority\" \"pass\"> <check \"policy-resource-effect-declared\" \"pass\">]>>",
        )
        .expect("parse malformed manifest");
        assert!(parse_service_manifest(&malformed).is_err());

        let short_ref = parse_text(
            "<service-manifest-v1 \"molten.service.manifest.v1\" <service-id \"svc:web\"> \
             <owner \"blake3:short\"> <target \"blake3:short\"> <requires []> <provides []> \
             <restart-policy \"blake3:short\"> <policy [\"blake3:short\"]> <resource [\"blake3:short\"]> \
             <effect-profile [\"blake3:short\"]> \
             <checks [<check \"explicit-authority\" \"pass\"> <check \"policy-resource-effect-declared\" \"pass\">]>>",
        )
        .expect("parse short-ref manifest");
        let error = parse_service_manifest(&short_ref).expect_err("short refs fail closed");
        assert!(error.to_string().contains("canonical content ref"));
    }

    struct Core {
        manifest: IoValue,
        manifest_ref: String,
        demand: IoValue,
        status: IoValue,
        status_ref: String,
    }

    struct Aux {
        supervisor: IoValue,
        link: IoValue,
        monitor: IoValue,
        monitor_ref: String,
        restart: IoValue,
        restart_ref: String,
    }

    struct Receipts {
        decision: IoValue,
        lifecycle: IoValue,
        cleanup: IoValue,
    }

    struct Case {
        manifest: IoValue,
        demand: IoValue,
        status: IoValue,
        supervisor: IoValue,
        link: IoValue,
        monitor: IoValue,
        restart: IoValue,
        decision: IoValue,
        lifecycle: IoValue,
        cleanup: IoValue,
    }

    fn base() -> Core {
        let manifest = service_manifest_value(&manifest_input()).expect("manifest");
        let manifest_ref = canonical_hash(&manifest).expect("manifest ref");
        let demand = service_demand_value(&ServiceDemandInput {
            demand_id: "demand:web".to_string(),
            service_id: "svc:web".to_string(),
            requester_ref: test_ref("requester"),
            manifest_ref: Some(manifest_ref.clone()),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("demand");
        let demand_ref = canonical_hash(&demand).expect("demand ref");
        let status = service_status_value(&ServiceStatusInput {
            service_id: "svc:web".to_string(),
            state: "ready".to_string(),
            manifest_ref: Some(manifest_ref.clone()),
            demand_refs: vec![demand_ref.clone()],
            dependency_status_refs: vec![test_ref("dep-status")],
            readiness_assertion_refs: vec![test_ref("ready")],
            failure_refs: Vec::new(),
            restart_count: 0,
            monitor_refs: Vec::new(),
            replay_refs: vec![test_ref("replay")],
        })
        .expect("status");
        let status_ref = canonical_hash(&status).expect("status ref");
        Core {
            manifest,
            manifest_ref,
            demand,
            status,
            status_ref,
        }
    }

    fn aux() -> Aux {
        let supervisor = service_supervisor_value(&ServiceSupervisorInput {
            supervisor_id: "supervisor:web".to_string(),
            service_ids: vec!["svc:web".to_string()],
            link_refs: vec![test_ref("link")],
            monitor_refs: vec![test_ref("monitor")],
            policy_refs: vec![test_ref("policy")],
        })
        .expect("supervisor");
        let link = service_link_value(&ServiceLinkInput {
            supervisor_id: "supervisor:web".to_string(),
            parent_service_id: "svc:web".to_string(),
            child_service_id: "svc:web".to_string(),
            propagation: "restart".to_string(),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("link");
        let monitor = service_monitor_value(&ServiceMonitorInput {
            monitor_id: "monitor:web".to_string(),
            service_id: "svc:web".to_string(),
            observer_ref: test_ref("observer"),
            notification_policy: "failure".to_string(),
            policy_refs: vec![test_ref("policy")],
        })
        .expect("monitor");
        let monitor_ref = canonical_hash(&monitor).expect("monitor ref");
        let restart = service_restart_policy_value(&ServiceRestartPolicyInput {
            policy_id: "restart:web".to_string(),
            max_attempts: 2,
            window_steps: 10,
            backoff_steps: 1,
            resource_refs: vec![test_ref("resource")],
        })
        .expect("restart policy");
        let restart_ref = canonical_hash(&restart).expect("restart policy ref");
        Aux {
            supervisor,
            link,
            monitor,
            monitor_ref,
            restart,
            restart_ref,
        }
    }

    fn decision(core: &Core, aux: &Aux) -> IoValue {
        service_restart_decision_value(&ServiceRestartDecisionInput {
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            policy_ref: aux.restart_ref.clone(),
            attempt: 1,
            max_attempts: 2,
            window_step: 0,
            backoff_slot: 0,
            prior_lifecycle_refs: vec![test_ref("prior")],
            authority_refs: vec![test_ref("authority")],
            resource_refs: vec![test_ref("resource")],
            diagnostics: Vec::new(),
        })
        .expect("restart decision")
    }

    fn lifecycle(core: &Core, aux: &Aux) -> IoValue {
        service_lifecycle_receipt_value(&ServiceLifecycleReceiptInput {
            operation: "ready".to_string(),
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            status_ref: Some(core.status_ref.clone()),
            authority_refs: vec![test_ref("authority-receipt")],
            resource_refs: vec![test_ref("resource-receipt")],
            effect_profile_refs: vec![test_ref("effect")],
            supervision_refs: vec![aux.monitor_ref.clone()],
            diagnostics: Vec::new(),
        })
        .expect("lifecycle")
    }

    fn cleanup(core: &Core) -> IoValue {
        service_cleanup_receipt_value(&ServiceCleanupReceiptInput {
            decision: "pass".to_string(),
            service_id: "svc:web".to_string(),
            manifest_ref: Some(core.manifest_ref.clone()),
            authority_refs: vec![test_ref("authority")],
            owned_assertion_refs: vec![test_ref("owned")],
            observer_refs: vec![test_ref("observer")],
            live_ref_refs: vec![test_ref("live")],
            exposed_ref_refs: vec![test_ref("exposed")],
            pending_effect_refs: vec![test_ref("effect")],
            retraction_refs: vec![test_ref("retraction")],
            revocation_refs: vec![test_ref("revocation")],
            retention_refs: vec![test_ref("retention")],
            diagnostics: Vec::new(),
        })
        .expect("cleanup")
    }

    fn receipts(core: &Core, aux: &Aux) -> Receipts {
        Receipts {
            decision: decision(core, aux),
            lifecycle: lifecycle(core, aux),
            cleanup: cleanup(core),
        }
    }

    fn case() -> Case {
        let core = base();
        let aux = aux();
        let receipts = receipts(&core, &aux);
        Case {
            manifest: core.manifest,
            demand: core.demand,
            status: core.status,
            supervisor: aux.supervisor,
            link: aux.link,
            monitor: aux.monitor,
            restart: aux.restart,
            decision: receipts.decision,
            lifecycle: receipts.lifecycle,
            cleanup: receipts.cleanup,
        }
    }

    fn assert_variants(case: &Case) {
        assert!(matches!(parse_service_record(&case.manifest).expect("manifest record"), ServiceRecord::Manifest(_)));
        assert!(matches!(parse_service_record(&case.demand).expect("demand record"), ServiceRecord::Demand(_)));
        assert!(matches!(parse_service_record(&case.status).expect("status record"), ServiceRecord::Status(_)));
        assert!(matches!(
            parse_service_record(&case.supervisor).expect("supervisor record"),
            ServiceRecord::Supervisor(_)
        ));
        assert!(matches!(parse_service_record(&case.link).expect("link record"), ServiceRecord::Link(_)));
        assert!(matches!(parse_service_record(&case.monitor).expect("monitor record"), ServiceRecord::Monitor(_)));
        assert!(matches!(
            parse_service_record(&case.restart).expect("restart record"),
            ServiceRecord::RestartPolicy(_)
        ));
        assert!(matches!(
            parse_service_record(&case.decision).expect("restart decision record"),
            ServiceRecord::RestartDecision(_)
        ));
        assert!(matches!(
            parse_service_record(&case.lifecycle).expect("lifecycle record"),
            ServiceRecord::LifecycleReceipt(_)
        ));
        assert!(matches!(
            parse_service_record(&case.cleanup).expect("cleanup record"),
            ServiceRecord::CleanupReceipt(_)
        ));
    }

    #[test]
    fn service_record_variants_roundtrip() {
        assert_variants(&case());
    }

    #[test]
    fn ledger_and_catalog_classify_service_records() {
        let dir = temp_dir("service-catalog");
        let registry = dir.join("registry");
        let ledger_root = dir.join("ledger");
        let manifest = service_manifest_value(&manifest_input()).expect("manifest");
        let imported = ledger::import_artifact(&ledger_root, &manifest).expect("ledger import");
        assert_eq!(imported.artifact_kind, "service-manifest");
        let listed = catalog::list(&registry, Some(&ledger_root), &CatalogListInput {
            kind: Some("service-manifest".to_string()),
            visibility: CatalogVisibilityInput::default(),
        })
        .expect("catalog list service manifest");
        assert_eq!(listed.items.len(), 1);
        let rendered = to_text(&listed.value).expect("render catalog result");
        assert!(rendered.contains("ledger-kind:service-manifest"));
        let request =
            catalog_mcp::mcp_request_value("catalog.list", vec![record("kind", vec![string("service-manifest")])])
                .expect("MCP request");
        let mcp = catalog_mcp::call(&registry, Some(&ledger_root), &request).expect("MCP list service manifest");
        assert_eq!(mcp.decision, "pass");
        assert!(to_text(&mcp.response_value).expect("render MCP response").contains("service-manifest"));
    }

    #[test]
    fn service_summary_redacts_secret_markers_and_is_not_parseable_evidence() {
        let lifecycle = parse_text(
            "<service-lifecycle-receipt-v1 \"molten.service.lifecycle-receipt.v1\" \
             <operation \"fail\"> <decision \"diagnostic\"> <service-id \"svc:web\"> \
             <manifest <none>> <status <none>> <authority []> <resource []> <effect-profile []> \
             <supervision []> <diagnostics [\"<secret do-not-render>\"]> \
             <checks [<check \"canonical-receipt\" \"pass\"> <check \"decision-before-side-effects\" \"pass\"> \
             <check \"text-not-evidence\" \"pass\">]>>",
        )
        .expect("parse secret lifecycle");
        let summary = service_summary(&lifecycle).expect("service summary");
        assert!(summary.contains("redacted=true"));
        assert!(!summary.contains("do-not-render"));
        let summary_value = parse_text(&format!("\"{summary}\"")).expect("parse summary string");
        assert!(parse_service_record(&summary_value).is_err());
    }

    #[hegel::test(test_cases = 16)]
    fn hegel_service_manifest_refs_are_stable_and_bounds_fail_closed(tc: TestCase) {
        let dependency_count = tc.draw(generators::integers::<u64>().min_value(0).max_value(4));
        let dependency_count_usize = usize::try_from(dependency_count).expect("bounded dependency count");
        let mut input = manifest_input();
        input.dependencies = (0..dependency_count_usize).map(|index| format!("svc:dep-{index}")).collect::<Vec<_>>();
        let value = service_manifest_value(&input).expect("manifest value");
        let first_ref = canonical_hash(&value).expect("first ref");
        let rendered = to_text(&value).expect("render manifest");
        let reparsed = parse_text(&rendered).expect("parse rendered manifest");
        assert_eq!(first_ref, canonical_hash(&reparsed).expect("second ref"));
        let mut too_many = input;
        too_many.dependencies = (0..=MAX_SERVICE_IDS).map(|index| format!("svc:overflow-{index}")).collect::<Vec<_>>();
        assert!(service_manifest_value(&too_many).is_err());
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        crate::test_support::cleanup_stale_molten_temp_dirs();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("molten-{label}-{}-{id}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).expect("remove stale temp dir");
        }
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
