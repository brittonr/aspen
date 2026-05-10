//! Verus specifications for pure raft-network helper logic.
//!
//! Mirrors deterministic framing/classifier kernels from `src/verified` without
//! pulling in Iroh, postcard, OpenRaft, or RPC channel types.

use vstd::prelude::*;

mod framing_spec;
mod heuristics_spec;
