//! The replay observation boundary.
//!
//! Molten replays at its semantic host boundary: the Wasm hostcalls, typed
//! effect ports, scheduler decisions, asynchronous deliveries, clocks, entropy
//! streams, external reads, and runtime observations a transition consumes.
//! Native syscalls inside an admitted runtime are outside logical replay.
//!
//! Each replay profile carries a closed source inventory. Every source has one
//! handling class. A recorded-observation source supplies sealed recorded
//! results during replay, and an external mutation is never repeated.
//! Completeness, ordering, request binding, and profile, cohort, and protection
//! identity are decided here, purely, before any transition executes. The
//! shell supplies bytes and runtime observations and cannot waive a blocker.

mod claims;
mod completeness;
mod inventory;
mod members;
mod observation;
mod ordering;

pub use claims::*;
pub use completeness::*;
pub use inventory::*;
pub use members::*;
pub use observation::*;
pub use ordering::*;

#[cfg(test)]
mod tests;

pub const REPLAY_SOURCE_INVENTORY_SCHEMA: &str = "molten.world-replay.source-inventory.v1";
pub const REPLAY_OBSERVATION_LEDGER_SCHEMA: &str = "molten.world-replay.observation-ledger.v1";
pub const REPLAY_SOURCE_INVENTORY_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.source-inventory.v1";
pub const REPLAY_OBSERVATION_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.observation.v1";
pub const REPLAY_OBSERVATION_LEDGER_IDENTITY_DOMAIN: &str = "onixresearch.molten.world-replay.observation-ledger.v1";

/// The most sources one inventory declares.
pub const MAX_REPLAY_SOURCES: usize = 256;
/// The most observation slots, and recorded observations, one ledger carries.
pub const MAX_REPLAY_OBSERVATIONS: usize = 4_096;
/// The most opaque diagnostics one boundary attaches.
pub const MAX_REPLAY_OPAQUE_DIAGNOSTICS: usize = 8;
pub const MAX_NATIVE_PROCESS_RECORDS: u32 = 64;
pub const MAX_NATIVE_PROCESS_NON_CLAIMS: usize = 8;
pub const MAX_NATIVE_PROCESS_TEXT_BYTES: usize = 256;
/// The longest source identifier, operation identifier, or error class.
pub const MAX_REPLAY_TEXT_BYTES: usize = 128;
/// The largest recorded result one observation binds.
pub const MAX_REPLAY_OBSERVATION_BYTES: u64 = 16 * 1_048_576;
/// The most blockers one admission reports.
pub const MAX_REPLAY_BOUNDARY_BLOCKERS: usize = 64;
