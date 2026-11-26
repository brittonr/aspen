//! Public outcomes of local lifecycle, control dispatch, service, and supervisor requests.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Init {
    pub config_ref: String,
    pub identity_ref: String,
    pub identity_receipt_ref: String,
    pub profile_resolution_ref: String,
    pub config_value: preserves::IOValue,
    pub identity_receipt_value: preserves::IOValue,
    pub profile_resolution_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub startup_ref: String,
    pub startup_value: preserves::IOValue,
    pub adapter_receipt_refs: Vec<crate::node_runtime::NodeAdapterReceiptRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub health_ref: String,
    pub control_receipt_ref: String,
    pub health_value: preserves::IOValue,
    pub control_receipt_value: preserves::IOValue,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stop {
    pub shutdown_ref: String,
    pub control_receipt_ref: String,
    pub shutdown_value: preserves::IOValue,
    pub control_receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSubmit {
    pub request_ref: String,
    pub inbox_entry: String,
    pub queue_receipt_ref: String,
    pub queue_receipt_value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlDispatch {
    pub operation: String,
    pub request_ref: String,
    pub control_receipt_ref: String,
    pub control_receipt_value: preserves::IOValue,
    pub subreceipt_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlLoop {
    pub loop_receipt_ref: String,
    pub loop_receipt_value: preserves::IOValue,
    pub heartbeat_receipt_ref: String,
    pub heartbeat_receipt_value: preserves::IOValue,
    pub processed_request_refs: Vec<String>,
    pub dispatch_receipt_refs: Vec<String>,
    pub has_stopped: bool,
    pub is_denied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlServe {
    pub service_receipt_ref: String,
    pub service_receipt_value: preserves::IOValue,
    pub service_lock_ref: Option<String>,
    pub heartbeat_receipt_refs: Vec<String>,
    pub ingress_receipt_refs: Vec<String>,
    pub loop_receipt_refs: Vec<String>,
    pub processed_request_refs: Vec<String>,
    pub supervisor_policy_ref: Option<String>,
    pub supervisor_receipt_refs: Vec<String>,
    pub ticks: u64,
    pub has_stopped: bool,
    pub decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSupervisorPolicy {
    pub policy_ref: String,
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub value: preserves::IOValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlSupervisorReceipt {
    pub receipt_ref: String,
    pub decision: String,
    pub operation: String,
    pub supervisor_policy_ref: Option<String>,
    pub diagnostics: Vec<String>,
    pub value: preserves::IOValue,
}
