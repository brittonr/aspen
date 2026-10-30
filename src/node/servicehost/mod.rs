//! Public, source-gated, verifier-authorized native node service-host admission.
//!
//! Bundle possession is not authority. The command boundary validates the actual
//! executable source gate, then a current exact bundle-load grant, then opens
//! durable native journals. Host owns operation admission before callbacks.

pub mod authority;
pub mod codec;
pub mod command;
pub mod effects;
pub mod installed;
pub mod operation;
mod runtime;

fn invalid(message: impl Into<String>) -> crate::error::Failure {
    crate::error::Failure::invalid_harness(message)
}
