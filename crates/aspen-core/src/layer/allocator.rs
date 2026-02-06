//! High-Contention Allocator (HCA) for directory prefix allocation.
//!
//! This module implements a FoundationDB-style High-Contention Allocator that
//! efficiently allocates unique integer prefixes even under high concurrent load.
//!
//! # Algorithm
//!
//! The HCA uses a sliding window approach to reduce contention:
//!
//! 1. Maintain a window of candidate integers [window_start, window_start + window_size)
//! 2. Randomly select candidates within the window and try to claim them with CAS
//! 3. If all candidates fail, atomically advance the window
//! 4. Use exponential backoff on conflicts
//!
//! # Window Sizing
//!
//! Window size increases as the counter grows:
//! - counter < 255: window = 64
//! - counter < 65535: window = 1024
//! - counter >= 65535: window = 8192
//!
//! # References
//!
//! - [High-Contention Allocator](https://ananthakumaran.in/2018/08/05/high-contention-allocator.html)
//! - [FoundationDB Directory Layer](https://apple.github.io/foundationdb/developer-guide.html)

use std::sync::Arc;
use std::time::Duration;

use rand::Rng;
use snafu::ResultExt;
use snafu::Snafu;

use super::subspace::Subspace;
use super::tuple::Element;
use super::tuple::Tuple;
use crate::constants::CAS_RETRY_INITIAL_BACKOFF_MS;
use crate::constants::CAS_RETRY_MAX_BACKOFF_MS;
use crate::constants::MAX_CAS_RETRIES;
use crate::constants::directory::DIR_HCA_PREFIX;
use crate::constants::directory::DIRECTORY_PREFIX;
use crate::constants::directory::HCA_CANDIDATES_PREFIX;
use crate::constants::directory::HCA_COUNTER_KEY;
use crate::constants::directory::HCA_INITIAL_WINDOW_SIZE;
use crate::constants::directory::HCA_LARGE_WINDOW_THRESHOLD;
use crate::constants::directory::HCA_MAX_CANDIDATES_PER_ATTEMPT;
use crate::constants::directory::HCA_MAX_WINDOW_SIZE;
use crate::constants::directory::HCA_MEDIUM_WINDOW_SIZE;
use crate::constants::directory::HCA_MEDIUM_WINDOW_THRESHOLD;
use crate::constants::directory::HCA_WINDOW_START_KEY;
use crate::error::KeyValueStoreError;
use crate::kv::CompareOp;
use crate::kv::CompareTarget;
use crate::kv::ReadRequest;
use crate::kv::TxnCompare;
use crate::kv::TxnOp;
use crate::kv::WriteCommand;
use crate::kv::WriteRequest;
use crate::traits::KeyValueStore;

// =============================================================================
// Error Types
// =============================================================================

/// Errors that can occur during prefix allocation.
#[derive(Debug, Snafu)]
pub enum AllocationError {
    /// Maximum retries exceeded during allocation.
    #[snafu(display("allocation failed after {attempts} attempts"))]
    MaxRetriesExceeded {
        /// Number of attempts made before giving up.
        attempts: u32,
    },

    /// Window advance failed due to contention.
    #[snafu(display("window advance failed: {reason}"))]
    WindowAdvanceFailed {
        /// Description of the failure.
        reason: String,
    },

    /// Counter has reached maximum value.
    #[snafu(display("prefix counter exhausted"))]
    CounterExhausted,

    /// Invalid state read from storage.
    #[snafu(display("corrupted allocator state: {reason}"))]
    CorruptedState {
        /// Description of the corruption.
        reason: String,
    },

    /// Storage error during allocation.
    #[snafu(display("storage error: {source}"))]
    Storage {
        /// The underlying KV store error.
        source: KeyValueStoreError,
    },
}

// =============================================================================
// HCA State
// =============================================================================

/// Current state of the High-Contention Allocator.
#[derive(Debug, Clone, Default)]
struct HcaState {
    /// Global counter (next guaranteed-unique value).
    counter: u64,
    /// Start of current allocation window.
    window_start: u64,
}

