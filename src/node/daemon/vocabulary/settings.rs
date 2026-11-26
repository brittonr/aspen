//! State-root file names, control limits, and their compile-time bounds.

pub(super) const CONFIG_FILE: &str = "config.preserves";

pub(super) const PROFILE_RESOLUTION_FILE: &str = "profile-resolution.preserves";

pub(super) const STARTUP_FILE: &str = "startup-receipt.preserves";

pub(super) const HEALTH_FILE: &str = "health-receipt.preserves";

pub(super) const SHUTDOWN_FILE: &str = "shutdown-receipt.preserves";

pub(super) const CONTROL_STATUS_FILE: &str = "status-control-receipt.preserves";

pub(super) const CONTROL_STOP_FILE: &str = "stop-control-receipt.preserves";

pub(super) const CONTROL_INBOX_DIR: &str = "control/inbox";

pub(super) const CONTROL_OUTBOX_DIR: &str = "control/outbox";

pub(super) const CONTROL_INGRESS_DIR: &str = "control/iroh-ingress";

pub(super) const CONTROL_SERVICE_DIR: &str = "control/service";

pub const DEFAULT_CONTROL_INGRESS_TOPIC: &str = "node-control";

pub const LOCAL_CONTROL_INGRESS_TRANSPORT: &str = "iroh-local-gossip";

pub const LIVE_CONTROL_INGRESS_TRANSPORT: &str = "iroh-gossip";

pub(super) const CONTROL_LOCK_FILE: &str = "control/node.lock.preserves";

pub(super) const CONTROL_SERVICE_LOCK_FILE: &str = "control/service/service.lock.preserves";

pub(super) const IDENTITY_RECEIPT_FILE: &str = "identity-receipt.preserves";

pub(super) const IDENTITY_FILE: &str = "identity.preserves";

pub(super) const MAX_PENDING_CONTROL_REQUESTS: usize = 1024;

pub(super) const MAX_CONTROL_LOOP_REQUESTS: u64 = 1024;

pub const DEFAULT_CONTROL_LOOP_REQUESTS: u64 = 64;

pub(super) const MAX_CONTROL_SERVICE_TICKS: u64 = 4096;

pub(super) const MAX_CONTROL_LIVE_LISTENER_EVENTS: u64 = 4096;

pub(super) const MAX_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 5;

pub(super) const MAX_CONTROL_LIVE_SEND_TIMEOUT_MS: u64 = 60_000;

pub const DEFAULT_CONTROL_LIVE_SEND_ATTEMPTS: u64 = 1;

pub const DEFAULT_CONTROL_SERVICE_TICKS: u64 = 1;

pub const DEFAULT_CONTROL_LIVE_LISTENER_EVENTS: u64 = 1;

pub const DEFAULT_CONTROL_LIVE_LISTENER_TIMEOUT_MS: u64 = 250;

pub(super) const LIVE_WORKFLOW_PROTOCOL_ID: &str = "proto:molten.node-control.live-workflow-bundle.v1";

pub(super) const LIVE_WORKFLOW_PROTOCOL_SESSION_PREFIX: &str = "session:node-control-live-workflow:";

pub(super) const LIVE_WORKFLOW_LIFECYCLE_DIAGNOSTIC_CAPACITY: usize = 24;

pub(super) const LIVE_PROFILE_DIAGNOSTIC_CAPACITY: usize = 12;

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
