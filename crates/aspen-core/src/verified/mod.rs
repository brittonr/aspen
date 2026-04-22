//! Verified pure functions for core distributed system operations.
//!
//! This module contains the production implementations of pure business logic
//! for Aspen's core operations. All functions are:
//!
//! - **Deterministic**: No I/O, no system calls, time passed as explicit parameter
//! - **Verified**: Formally proved correct using Verus (see `verus/` directory)
//! - **Production-ready**: Compiled normally by cargo with no ghost code overhead
//!
//! # Architecture
//!
//! This module implements the "Functional Core, Imperative Shell" (FCIS) pattern:
//!
//! - **verified/** (this module): Production exec functions compiled by cargo
//! - **verus/**: Standalone Verus specs with ensures/requires clauses verified by Verus
//!
//! # Modules
//!
//! - **scan**: Scan operations (pagination, filtering, continuation tokens)
//! - **validation**: Input validation for write commands and cluster operations
//! - **types**: Pure helper methods for NodeState, NodeId
//! - **hlc**: Timestamp conversions (HLC ↔ Unix milliseconds)
//!
//! # Public surface
//!
//! The documented `verified::*` surface for this change is limited to scan
//! helpers exposed at the module root and the `verified::scan::*` module path.

pub mod scan;

// Re-export scan functions at module level for convenience
pub use scan::build_scan_metadata;
pub use scan::decode_continuation_token;
pub use scan::encode_continuation_token;
pub use scan::execute_scan;
pub use scan::filter_scan_entries;
pub use scan::normalize_scan_limit;
pub use scan::paginate_entries;

