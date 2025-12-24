//! Fuzz target for Raft log entry deserialization.
//!
//! This target fuzzes the bincode deserialization of log entries stored in
//! redb. While this is typically local disk data, corruption can occur due to:
//! - Disk corruption / bit flips
//! - Incomplete writes (power loss)
//! - Storage bugs
//!
//! Attack vectors tested:
//! - Malformed bincode bytes
//! - Truncated entries
//! - Invalid log indices
//! - Corrupted entry payloads

// Import types used in log storage
use aspen::fuzz_helpers::AppRequest;
use aspen::fuzz_helpers::AppResponse;
use bolero::check;
use openraft::Entry;

// Type alias matching storage.rs
type LogEntry = Entry<aspen::fuzz_helpers::AppTypeConfig>;

#[test]
fn fuzz_log_entries() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // Fuzz log entry deserialization (storage.rs:493, 550, 581)
        let _ = bincode::deserialize::<LogEntry>(data);

        // Fuzz AppRequest deserialization (the payload inside entries)
        let _ = bincode::deserialize::<AppRequest>(data);

        // Fuzz AppResponse deserialization
        let _ = bincode::deserialize::<AppResponse>(data);

        // Also fuzz as raw openraft LogId for metadata corruption
        // LogId requires a full RaftTypeConfig, so we use AppTypeConfig
        let _ = bincode::deserialize::<openraft::LogId<aspen::fuzz_helpers::AppTypeConfig>>(data);
    });
}