/// Result of trying to claim a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimResult {
    /// Successfully claimed the candidate.
    Success,
    /// Candidate was already claimed by another allocator.
    AlreadyClaimed,
}

// =============================================================================
// High-Contention Allocator
// =============================================================================

/// High-Contention Allocator for efficient prefix allocation.
///
/// This allocator uses a window-based approach with random candidate selection
/// to minimize conflicts under concurrent load.
///
/// # Example
///
/// ```ignore
/// use aspen_core::layer::HighContentionAllocator;
///
/// let allocator = HighContentionAllocator::new(kv_store.clone());
///
/// // Allocate a unique prefix
/// let prefix_int = allocator.allocate().await?;
/// let prefix_bytes = allocator.encode_prefix(prefix_int);
/// ```
pub struct HighContentionAllocator<KV: KeyValueStore + ?Sized> {
    /// Underlying KV store.
    store: Arc<KV>,
    /// Subspace for HCA state storage.
    hca_subspace: Subspace,
}

impl<KV: KeyValueStore + ?Sized> HighContentionAllocator<KV> {
    /// Create a new High-Contention Allocator.
    ///
    /// The allocator stores its state under the directory prefix (0xFE).
    pub fn new(store: Arc<KV>) -> Self {
        // Create subspace under (0xFE, "hca")
        let hca_subspace = Subspace::new(Tuple::new().push(vec![DIRECTORY_PREFIX]).push(DIR_HCA_PREFIX));

        Self { store, hca_subspace }
    }

