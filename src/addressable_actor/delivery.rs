//! Actor ingress through the existing durable coordination-delivery owner.
//!
//! This adapter adds no mailbox, authority store or semantic journal. Operator admission
//! binds the exact preview before Local mutates; native callback state is the semantic commit.

mod native;
mod timers;

#[cfg(test)]
mod tests;

use molten_core::coordination_delivery::*;
pub use native::Commit;

use crate::coordination_delivery::DeliveryCommitPort;
use crate::fabric_time::TimerClockAdapter;
use crate::node_service_hosting::AuthorityPort;
use crate::system_extension::SystemExtensionExecutor;

/// Finite consumer and timer bound, not a promise of unbounded actor storage.
pub const MAX_REQUESTS: usize = 128;

pub struct Bridge<C: TimerClockAdapter> {
    local: crate::coordination_delivery::Local,
    manifest: DeliveryManifest,
    policy: DeliveryPolicy,
    time_profile: crate::fabric_time::AdmittedTimeProfile,
    time_admission_ref: String,
    timers: timers::Scheduler<C>,
    status: ObservedStatus,
}

#[derive(Debug)]
pub enum Acknowledgement {
    Committed {
        receipt_ref: String,
        status: crate::coordination_delivery::ServiceStatus,
    },
    /// Read from Local; the terminal service was not called a second time.
    AlreadyAcknowledged { state_ref: String, delivery_id: String },
}

include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/addressable_actor/delivery/parts/bridge/p000/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/addressable_actor/delivery/parts/bridge/p001/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/addressable_actor/delivery/parts/bridge/p002/body.rs"));
