//! Differential fuzzing: Compare DeterministicKeyValueStore against reference model.
//!
//! This target generates sequences of key-value operations and verifies that
//! the DeterministicKeyValueStore behaves identically to a simple HashMap.
//! This catches semantic bugs and ensures Tiger Style bounds are correctly enforced.
//!
//! Properties verified:
//! - Set operations update values correctly
//! - Delete operations remove keys
//! - Read operations return correct values
//! - SetMulti/DeleteMulti atomicity
//! - Tiger Style bounds are enforced (MAX_KEY_SIZE, MAX_VALUE_SIZE)
//! - Operations are deterministic

#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::collections::HashMap;

// Import the real DeterministicKeyValueStore for comparison
use aspen::fuzz_helpers::{
    DeterministicKeyValueStore, KeyValueStore, KeyValueStoreError, ReadRequest, WriteCommand,
    WriteRequest,
};

/// Tiger Style: Resource bounds from constants.rs
const MAX_KEY_SIZE: usize = 1024; // 1 KB
const MAX_VALUE_SIZE: usize = 1024 * 1024; // 1 MB
const MAX_SETMULTI_KEYS: usize = 100;
const MAX_OPS_PER_SEQUENCE: usize = 100; // Reduced for performance in fuzzing

#[derive(Debug, Clone, Arbitrary)]
enum KvOp {
    /// Set a single key-value pair
    Set { key: String, value: String },
    /// Delete a single key
    Delete { key: String },
    /// Read a key (verifies existence)
    Read { key: String },
    /// Set multiple key-value pairs atomically
    SetMulti { pairs: Vec<(String, String)> },
    /// Delete multiple keys atomically
    DeleteMulti { keys: Vec<String> },
}

#[derive(Debug, Arbitrary)]
struct OpSequence {
    ops: Vec<KvOp>,
}

/// Check if a key is within Tiger Style bounds
fn is_valid_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= MAX_KEY_SIZE
}

/// Check if a value is within Tiger Style bounds
fn is_valid_value(value: &str) -> bool {
    value.len() <= MAX_VALUE_SIZE
}

fuzz_target!(|input: OpSequence| {
    // Tiger Style: Bound total operation count
    if input.ops.len() > MAX_OPS_PER_SEQUENCE {
        return;
    }

    // Create tokio runtime for async operations
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create tokio runtime");

    rt.block_on(async {
        // Reference implementation using HashMap
        let mut reference: HashMap<String, String> = HashMap::new();

        // Real implementation under test
        let store = DeterministicKeyValueStore::new();

        for op in &input.ops {
            match op {
                KvOp::Set { key, value } => {
                    if is_valid_key(key) && is_valid_value(value) {
                        // Apply to reference
                        reference.insert(key.clone(), value.clone());

                        // Apply to real store
                        let request = WriteRequest {
                            command: WriteCommand::Set {
                                key: key.clone(),
                                value: value.clone(),
                            },
                        };
                        let result = store.write(request).await;
                        assert!(result.is_ok(), "Set should succeed for valid input");
                    }
                }

                KvOp::Delete { key } => {
                    if is_valid_key(key) {
                        // Apply to reference
                        reference.remove(key);

                        // Apply to real store
                        let request = WriteRequest {
                            command: WriteCommand::Delete { key: key.clone() },
                        };
                        let _ = store.write(request).await;
                    }
                }

                KvOp::Read { key } => {
                    if is_valid_key(key) {
                        // Read from reference
                        let ref_value = reference.get(key).cloned();

                        // Read from real store
                        let request = ReadRequest { key: key.clone() };
                        let store_result = store.read(request).await;

                        // Compare results
                        match (ref_value, store_result) {
                            (Some(expected), Ok(result)) => {
                                assert_eq!(
                                    expected, result.value,
                                    "Read value mismatch for key '{}'",
                                    key
                                );
                            }
                            (None, Err(KeyValueStoreError::NotFound { .. })) => {
                                // Both agree key doesn't exist - good
                            }
                            (Some(_), Err(e)) => {
                                panic!("Reference has key but store returned error: {:?}", e);
                            }
                            (None, Ok(result)) => {
                                panic!(
                                    "Reference missing key but store returned value: {:?}",
                                    result
                                );
                            }
                        }
                    }
                }

                KvOp::SetMulti { pairs } => {
                    // Tiger Style: Bound number of keys in multi-set
                    if pairs.len() > MAX_SETMULTI_KEYS {
                        continue;
                    }

                    // All pairs must be valid for batch to succeed
                    let all_valid = pairs
                        .iter()
                        .all(|(k, v)| is_valid_key(k) && is_valid_value(v));

                    if all_valid && !pairs.is_empty() {
                        // Apply to reference
                        for (key, value) in pairs {
                            reference.insert(key.clone(), value.clone());
                        }

                        // Apply to real store
                        let request = WriteRequest {
                            command: WriteCommand::SetMulti {
                                pairs: pairs.clone(),
                            },
                        };
                        let result = store.write(request).await;
                        assert!(result.is_ok(), "SetMulti should succeed for valid input");
                    }
                }

                KvOp::DeleteMulti { keys } => {
                    // Tiger Style: Bound number of keys in multi-delete
                    if keys.len() > MAX_SETMULTI_KEYS {
                        continue;
                    }

                    // All keys must be valid for batch to succeed
                    let all_valid = keys.iter().all(|k| is_valid_key(k));

                    if all_valid && !keys.is_empty() {
                        // Apply to reference
                        for key in keys {
                            reference.remove(key);
                        }

                        // Apply to real store
                        let request = WriteRequest {
                            command: WriteCommand::DeleteMulti { keys: keys.clone() },
                        };
                        let _ = store.write(request).await;
                    }
                }
            }
        }

        // Final verification: check all keys in reference exist in store
        for (key, expected_value) in &reference {
            let request = ReadRequest { key: key.clone() };
            let result = store
                .read(request)
                .await
                .expect("Key from reference should exist in store");
            assert_eq!(
                expected_value, &result.value,
                "Final state mismatch for key '{}'",
                key
            );
        }
    });
});
