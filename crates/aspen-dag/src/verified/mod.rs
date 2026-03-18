//! Verified pure functions for DAG traversal.
//!
//! This module contains deterministic, side-effect-free functions used by
//! the traversal engine. All functions are:
//!
//! - **Deterministic**: No I/O, no system calls
//! - **Verified**: Formally proved correct using Verus (see `verus/` directory)
//! - **Production-ready**: Compiled normally by cargo with no ghost code overhead
//!
//! # Architecture
//!
//! Implements the "Functional Core, Imperative Shell" (FCIS) pattern:
//!
//! - **verified/** (this module): Production exec functions compiled by cargo
//! - **verus/**: Standalone Verus specs with ensures/requires clauses

pub mod traversal;
