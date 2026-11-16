#![no_std]
#![forbid(unsafe_code)]
#![allow(unknown_lints)]
#![allow(
    non_trait_imports,
    reason = "the no_std core composes one closed protocol vocabulary through explicit module imports"
)]
#![allow(
    path_segment_repetition,
    reason = "VM Cohort protocol names remain explicit and searchable at public boundaries"
)]
#![allow(
    module_file_count,
    reason = "checkpoint, planning, lifecycle, retry, receipt, identity, model, and tests are separate owned capabilities"
)]

//! Deterministic checkpoint admission, clone planning, lifecycle reduction,
//! retry policy, cleanup obligations, and bounded receipt projection.
//!
//! This crate performs no host effect. Linux and KVM types stay outside it.

extern crate alloc;

mod checkpoint;
mod cohort;
mod identity;
mod lifecycle;
mod model;
mod receipt;

pub use checkpoint::*;
pub use cohort::*;
pub use lifecycle::*;
pub use model::*;
pub use receipt::*;

#[cfg(test)]
mod tests;
