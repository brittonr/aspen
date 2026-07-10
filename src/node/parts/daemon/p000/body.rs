type IoValue = preserves::IOValue;
type Ipv4Addr = std::net::Ipv4Addr;
type MoltenError = crate::error::MoltenError;
type Path = std::path::Path;
type PathBuf = std::path::PathBuf;
type Result<T> = crate::error::Result<T>;
type SocketAddr = std::net::SocketAddr;

#[cfg(test)]
type Counter = std::sync::atomic::AtomicU64;

#[cfg(test)]
const RELAXED: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;

mod fs {
    pub(super) fn create_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub(super) fn read_dir(path: impl AsRef<std::path::Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }

    pub(super) fn read_to_string(path: impl AsRef<std::path::Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub(super) fn remove_file(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    pub(super) fn write(path: impl AsRef<std::path::Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }

    #[cfg(test)]
    pub(super) fn remove_dir_all(path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }
}

use n0_future::StreamExt;

use crate::bounded::VecSink;

const CONFIG_FILE: &str = "config.preserves";
const PROFILE_RESOLUTION_FILE: &str = "profile-resolution.preserves";
const STARTUP_FILE: &str = "startup-receipt.preserves";
const HEALTH_FILE: &str = "health-receipt.preserves";
const SHUTDOWN_FILE: &str = "shutdown-receipt.preserves";
const CONTROL_STATUS_FILE: &str = "status-control-receipt.preserves";
const CONTROL_STOP_FILE: &str = "stop-control-receipt.preserves";
const CONTROL_INBOX_DIR: &str = "control/inbox";
const CONTROL_OUTBOX_DIR: &str = "control/outbox";
const CONTROL_INGRESS_DIR: &str = "control/iroh-ingress";
const CONTROL_IDEMPOTENCY_DIR: &str = "control/idempotency";
const CONTROL_SERVICE_DIR: &str = "control/service";
pub const DEFAULT_CONTROL_INGRESS_TOPIC: &str = "node-control";
pub const LOCAL_CONTROL_INGRESS_TRANSPORT: &str = "iroh-local-gossip";
pub const LIVE_CONTROL_INGRESS_TRANSPORT: &str = "iroh-gossip";
const CONTROL_LOCK_FILE: &str = "control/node.lock.preserves";
const CONTROL_SERVICE_LOCK_FILE: &str = "control/service/service.lock.preserves";
const IDENTITY_RECEIPT_FILE: &str = "identity-receipt.preserves";
const IDENTITY_FILE: &str = "identity.preserves";
const MAX_PENDING_CONTROL_REQUESTS: usize = 1024;
const MAX_CONTROL_LOOP_REQUESTS: u64 = 1024;
pub const DEFAULT_CONTROL_LOOP_REQUESTS: u64 = 64;
const MAX_CONTROL_SERVICE_TICKS: u64 = 4096;
const MAX_CONTROL_LIVE_LISTENER_EVENTS: u64 = 4096;
const MAX_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 5;
const MAX_CONTROL_LIVE_SEND_TIMEOUT_MS: u64 = 60_000;
pub const DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 1;
pub const DEFAULT_CONTROL_SERVICE_TICKS: u64 = 1;
pub const DEFAULT_CONTROL_LIVE_LISTENER_EVENTS: u64 = 1;
pub const DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS: u64 = 250;
const LIVE_WORKFLOW_PROTOCOL_ID: &str = "proto:molten.node-control.live-workflow-bundle.v1";
const LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX: &str = "session:node-control-live-workflow:";
const LIVE_WORKFLOW_LIFECYCLE_DIAGNOSTIC_CAPACITY: usize = 24;
const LIVE_PROFILE_DIAGNOSTIC_CAPACITY: usize = 12;
const LIVE_PROFILE_RELAY_DIRECT: &str = "direct";
const LIVE_PROFILE_RELAY_RELAY: &str = "relay";
const LIVE_PROFILE_RELAY_AUTO: &str = "auto";

const _: () = assert!(MAX_PENDING_CONTROL_REQUESTS > 0);
const _: () = assert!(MAX_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LOOP_REQUESTS <= MAX_CONTROL_LOOP_REQUESTS);
const _: () = assert!(MAX_CONTROL_SERVICE_TICKS > 0);
const _: () = assert!(DEFAULT_CONTROL_SERVICE_TICKS > 0);
const _: () = assert!(DEFAULT_CONTROL_SERVICE_TICKS <= MAX_CONTROL_SERVICE_TICKS);
const _: () = assert!(MAX_CONTROL_LIVE_LISTENER_EVENTS > 0);
const _: () = assert!(MAX_CONTROL_LIVE_SEND_ATTEMPTS > 0);
const _: () = assert!(MAX_CONTROL_LIVE_SEND_TIMEOUT_MS > 0);
const _: () = assert!(DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS > 0);
const _: () = assert!(DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS <= MAX_CONTROL_LIVE_SEND_ATTEMPTS);
const _: () = assert!(DEFAULT_CONTROL_LIVE_LISTENER_EVENTS <= MAX_CONTROL_LIVE_LISTENER_EVENTS);
const _: () = assert!(DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS > 0);

#[derive(Debug, Clone, Copy)]
pub struct InitInput<'a> {
    pub state_root: &'a Path,
    pub node_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ProfileInitInput<'a> {
    pub state_root: &'a Path,
    pub node_id: &'a str,
    pub profile: &'a crate::node_profile_config::CheckedNodeProfile,
    pub overrides: &'a crate::node_profile_config::NodeProfileOverrides,
}

#[derive(Debug, Clone, Copy)]
pub struct RunInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StatusInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct StopInput<'a> {
    pub state_root: &'a Path,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSubmitInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlDispatchInput<'a> {
    pub state_root: &'a Path,
    pub request_path: Option<&'a Path>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLoopInput<'a> {
    pub state_root: &'a Path,
    pub max_requests: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_ticks: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IoValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeLifecycleFiles {
    pub has_config: bool,
    pub has_identity_receipt: bool,
    pub has_startup: bool,
    pub has_shutdown: bool,
    pub has_active_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeLifecycleState {
    Empty,
    Initialized,
    Running,
    Stopped,
    Inconsistent,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlSupervisorPolicyInput<'a> {
    pub max_restarts: u64,
    pub restart_window_ticks: u64,
    pub heartbeat_timeout_ticks: u64,
    pub shutdown_drain_ticks: u64,
    pub stale_lock_recovery: bool,
    pub policy_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct SupervisorReceiptValueInput<'a> {
    decision: &'a str,
    operation: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: Option<&'a str>,
    supervisor_policy_ref: Option<&'a str>,
    topic: &'a str,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ServiceLockValueInput<'a> {
    state_root: &'a Path,
    startup_receipt_ref: &'a str,
    node_id: &'a str,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    service_run_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct ServiceHeartbeatValueInput<'a> {
    startup_receipt_ref: &'a str,
    service_lock_ref: &'a str,
    tick: u64,
    delivered_count: u64,
    processed_count: u64,
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
struct ServiceRunReceiptValueInput<'a> {
    decision: &'a str,
    startup_receipt_ref: &'a str,
    service_lock_ref: Option<&'a str>,
    topic: &'a str,
    max_ticks: u64,
    max_requests_per_tick: u64,
    ticks: u64,
    heartbeat_receipt_refs: &'a [String],
    ingress_receipt_refs: &'a [String],
    loop_receipt_refs: &'a [String],
    processed_request_refs: &'a [String],
    has_stopped: bool,
    supervisor_policy_ref: Option<&'a str>,
    supervisor_receipt_refs: &'a [String],
    diagnostics: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressEnvelopeInput<'a> {
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressPublishInput<'a> {
    pub state_root: &'a Path,
    pub envelope_value: &'a IoValue,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlIngressDeliverInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub envelope_ref: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressPublishInput<'a> {
    pub sender: &'a iroh_gossip::api::GossipSender,
    pub envelope_value: &'a IoValue,
    pub node_id: &'a str,
    pub topology_profile_ref: Option<&'a str>,
    pub transport_profile_ref: Option<&'a str>,
    pub effective_max_attempts: Option<u64>,
    pub effective_join_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveIngressReceiveBytesInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub receiver_node: &'a str,
    pub delivered_from: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeInput<'a> {
    pub state_root: &'a Path,
    pub topic: &'a str,
    pub max_events: u64,
    pub event_timeout_ms: u64,
    pub max_requests_per_tick: u64,
    pub supervisor_policy_value: Option<&'a IoValue>,
}

#[derive(Debug, Clone, Copy)]
pub struct ControlLiveServeLoopbackInput<'a> {
    pub state_root: &'a Path,
    pub request_value: &'a IoValue,
    pub from_peer: &'a str,
    pub to_node: &'a str,
    pub topic: &'a str,
    pub sequence: u64,
    pub peer_bootstrap_refs: &'a [String],
    pub authority_refs: &'a [String],
    pub policy_refs: &'a [String],
    pub resource_refs: &'a [String],
    pub evidence_refs: &'a [String],
    pub max_requests_per_tick: u64,
}