    /// Allocate a unique integer prefix.
    ///
    /// This is the main entry point for allocation. It implements the full
    /// HCA algorithm with window management and retry logic.
    ///
    /// # Returns
    ///
    /// A unique u64 value that can be encoded as a binary prefix.
    ///
    /// # Errors
    ///
    /// Returns `AllocationError::MaxRetriesExceeded` if allocation fails
    /// after the maximum number of attempts (100).
    pub async fn allocate(&self) -> Result<u64, AllocationError> {
        let mut attempt = 0u32;
        let mut backoff_ms = CAS_RETRY_INITIAL_BACKOFF_MS;

        loop {
            // Step 1: Read current HCA state
            let state = self.read_state().await?;

            // Step 2: Calculate window size based on counter
            let window_size = calculate_window_size(state.window_start);
            let window_end = state.window_start.saturating_add(window_size);

            // Step 3: Try random candidates within the window
            // Generate all random offsets upfront to avoid holding RNG across await
            let candidates: Vec<u64> = {
                let mut rng = rand::rng();
                (0..HCA_MAX_CANDIDATES_PER_ATTEMPT)
                    .map(|_| state.window_start.saturating_add(rng.random_range(0..window_size)))
                    .collect()
            };

            for candidate in candidates {
                match self.try_claim_candidate(candidate).await? {
                    ClaimResult::Success => return Ok(candidate),
                    ClaimResult::AlreadyClaimed => continue,
                }
            }

            // Step 4: All candidates exhausted - advance window
            let new_window_start = state.counter.max(window_end);
            let new_counter = new_window_start.saturating_add(window_size);

            // Check for overflow
            if new_counter < new_window_start {
                return Err(AllocationError::CounterExhausted);
            }

            match self.advance_window(&state, new_window_start, new_counter).await {
                Ok(()) => {
                    // Window advanced successfully, retry with new window
                    continue;
                }
                Err(AllocationError::WindowAdvanceFailed { .. }) => {
                    // Another allocator advanced the window, retry
                    attempt += 1;
                    if attempt >= MAX_CAS_RETRIES {
                        return Err(AllocationError::MaxRetriesExceeded { attempts: attempt });
                    }
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(CAS_RETRY_MAX_BACKOFF_MS);
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Encode an allocated integer as a variable-length binary prefix.
    ///
    /// Uses the Tuple layer's integer encoding for lexicographic ordering.
    pub fn encode_prefix(&self, value: u64) -> Vec<u8> {
        Tuple::new().push(value as i64).pack()
    }

    /// Decode a binary prefix back to an integer.
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes don't represent a valid tuple-encoded integer.
    pub fn decode_prefix(&self, bytes: &[u8]) -> Result<u64, AllocationError> {
        let tuple = Tuple::unpack(bytes).map_err(|e| AllocationError::CorruptedState {
            reason: format!("invalid prefix encoding: {e}"),
        })?;

        match tuple.get(0) {
            Some(Element::Int(n)) if *n >= 0 => Ok(*n as u64),
            Some(Element::Bytes(b)) if b.len() == 8 => {
                // Handle large u64 values stored as bytes
                let arr: [u8; 8] = b.as_slice().try_into().map_err(|_| AllocationError::CorruptedState {
                    reason: "invalid u64 bytes".to_string(),
                })?;
                Ok(u64::from_be_bytes(arr))
            }
            _ => Err(AllocationError::CorruptedState {
                reason: "prefix is not a valid integer".to_string(),
            }),
        }
    }

    // -------------------------------------------------------------------------
    // Internal Methods
    // -------------------------------------------------------------------------

    /// Read the current HCA state from storage.
    async fn read_state(&self) -> Result<HcaState, AllocationError> {
        let counter_key = self.key_for(HCA_COUNTER_KEY);
        let window_key = self.key_for(HCA_WINDOW_START_KEY);

        // Read counter
        let counter = match self.store.read(ReadRequest::new(counter_key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value_str.is_empty() {
                    0
                } else {
                    value_str.parse::<u64>().map_err(|_| AllocationError::CorruptedState {
                        reason: format!("counter is not a valid u64: {value_str}"),
                    })?
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => 0,
            Err(e) => return Err(AllocationError::Storage { source: e }),
        };

        // Read window_start
        let window_start = match self.store.read(ReadRequest::new(window_key.clone())).await {
            Ok(result) => {
                let value_str = result.kv.map(|kv| kv.value).unwrap_or_default();
                if value_str.is_empty() {
                    0
                } else {
                    value_str.parse::<u64>().map_err(|_| AllocationError::CorruptedState {
                        reason: format!("window_start is not a valid u64: {value_str}"),
                    })?
                }
            }
            Err(KeyValueStoreError::NotFound { .. }) => 0,
            Err(e) => return Err(AllocationError::Storage { source: e }),
        };

        Ok(HcaState { counter, window_start })
    }

    /// Try to claim a specific candidate using CAS.
    async fn try_claim_candidate(&self, candidate: u64) -> Result<ClaimResult, AllocationError> {
        let key = self.candidate_key(candidate);

        match self
            .store
            .write(WriteRequest {
                command: WriteCommand::CompareAndSwap {
                    key,
                    expected: None,           // Must not exist
                    new_value: String::new(), // Empty marker value
                },
            })
            .await
        {
            Ok(_) => Ok(ClaimResult::Success),
            Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => Ok(ClaimResult::AlreadyClaimed),
            Err(e) => Err(AllocationError::Storage { source: e }),
        }
    }

    /// Advance the allocation window atomically.
    async fn advance_window(
        &self,
        old_state: &HcaState,
        new_window_start: u64,
        new_counter: u64,
    ) -> Result<(), AllocationError> {
        let counter_key = self.key_for(HCA_COUNTER_KEY);
        let window_key = self.key_for(HCA_WINDOW_START_KEY);

        // Build comparison conditions
        let mut compare = Vec::with_capacity(2);

        // Compare counter value (either matches or key doesn't exist)
        if old_state.counter == 0 {
            compare.push(TxnCompare {
                key: counter_key.clone(),
                target: CompareTarget::Version,
                op: CompareOp::Equal,
                value: "0".to_string(), // Version 0 = doesn't exist
            });
        } else {
            compare.push(TxnCompare {
                key: counter_key.clone(),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: old_state.counter.to_string(),
            });
        }

        // Compare window_start value
        if old_state.window_start == 0 {
            compare.push(TxnCompare {
                key: window_key.clone(),
                target: CompareTarget::Version,
                op: CompareOp::Equal,
                value: "0".to_string(),
            });
        } else {
            compare.push(TxnCompare {
                key: window_key.clone(),
                target: CompareTarget::Value,
                op: CompareOp::Equal,
                value: old_state.window_start.to_string(),
            });
        }

        // Build success operations
        let success = vec![
            TxnOp::Put {
                key: counter_key,
                value: new_counter.to_string(),
            },
            TxnOp::Put {
                key: window_key,
                value: new_window_start.to_string(),
            },
        ];

        // Execute transaction
        let result = self
            .store
            .write(WriteRequest {
                command: WriteCommand::Transaction {
                    compare,
                    success,
                    failure: vec![],
                },
            })
            .await
            .context(StorageSnafu)?;

        match result.succeeded {
            Some(true) => Ok(()),
            _ => Err(AllocationError::WindowAdvanceFailed {
                reason: "state changed by another allocator".to_string(),
            }),
        }
    }

    /// Generate a key for an HCA field.
    fn key_for(&self, field: &str) -> String {
        String::from_utf8_lossy(&self.hca_subspace.pack(&Tuple::new().push(field))).to_string()
    }

    /// Generate a key for a candidate marker.
    fn candidate_key(&self, candidate: u64) -> String {
        String::from_utf8_lossy(
            &self.hca_subspace.pack(&Tuple::new().push(HCA_CANDIDATES_PREFIX).push(candidate as i64)),
        )
        .to_string()
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Calculate the window size based on the current position.
///
/// Window size increases as allocations progress to maintain efficiency:
/// - start < 255: small window (64) for low contention scenarios
/// - start < 65535: medium window (1024) for moderate contention
/// - start >= 65535: large window (8192) for high contention
fn calculate_window_size(start: u64) -> u64 {
    if start < HCA_MEDIUM_WINDOW_THRESHOLD {
        HCA_INITIAL_WINDOW_SIZE
    } else if start < HCA_LARGE_WINDOW_THRESHOLD {
        HCA_MEDIUM_WINDOW_SIZE
    } else {
        HCA_MAX_WINDOW_SIZE
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::inmemory::DeterministicKeyValueStore;

    #[tokio::test]
    async fn test_allocate_sequential() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        let id1 = allocator.allocate().await.unwrap();
        let id2 = allocator.allocate().await.unwrap();
        let id3 = allocator.allocate().await.unwrap();

        // All IDs should be unique
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[tokio::test]
    async fn test_allocate_uniqueness() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        let mut ids = HashSet::new();
        for _ in 0..100 {
            let id = allocator.allocate().await.unwrap();
            assert!(!ids.contains(&id), "Duplicate ID: {id}");
            ids.insert(id);
        }

        assert_eq!(ids.len(), 100);
    }

    #[tokio::test]
    async fn test_encode_decode_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Test various values
        for value in [0u64, 1, 127, 128, 255, 256, 65535, 65536, u64::MAX / 2] {
            let encoded = allocator.encode_prefix(value);
            let decoded = allocator.decode_prefix(&encoded).unwrap();
            assert_eq!(value, decoded, "roundtrip failed for value {value}");
        }
    }

    #[tokio::test]
    async fn test_window_size_calculation() {
        // Small window for low values
        assert_eq!(calculate_window_size(0), HCA_INITIAL_WINDOW_SIZE);
        assert_eq!(calculate_window_size(100), HCA_INITIAL_WINDOW_SIZE);
        assert_eq!(calculate_window_size(254), HCA_INITIAL_WINDOW_SIZE);

        // Medium window for mid-range values
        assert_eq!(calculate_window_size(255), HCA_MEDIUM_WINDOW_SIZE);
        assert_eq!(calculate_window_size(1000), HCA_MEDIUM_WINDOW_SIZE);
        assert_eq!(calculate_window_size(65534), HCA_MEDIUM_WINDOW_SIZE);

        // Large window for high values
        assert_eq!(calculate_window_size(65535), HCA_MAX_WINDOW_SIZE);
        assert_eq!(calculate_window_size(100_000), HCA_MAX_WINDOW_SIZE);
    }

    #[tokio::test]
    async fn test_prefix_ordering() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Verify that encoded prefixes maintain lexicographic ordering
        let values = [0u64, 1, 100, 255, 256, 1000, 65535, 65536];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| allocator.encode_prefix(v)).collect();

        for i in 1..encoded.len() {
            assert!(encoded[i - 1] < encoded[i], "ordering failed: {} should be < {}", values[i - 1], values[i]);
        }
    }

    #[tokio::test]
    async fn test_concurrent_allocation() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = Arc::new(HighContentionAllocator::new(store));

        // Spawn multiple concurrent allocators
        let handles: Vec<_> = (0..5)
            .map(|_| {
                let alloc = allocator.clone();
                tokio::spawn(async move {
                    let mut ids = Vec::new();
                    for _ in 0..20 {
                        ids.push(alloc.allocate().await.unwrap());
                    }
                    ids
                })
            })
            .collect();

        // Collect all IDs and verify uniqueness
        let mut all_ids = HashSet::new();
        for handle in handles {
            let ids = handle.await.unwrap();
            for id in ids {
                assert!(!all_ids.contains(&id), "Duplicate ID: {id}");
                all_ids.insert(id);
            }
        }

        // Should have 100 unique IDs (5 tasks * 20 allocations)
        assert_eq!(all_ids.len(), 100);
    }

    // =========================================================================
    // Window Size Boundary Tests
    // =========================================================================

    #[test]
    fn test_window_size_boundary_small_to_medium() {
        // Exactly at threshold
        assert_eq!(calculate_window_size(254), HCA_INITIAL_WINDOW_SIZE);
        assert_eq!(calculate_window_size(255), HCA_MEDIUM_WINDOW_SIZE);
    }

    #[test]
    fn test_window_size_boundary_medium_to_large() {
        // Exactly at threshold
        assert_eq!(calculate_window_size(65534), HCA_MEDIUM_WINDOW_SIZE);
        assert_eq!(calculate_window_size(65535), HCA_MAX_WINDOW_SIZE);
    }

    #[test]
    fn test_window_size_large_values() {
        // Very large values should use max window
        assert_eq!(calculate_window_size(1_000_000), HCA_MAX_WINDOW_SIZE);
        assert_eq!(calculate_window_size(u64::MAX / 2), HCA_MAX_WINDOW_SIZE);
    }

    // =========================================================================
    // Encode/Decode Edge Cases
    // =========================================================================

    #[tokio::test]
    async fn test_encode_decode_zero() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        let encoded = allocator.encode_prefix(0);
        let decoded = allocator.decode_prefix(&encoded).unwrap();
        assert_eq!(decoded, 0);
    }

    #[tokio::test]
    async fn test_encode_decode_max_i64() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // i64::MAX as u64 should work
        let value = i64::MAX as u64;
        let encoded = allocator.encode_prefix(value);
        let decoded = allocator.decode_prefix(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[tokio::test]
    async fn test_decode_invalid_prefix() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Empty bytes should fail
        let result = allocator.decode_prefix(&[]);
        assert!(result.is_err());

        // Invalid tuple encoding
        let result = allocator.decode_prefix(&[0xFF, 0xFF, 0xFF]);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prefix_ordering_across_boundaries() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Test ordering across window size boundaries
        let values = [0u64, 254, 255, 256, 65534, 65535, 65536, 100_000];
        let encoded: Vec<Vec<u8>> = values.iter().map(|&v| allocator.encode_prefix(v)).collect();

        for i in 1..encoded.len() {
            assert!(
                encoded[i - 1] < encoded[i],
                "ordering failed at boundary: {} should be < {}",
                values[i - 1],
                values[i]
            );
        }
    }

    // =========================================================================
    // AllocationError Tests
    // =========================================================================

    #[test]
    fn test_allocation_error_display_max_retries() {
        let err = AllocationError::MaxRetriesExceeded { attempts: 100 };
        let display = format!("{}", err);
        assert!(display.contains("100"));
        assert!(display.contains("attempts"));
    }

    #[test]
    fn test_allocation_error_display_window_advance_failed() {
        let err = AllocationError::WindowAdvanceFailed {
            reason: "contention detected".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("contention detected"));
    }

    #[test]
    fn test_allocation_error_display_counter_exhausted() {
        let err = AllocationError::CounterExhausted;
        let display = format!("{}", err);
        assert!(display.contains("exhausted"));
    }

    #[test]
    fn test_allocation_error_display_corrupted_state() {
        let err = AllocationError::CorruptedState {
            reason: "invalid format".to_string(),
        };
        let display = format!("{}", err);
        assert!(display.contains("corrupted"));
        assert!(display.contains("invalid format"));
    }

    #[test]
    fn test_allocation_error_debug() {
        let err = AllocationError::MaxRetriesExceeded { attempts: 50 };
        let debug = format!("{:?}", err);
        assert!(debug.contains("MaxRetriesExceeded"));
        assert!(debug.contains("50"));
    }

    // =========================================================================
    // HcaState Tests
    // =========================================================================

    #[test]
    fn test_hca_state_default() {
        let state = HcaState::default();
        assert_eq!(state.counter, 0);
        assert_eq!(state.window_start, 0);
    }

    #[test]
    fn test_hca_state_clone() {
        let state = HcaState {
            counter: 100,
            window_start: 50,
        };
        let cloned = state.clone();
        assert_eq!(state.counter, cloned.counter);
        assert_eq!(state.window_start, cloned.window_start);
    }

    #[test]
    fn test_hca_state_debug() {
        let state = HcaState {
            counter: 42,
            window_start: 10,
        };
        let debug = format!("{:?}", state);
        assert!(debug.contains("HcaState"));
        assert!(debug.contains("42"));
        assert!(debug.contains("10"));
    }

    // =========================================================================
    // ClaimResult Tests
    // =========================================================================

    #[test]
    fn test_claim_result_equality() {
        assert_eq!(ClaimResult::Success, ClaimResult::Success);
        assert_eq!(ClaimResult::AlreadyClaimed, ClaimResult::AlreadyClaimed);
        assert_ne!(ClaimResult::Success, ClaimResult::AlreadyClaimed);
    }

    #[test]
    fn test_claim_result_copy() {
        let result = ClaimResult::Success;
        let copied: ClaimResult = result; // Copy
        assert_eq!(result, copied);
    }

    #[test]
    fn test_claim_result_debug() {
        assert_eq!(format!("{:?}", ClaimResult::Success), "Success");
        assert_eq!(format!("{:?}", ClaimResult::AlreadyClaimed), "AlreadyClaimed");
    }

    // =========================================================================
    // Large Scale Allocation Tests
    // =========================================================================

    #[tokio::test]
    async fn test_allocate_many_sequential() {
        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        let mut ids = HashSet::new();
        // Allocate 300 IDs to cross multiple window boundaries
        for _ in 0..300 {
            let id = allocator.allocate().await.unwrap();
            assert!(!ids.contains(&id), "Duplicate ID: {id}");
            ids.insert(id);
        }

        assert_eq!(ids.len(), 300);
    }

    // =========================================================================
    // Error Path Tests
    // =========================================================================

    #[tokio::test]
    async fn test_decode_prefix_from_bytes_element() {
        use crate::layer::tuple::Tuple;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Create a tuple with 8 bytes representing a u64 value
        let value: u64 = 12345678;
        let bytes = value.to_be_bytes().to_vec();
        let tuple = Tuple::new().push(bytes);
        let packed = tuple.pack();

        // This should decode using the Element::Bytes fallback path
        let decoded = allocator.decode_prefix(&packed).unwrap();
        assert_eq!(decoded, value);
    }

    #[tokio::test]
    async fn test_decode_prefix_invalid_bytes_length() {
        use crate::layer::tuple::Tuple;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Create a tuple with bytes of wrong length (not 8 bytes)
        let bytes = vec![1u8, 2, 3]; // Only 3 bytes, not 8
        let tuple = Tuple::new().push(bytes);
        let packed = tuple.pack();

        // Should fail because bytes aren't 8 bytes long
        let result = allocator.decode_prefix(&packed);
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::CorruptedState { reason } => {
                assert!(reason.contains("not a valid integer"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decode_prefix_non_integer_element() {
        use crate::layer::tuple::Tuple;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Create a tuple with a string (not bytes or int)
        let tuple = Tuple::new().push("not_a_number");
        let packed = tuple.pack();

        // Should fail
        let result = allocator.decode_prefix(&packed);
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::CorruptedState { reason } => {
                assert!(reason.contains("not a valid integer"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_decode_prefix_negative_integer() {
        use crate::layer::tuple::Tuple;

        let store = Arc::new(DeterministicKeyValueStore::new());
        let allocator = HighContentionAllocator::new(store);

        // Create a tuple with a negative integer
        let tuple = Tuple::new().push(-5i64);
        let packed = tuple.pack();

        // Should fail because prefix must be non-negative
        let result = allocator.decode_prefix(&packed);
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::CorruptedState { reason } => {
                assert!(reason.contains("not a valid integer"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_corrupted_counter_value() {
        use crate::constants::directory::DIR_HCA_PREFIX;
        use crate::constants::directory::DIRECTORY_PREFIX;
        use crate::kv::WriteRequest;

        let store = DeterministicKeyValueStore::new();

        // Create the same subspace the allocator uses
        let hca_subspace = Subspace::new(Tuple::new().push(vec![DIRECTORY_PREFIX]).push(DIR_HCA_PREFIX));
        let counter_key = String::from_utf8_lossy(&hca_subspace.pack(&Tuple::new().push(HCA_COUNTER_KEY))).to_string();

        // Corrupt the counter by writing an invalid value
        store.write(WriteRequest::set(&counter_key, "not_a_number")).await.unwrap();

        let allocator = HighContentionAllocator::new(store.clone());

        // Allocate should fail due to corrupted counter
        let result = allocator.allocate().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::CorruptedState { reason } => {
                assert!(reason.contains("counter"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_corrupted_window_start_value() {
        use crate::constants::directory::DIR_HCA_PREFIX;
        use crate::constants::directory::DIRECTORY_PREFIX;
        use crate::kv::WriteRequest;

        let store = DeterministicKeyValueStore::new();

        // Create the same subspace the allocator uses
        let hca_subspace = Subspace::new(Tuple::new().push(vec![DIRECTORY_PREFIX]).push(DIR_HCA_PREFIX));
        let counter_key = String::from_utf8_lossy(&hca_subspace.pack(&Tuple::new().push(HCA_COUNTER_KEY))).to_string();
        let window_key =
            String::from_utf8_lossy(&hca_subspace.pack(&Tuple::new().push(HCA_WINDOW_START_KEY))).to_string();

        // Write a valid counter but corrupted window_start
        store.write(WriteRequest::set(&counter_key, "100")).await.unwrap();
        store.write(WriteRequest::set(&window_key, "invalid")).await.unwrap();

        let allocator = HighContentionAllocator::new(store.clone());

        // Allocate should fail due to corrupted window_start
        let result = allocator.allocate().await;
        assert!(result.is_err());
        match result.unwrap_err() {
            AllocationError::CorruptedState { reason } => {
                assert!(reason.contains("window_start"));
            }
            other => panic!("unexpected error: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_storage_error_display() {
        let kv_error = KeyValueStoreError::Failed {
            reason: "test error".to_string(),
        };
        let err = AllocationError::Storage { source: kv_error };
        let display = format!("{}", err);
        assert!(display.contains("storage") || display.contains("Storage"));
    }
}
