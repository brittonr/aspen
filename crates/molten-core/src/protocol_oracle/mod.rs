//! Protocol-aware simulation oracles.
//!
//! An extension emits bounded canonical projections of its admitted state at
//! each transition. This core admits them under a typed profile, assembles
//! them into a cohort by participant and logical position, and evaluates
//! safety and liveness independently of the transition under test: a reported
//! invariant name or pass list never establishes success. Results keep their
//! level and their earliest failure; missing or conflicting observations stay
//! incomplete instead of passing. Novelty identities and work counters are
//! canonical and deterministic, and evidence binds every identity with explicit
//! non-claims. The extension owns its projection meaning through
//! [`ProtocolSemantics`]; the core is workload-neutral and performs no I/O.

#![allow(
    tigerstyle::concrete_construction_in_core,
    reason = "the pure core constructs bounded projections, cohorts, and results, never infrastructure adapters"
)]

mod cohort;
mod counters;
mod evidence;
mod liveness;
mod novelty;
mod profile;
mod projection;
mod safety;
mod value;

pub use cohort::*;
pub use counters::*;
pub use evidence::*;
pub use liveness::*;
pub use novelty::*;
pub use profile::*;
pub use projection::*;
pub use safety::*;
pub use value::*;

#[cfg(test)]
mod tests;

/// The schema of every runtime projection.
pub const PROJECTION_SCHEMA: &str = "molten.protocol-projection.v1";
/// The schema of the typed oracle profile.
pub const ORACLE_PROFILE_SCHEMA: &str = "molten.protocol-oracle-profile.v1";
/// The BLAKE3 derive-key context of projection refs.
pub const PROJECTION_DOMAIN: &str = "molten.protocol-projection.v1";
/// The BLAKE3 derive-key context of projected state refs.
pub const STATE_DOMAIN: &str = "molten.protocol-state.v1";
/// The BLAKE3 derive-key context of novelty identities.
pub const NOVELTY_DOMAIN: &str = "molten.protocol-novelty.v1";
/// The BLAKE3 derive-key context of evidence refs.
pub const EVIDENCE_DOMAIN: &str = "molten.protocol-oracle-evidence.v1";
/// The record label of every projection.
pub const PROJECTION_LABEL: &str = "protocol-projection";

/// The non-claims every protocol oracle profile and evidence bundle carries.
pub const PROTOCOL_ORACLE_NON_CLAIMS: &[&str] = &[
    "simulation results do not prove KVM, live, production, or release behavior",
    "oracle results hold only for the recorded world, scheduler, workload, faults, and projections",
    "work counters are deterministic algorithmic observations, not hardware performance",
    "an independent oracle can share a defect with the implementation it checks",
];
