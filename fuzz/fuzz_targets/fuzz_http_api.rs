//! Fuzz target for HTTP API request deserialization.
//!
//! This target fuzzes the JSON deserialization of HTTP API requests from
//! external clients. These are MEDIUM RISK since they handle untrusted
//! user input, but are mediated by axum's JSON extractor.
//!
//! Attack vectors tested:
//! - Malformed JSON
//! - Missing required fields
//! - Invalid enum variants
//! - Oversized keys/values (should be rejected by validation)
//! - Special characters in keys

use bolero::check;

// Import API request types
use aspen::fuzz_helpers::{
    AddLearnerRequest, ChangeMembershipRequest, DeleteRequest, InitRequest, ReadRequest,
    ScanRequest, WriteRequest,
};

#[test]
fn fuzz_http_api() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Fuzz cluster control endpoints
        let _ = serde_json::from_slice::<InitRequest>(data);
        let _ = serde_json::from_slice::<AddLearnerRequest>(data);
        let _ = serde_json::from_slice::<ChangeMembershipRequest>(data);

        // Fuzz key-value store endpoints
        let _ = serde_json::from_slice::<WriteRequest>(data);
        let _ = serde_json::from_slice::<ReadRequest>(data);
        let _ = serde_json::from_slice::<DeleteRequest>(data);
        let _ = serde_json::from_slice::<ScanRequest>(data);
    });
}
