// Library builds are `#![no_std]` + `alloc`; the unit-test harness keeps the standard library.
#![cfg_attr(not(test), no_std)]
// The no_std build swaps the standard prelude for `core`'s plus the `alloc` items the standard
// prelude would otherwise provide, so every module keeps one source form under both profiles.
#![cfg_attr(not(test), feature(prelude_import))]
#![cfg_attr(not(test), allow(internal_features))]

//! Pure replicated-Raft kernel for Aspen's static three-voter consensus groups.
//!
//! The crate owns the replica state, the deterministic transition function (election, log
//! replication, leader read index, snapshots, recovery and rejoin), the typed effect plans a step
//! emits, start admission, peer-envelope shape validation, and the runtime port identity checks.
//! It links no Tokio, Iroh, Redb, Preserves, clock, entropy, filesystem or network code. Canonical
//! identities (election timer, snapshot, recovery and entry refs) enter through the
//! [`identity::ReplicaIdentity`] port, so the `molten` shell (`src/fabric_consistency/raft/`)
//! binds BLAKE3 over canonical packed Preserves while tests and models bind their own encoding.
//! Errors are the typed [`denial::ReplicaError`]; the shell maps each class to its `Failure`.
// r[impl molten.consensus.no_std_raft_and_reliability_kernels]
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

#[cfg(not(test))]
#[allow(
    unused_imports,
    reason = "rustc's unused-import analysis does not count type names resolved through the prelude"
)]
mod no_std_prelude {
    pub use alloc::borrow::ToOwned;
    pub use alloc::boxed::Box;
    pub use alloc::format;
    pub use alloc::string::String;
    pub use alloc::string::ToString;
    pub use alloc::vec;
    pub use alloc::vec::Vec;
    pub use core::prelude::rust_2024::*;
}

#[cfg(not(test))]
#[prelude_import]
#[allow(unused_imports)]
use crate::no_std_prelude::*;

pub mod admission;
pub mod denial;
pub mod identity;
pub mod model;
pub mod ports;
pub mod transition;

#[cfg(test)]
mod tests;
