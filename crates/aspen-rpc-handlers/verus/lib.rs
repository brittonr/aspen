//! Verus specifications and proofs for RPC handler pure verified helpers.
//!
//! Runtime handler code stays in `src/verified`; this crate mirrors the small
//! deterministic admission kernels used by those handlers.

use vstd::prelude::*;

verus! {

mod timeout_spec;
mod validation_spec;

} // verus!
