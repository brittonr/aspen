//! Actor operations on the existing native supervisor and Host intent store.
//!
//! Dormancy is a completed checkpoint followed by drain, never terminal shutdown.
//! Rewake records actual process loss, then recovers the checkpoint with a fresh
//! incarnation. Every operation is bound to exact current Host authority; previews
//! only describe the request for an independent signer and grant no authority.

mod dormant;

use crate::node_service_hosting::AuthorityPort;
use crate::node_service_hosting::RuntimePort;

/// Exact actor operation projected for an independent current-operation signer.
#[derive(Clone, Copy)]
pub enum Input<'a> {
    Observe(&'a str),
    Checkpoint(&'a str),
    Sleep(&'a str),
    Drain(&'a str),
    Rewake(&'a str),
    State(&'a str),
    CheckpointValue(&'a str),
}

impl<'a> Input<'a> {
    fn parts(self) -> (&'static str, &'a str) {
        match self {
            Self::Observe(member) => ("actor-observe", member),
            Self::Checkpoint(member) => ("actor-checkpoint", member),
            Self::Sleep(member) => ("actor-sleep", member),
            Self::Drain(member) => ("actor-drain", member),
            Self::Rewake(member) => ("actor-rewake", member),
            Self::State(member) => ("actor-state", member),
            Self::CheckpointValue(member) => ("actor-checkpoint-value", member),
        }
    }
}

include!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/src/node/servicehosting/native/actor/parts/operations/body.rs"
));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/servicehosting/native/actor/parts/values/body.rs"));
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/node/servicehosting/native/actor/parts/delivery/body.rs"));
