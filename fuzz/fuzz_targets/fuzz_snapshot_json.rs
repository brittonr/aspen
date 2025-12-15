//! Fuzz target for snapshot JSON deserialization.
//!
//! This target fuzzes the JSON deserialization of snapshot data that arrives
//! during snapshot installation from the leader. This is HIGH RISK since
//! snapshot data comes from network peers.
//!
//! Attack vectors tested:
//! - Malformed JSON
//! - Invalid BTreeMap structure
//! - Oversized maps (DoS)
//! - Invalid key/value types
//! - Deeply nested structures

#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeMap;

fuzz_target!(|data: &[u8]| {
    // Fuzz JSON snapshot deserialization
    // This matches storage_sqlite.rs:1182 - install_snapshot()
    // The snapshot data is expected to be a BTreeMap<String, String>
    let _ = serde_json::from_slice::<BTreeMap<String, String>>(data);

    // Also fuzz as generic JSON Value to catch parser bugs
    let _ = serde_json::from_slice::<serde_json::Value>(data);
});
