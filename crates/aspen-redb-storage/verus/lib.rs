//! Verus specifications for pure redb storage helper logic.
//!
//! Mirrors the deterministic helpers in `src/verified` without pulling in
//! redb, Raft, serialization, or Blake3 runtime dependencies.

use vstd::prelude::*;

mod kv_spec;
