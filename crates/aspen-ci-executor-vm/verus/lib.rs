//! Verus specifications and proofs for VM executor pure verified helpers.
//!
//! Runtime snapshot-admission logic stays in `src/verified/snapshot.rs`; this
//! crate mirrors its deterministic decision kernel and proves the small
//! threshold/pressure contracts.

use vstd::prelude::*;

verus! {

mod snapshot_spec;

} // verus!
