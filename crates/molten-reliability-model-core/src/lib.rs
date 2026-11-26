#![cfg_attr(not(test), no_std)]

//! Pure live-reliability models for Aspen's bounded service-registry register.
//!
//! This crate owns register profile admission, the seeded SplitMix64 operation generator, the
//! public-client history projection and audit, the closed crash-fault matrix, and the bounded
//! serde vocabulary shared by mechanism profile DTOs. It has no filesystem, process, network,
//! clock, Tokio, Iroh, Redb, Wasmtime, `serde_json`, or Preserves dependency: canonical
//! identities and encoded sizes enter through explicit caller-supplied ports, and the `molten`
//! shell (`src/reliability/`) keeps JSON file I/O and Preserves hashing.
// r[impl molten.consensus.no_std_raft_and_reliability_kernels]
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

pub mod fault;
pub mod generator;
pub mod history;
pub mod mechanism;
pub mod profile;

#[cfg(test)]
mod tests;
