#![no_main]
//! Fuzz target: PathInfo protobuf encoding/decoding.
//!
//! Feeds arbitrary bytes to protobuf PathInfo decode to find parsing
//! issues in the protobuf → PathInfo conversion pipeline.

use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    // Try to decode as protobuf PathInfo
    if let Ok(proto_pi) = snix_store::proto::PathInfo::decode(data) {
        // Try to convert to the validated PathInfo type
        let _ = snix_store::pathinfoservice::PathInfo::try_from(proto_pi);
    }
});
