#![no_main]
//! Fuzz target: Directory protobuf deserialization.
//!
//! Feeds arbitrary bytes to protobuf Directory decode to find parsing
//! issues in the protobuf → Directory conversion pipeline.

use libfuzzer_sys::fuzz_target;
use prost::Message;

fuzz_target!(|data: &[u8]| {
    // Try to decode as protobuf Directory
    if let Ok(proto_dir) = snix_castore::proto::Directory::decode(data) {
        // Try to convert to the validated Directory type
        let _ = snix_castore::Directory::try_from(proto_dir);
    }
});
