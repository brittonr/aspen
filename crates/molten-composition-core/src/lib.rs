#![cfg_attr(not(test), no_std)]

//! Pure service recovery composition planner vocabulary.
//!
//! The crate owns the scenario and variant vocabulary, the facts one scenario records, the
//! property evaluation that turns recorded facts into per-property outcomes, and the slice
//! contract of saturating work. Scenario drivers stay in the `molten` shell
//! (`src/harness/composition/`): they run the `molten-core` lifecycle, recovery-group, fabric
//! time, and fabric durability cores and hash checkpoints over canonical Preserves, none of
//! which this crate links.
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

pub mod judgment;
pub mod quantum;
pub mod vocabulary;

#[cfg(test)]
mod tests;
