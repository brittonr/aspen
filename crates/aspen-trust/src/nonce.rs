//! Counter-based nonce generation for AEAD encryption.
//!
//! Each nonce is 12 bytes: `[node_id: 4 bytes (big-endian)][counter: 8 bytes (big-endian)]`.
//! The counter is monotonically increasing per node, guaranteeing uniqueness
//! across nodes (different node_id prefix) and over time (incrementing counter).
//!
//! The counter is persisted to prevent nonce reuse after restart.

use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use snafu::Snafu;

/// Errors from nonce generation.
#[derive(Debug, Snafu)]
pub enum NonceError {
    /// Counter space for this node/key has been exhausted.
    #[snafu(display("nonce counter exhausted for node {node_id}"))]
    Exhausted { node_id: u32 },
}

/// Counter-based nonce generator.
///
/// Thread-safe via atomic counter. The caller is responsible for persisting
/// the counter value to durable storage periodically or on each increment.
pub struct NonceGenerator {
    /// Node identifier (first 4 bytes of nonce).
    node_id: u32,
    /// Monotonically increasing counter (last 8 bytes of nonce).
    counter: AtomicU64,
}

impl NonceGenerator {
    /// Create a new nonce generator.
    ///
    /// `initial_counter` should be loaded from persistent storage (or 0 for
    /// a fresh node). The generator starts from `initial_counter + 1` to
    /// avoid reusing the last persisted value after a crash.
    pub fn new(node_id: u32, initial_counter: u64) -> Self {
        Self {
            node_id,
            counter: AtomicU64::new(initial_counter.saturating_add(1)),
        }
    }

    /// Generate the next unique 12-byte nonce.
    ///
    /// Returns `(nonce, counter_value)` — the caller should persist
    /// `counter_value` to durable storage.
    pub fn next_nonce(&self) -> Result<([u8; 12], u64), NonceError> {
        let mut counter = self.counter.load(Ordering::SeqCst);
        loop {
            if counter == u64::MAX {
                return Err(NonceError::Exhausted { node_id: self.node_id });
            }

            let next_counter = counter.saturating_add(1);
            match self.counter.compare_exchange(counter, next_counter, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(observed) => counter = observed,
            }
        }

        let mut nonce = [0u8; 12];
        nonce[..4].copy_from_slice(&self.node_id.to_be_bytes());
        nonce[4..12].copy_from_slice(&counter.to_be_bytes());
        Ok((nonce, counter))
    }

    /// Current counter value (for persistence).
    pub fn current_counter(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counter_from_nonce_bytes(nonce: [u8; 12]) -> u64 {
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&nonce[4..12]);
        u64::from_be_bytes(counter_bytes)
    }

    #[test]
    fn test_sequential_nonces_are_unique() {
        let nonce_gen = NonceGenerator::new(1, 0);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..1000 {
            let (nonce, _) = nonce_gen.next_nonce().unwrap();
            assert!(seen.insert(nonce), "duplicate nonce generated");
        }
    }

    #[test]
    fn test_different_node_ids_produce_different_nonces() {
        let nonce_gen1 = NonceGenerator::new(1, 0);
        let nonce_gen2 = NonceGenerator::new(2, 0);

        let (n1, _) = nonce_gen1.next_nonce().unwrap();
        let (n2, _) = nonce_gen2.next_nonce().unwrap();
        assert_ne!(n1, n2);
    }

    #[test]
    fn test_nonce_structure() {
        let nonce_gen = NonceGenerator::new(0x0102_0304, 99);

        let (nonce, counter) = nonce_gen.next_nonce().unwrap();
        assert_eq!(counter, 100); // initial_counter(99) + 1, then fetch_add returns 100
        assert_eq!(&nonce[..4], &[0x01, 0x02, 0x03, 0x04]);
        // Counter bytes
        let counter_bytes = counter_from_nonce_bytes(nonce);
        assert_eq!(counter_bytes, 100);
    }

    #[test]
    fn test_counter_survives_restart_simulation() {
        let gen1 = NonceGenerator::new(1, 0);
        // Generate a few nonces
        for _ in 0..10 {
            gen1.next_nonce().unwrap();
        }
        let persisted = gen1.current_counter();

        // "Restart" with persisted counter
        let gen2 = NonceGenerator::new(1, persisted);
        let (n1, _) = gen2.next_nonce().unwrap();

        // Verify no overlap with gen1's range
        // gen1 used counters 1..11, gen2 starts at persisted+1
        let counter_from_nonce = counter_from_nonce_bytes(n1);
        assert!(counter_from_nonce > 10);
    }

    #[test]
    fn test_counter_exhaustion_is_reported_without_wrap() {
        // new(1, MAX-2) starts counter at MAX-2+1 = MAX-1.
        // First generation returns MAX-1, then marks the generator exhausted.
        let initial_counter = (u64::MAX).saturating_sub(2);
        let nonce_gen = NonceGenerator::new(1, initial_counter);
        let (_, c1) = nonce_gen.next_nonce().unwrap();
        assert_eq!(c1, (u64::MAX).saturating_sub(1));
        assert!(nonce_gen.next_nonce().is_err());
        assert_eq!(nonce_gen.current_counter(), u64::MAX);
    }
}
