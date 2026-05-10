//! Verus specifications and proofs for `aspen-ci-core` pure verified helpers.
//!
//! This proof crate mirrors small deterministic kernels from `src/verified/`
//! without depending on production Rust. It preserves Aspen's two-file
//! architecture: runtime helpers stay ordinary Rust, while formal contracts live
//! here.

use vstd::prelude::*;

verus! {

mod resource_spec;
mod timeout_spec;

} // verus!
