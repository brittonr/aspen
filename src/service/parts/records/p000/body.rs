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
