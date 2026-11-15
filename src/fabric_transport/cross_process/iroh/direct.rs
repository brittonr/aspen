//! Admitted direct endpoints over the owning cross-process Iroh shell.
use super::*;
use crate::fabric_transport::ProtectedSocketService;

#[path = "direct/evidence.rs"]
mod evidence;
#[path = "direct/pump.rs"]
mod pump;
use evidence::*;
use pump::*;

#[derive(Debug)]
pub struct Config {
    pub listener: ListenerInput,
    pub peer: DirectPeerBinding,
    pub bounds: DirectStreamBounds,
    pub receive_budget: ReceiveBudget,
    /// Imperative owner supplies the monotonic clock; admission never reads ambient time.
    pub clock: fn() -> std::time::Instant,
}

/// Created only after actual authenticated peer binding, typed receive gate,
/// exact operation preface and the consumer's current authority decision.
#[derive(Debug)]
pub struct AdmittedOperation {
    request: DirectStreamRequest,
    peer: DirectPeerBinding,
    authority_ref: String,
}
impl AdmittedOperation {
    pub fn request(&self) -> &DirectStreamRequest {
        &self.request
    }
    pub fn peer(&self) -> &DirectPeerBinding {
        &self.peer
    }
    pub fn authority_ref(&self) -> &str {
        &self.authority_ref
    }
}

pub struct Acceptor {
    inner: Listener,
    peer: DirectPeerBinding,
    bounds: DirectStreamBounds,
    gate: ReceiveGate,
    clock_origin: std::time::Instant,
    clock: fn() -> std::time::Instant,
    initial_tick: u64,
}

#[derive(Debug, Clone)]
pub struct Evidence {
    pub delivery: DeliveryOutcome,
    pub terminal: DirectStreamTerminal,
    pub sent_bytes: u64,
    pub received_bytes: u64,
    pub sent_frames: u64,
    pub received_frames: u64,
    pub local_consumed_bytes: u64,
    pub automatic_retry_count: u64,
    pub reset_observed: bool,
    pub cleanup_observed: bool,
    pub actual_path: DirectStreamPath,
    pub duration_ms: u64,
    pub evidence_ref: String,
    pub value: preserves::IOValue,
}

/// One explicitly provisioned outbound operation, without retry or ambient authority.
pub struct Outbound {
    pub input: ClientInput,
    pub peer: DirectPeerBinding,
    pub bounds: DirectStreamBounds,
    pub request: DirectStreamRequest,
    pub clock: fn() -> std::time::Instant,
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/fabric_transport/cross_process/iroh/direct/parts/listener/body.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/fabric_transport/cross_process/iroh/direct/parts/outbound/body.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/fabric_transport/cross_process/iroh/direct/parts/operation/body.rs"
));
include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/fabric_transport/cross_process/iroh/direct/parts/endpoint/body.rs"
));
