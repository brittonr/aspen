#![cfg_attr(not(test), no_std)]

//! Pure Wasm component admission, runtime-profile validation, and optimization policy.
//!
//! The crate evaluates parsed component facts, typed import grants, and reviewed profile exports
//! into deterministic allow/deny decisions. It links no Wasmtime, `wasmparser`, filesystem,
//! `serde_json`, or Preserves code: the `molten` shell (`src/wasm/`) embeds the generated profile
//! exports from `config/`, decodes them, inspects component bytes, and supplies the declared
//! import-surface verdict through an explicit port.
// r[impl molten.wasm.no_std_wasm_policy_kernel]
// r[impl molten.project.tier1_no_std_kernel_crates]

extern crate alloc;

pub mod component;
pub mod performance;
pub mod refs;

#[cfg(test)]
mod tests;
