//! Aspen-owned storage port traits for Redb-backed state.
//!
//! These traits abstract over the Redb storage layer so that:
//! - Domain logic (TTL cleanup, lease cleanup, chain verification) can be
//!   tested with in-memory implementations.
//! - The OpenRaft adapter layer owns the openraft coupling without leaking
//!   Redb types into domain consumers.
//! - The single-fsync invariant is documented at the trait level.
//!
//! ## Trait hierarchy
//!
//! ```text
//! KvStateRead ── get, get_with_revision, scan, count/list expired
//! KvStateWrite ─ delete_expired_keys (TTL cleanup)
//! LeaseRead ──── get_lease, get_lease_keys, list_leases, count
//! LeaseWrite ─── delete_expired_leases
//! ChainRead ──── chain_tip_for_verification
//! ```

use aspen_kv_types::KeyValueWithRevision;
use aspen_storage_types::KvEntry;

use crate::storage_shared::SharedStorageError;
use crate::verified::ChainHash;

/// Read-side KV state machine port.
pub trait KvStateRead: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError>;

    fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError>;

    fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError>;

    fn count_expired_keys(&self) -> Result<u64, SharedStorageError>;

    fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError>;

    fn get_expired_keys_with_metadata(
        &self,
        batch_limit: u32,
    ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError>;
}

/// Write-side KV state machine port (TTL cleanup).
pub trait KvStateWrite: Send + Sync {
    fn delete_expired_keys(&self, batch_limit: u32) -> Result<u32, SharedStorageError>;
}

/// Read-side lease state port.
pub trait LeaseRead: Send + Sync {
    fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError>;
    fn get_lease_keys(&self, lease_id: u64) -> Result<Vec<String>, SharedStorageError>;
    fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError>;
    fn count_expired_leases(&self) -> Result<u64, SharedStorageError>;
    fn count_active_leases(&self) -> Result<u64, SharedStorageError>;
}

/// Write-side lease state port (cleanup).
pub trait LeaseWrite: Send + Sync {
    fn delete_expired_leases(&self, batch_limit: u32) -> Result<u32, SharedStorageError>;
}

/// Chain integrity read port.
pub trait ChainRead: Send + Sync {
    fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    struct InMemoryKvState {
        entries: std::collections::HashMap<String, KvEntry>,
    }

    impl InMemoryKvState {
        fn new() -> Self {
            Self {
                entries: std::collections::HashMap::new(),
            }
        }

        fn insert(&mut self, key: &str, value: &str, version: i64) {
            self.entries.insert(
                key.to_string(),
                KvEntry {
                    value: value.to_string(),
                    version,
                    create_revision: 1,
                    mod_revision: version,
                    expires_at_ms: None,
                    lease_id: None,
                },
            );
        }
    }

    impl KvStateRead for InMemoryKvState {
        fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
            Ok(self.entries.get(key).cloned())
        }

        fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
            Ok(self.entries.get(key).map(|e| KeyValueWithRevision {
                key: key.to_string(),
                value: e.value.clone(),
                version: e.version as u64,
                create_revision: e.create_revision as u64,
                mod_revision: e.mod_revision as u64,
            }))
        }

        fn scan(
            &self,
            prefix: &str,
            _after_key: Option<&str>,
            limit: Option<u32>,
        ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
            let limit = limit.unwrap_or(100) as usize;
            let results: Vec<_> = self
                .entries
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .take(limit)
                .map(|(k, e)| KeyValueWithRevision {
                    key: k.clone(),
                    value: e.value.clone(),
                    version: e.version as u64,
                    create_revision: e.create_revision as u64,
                    mod_revision: e.mod_revision as u64,
                })
                .collect();
            Ok(results)
        }

        fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
            Ok(0)
        }

        fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
            Ok(self.entries.values().filter(|e| e.expires_at_ms.is_some()).count() as u64)
        }

        fn get_expired_keys_with_metadata(
            &self,
            _batch_limit: u32,
        ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
            Ok(Vec::new())
        }
    }

    impl KvStateWrite for InMemoryKvState {
        fn delete_expired_keys(&self, _batch_limit: u32) -> Result<u32, SharedStorageError> {
            Ok(0)
        }
    }

    #[test]
    fn test_kv_state_get() {
        let mut state = InMemoryKvState::new();
        state.insert("key1", "value1", 1);
        let result = state.get("key1").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().value, "value1");
    }

    #[test]
    fn test_kv_state_get_missing() {
        let state = InMemoryKvState::new();
        assert!(state.get("missing").unwrap().is_none());
    }

    #[test]
    fn test_kv_state_scan() {
        let mut state = InMemoryKvState::new();
        state.insert("prefix/a", "va", 1);
        state.insert("prefix/b", "vb", 2);
        state.insert("other/c", "vc", 3);
        let results = state.scan("prefix/", None, None).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_kv_state_revision() {
        let mut state = InMemoryKvState::new();
        state.insert("rkey", "rval", 5);
        let result = state.get_with_revision("rkey").unwrap().unwrap();
        assert_eq!(result.version, 5);
    }

    struct InMemoryLeaseState {
        leases: std::collections::HashMap<u64, (u32, u32, Vec<String>)>,
    }

    impl InMemoryLeaseState {
        fn new() -> Self {
            Self {
                leases: std::collections::HashMap::new(),
            }
        }
    }

    impl LeaseRead for InMemoryLeaseState {
        fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError> {
            Ok(self.leases.get(&lease_id).map(|(g, r, _)| (*g, *r)))
        }
        fn get_lease_keys(&self, lease_id: u64) -> Result<Vec<String>, SharedStorageError> {
            Ok(self.leases.get(&lease_id).map(|(_, _, k)| k.clone()).unwrap_or_default())
        }
        fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError> {
            Ok(self.leases.iter().map(|(id, (g, r, _))| (*id, *g, *r)).collect())
        }
        fn count_expired_leases(&self) -> Result<u64, SharedStorageError> {
            Ok(0)
        }
        fn count_active_leases(&self) -> Result<u64, SharedStorageError> {
            Ok(self.leases.len() as u64)
        }
    }

    impl LeaseWrite for InMemoryLeaseState {
        fn delete_expired_leases(&self, _batch_limit: u32) -> Result<u32, SharedStorageError> {
            Ok(0)
        }
    }

    #[test]
    fn test_lease_get_missing() {
        let leases = InMemoryLeaseState::new();
        assert!(leases.get_lease(999).unwrap().is_none());
    }

    #[test]
    fn test_lease_count() {
        let mut leases = InMemoryLeaseState::new();
        leases.leases.insert(1, (60, 60, vec![]));
        leases.leases.insert(2, (120, 100, vec![]));
        assert_eq!(leases.count_active_leases().unwrap(), 2);
    }

    struct FixedChainTip(u64, ChainHash);

    impl ChainRead for FixedChainTip {
        fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
            Ok((self.0, self.1))
        }
    }

    #[test]
    fn test_chain_tip() {
        let tip = FixedChainTip(100, [0xAA; 32]);
        let (idx, hash) = tip.chain_tip_for_verification().unwrap();
        assert_eq!(idx, 100);
        assert_eq!(hash[0], 0xAA);
    }

    // =========================================================================
    // TTL expiration and cleanup port tests
    // =========================================================================

    struct TtlAwareKvState {
        entries: std::collections::HashMap<String, KvEntry>,
        now_ms: u64,
    }

    impl TtlAwareKvState {
        fn new(now_ms: u64) -> Self {
            Self {
                entries: std::collections::HashMap::new(),
                now_ms,
            }
        }

        fn insert_with_ttl(&mut self, key: &str, value: &str, version: i64, expires_at_ms: u64) {
            self.entries.insert(
                key.to_string(),
                KvEntry {
                    value: value.to_string(),
                    version,
                    create_revision: 1,
                    mod_revision: version,
                    expires_at_ms: Some(expires_at_ms),
                    lease_id: None,
                },
            );
        }
    }

    impl KvStateRead for TtlAwareKvState {
        fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
            match self.entries.get(key) {
                Some(e) if e.expires_at_ms.is_some_and(|t| self.now_ms > t) => Ok(None),
                other => Ok(other.cloned()),
            }
        }

        fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
            self.get(key).map(|opt| {
                opt.map(|e| KeyValueWithRevision {
                    key: key.to_string(),
                    value: e.value,
                    version: e.version as u64,
                    create_revision: e.create_revision as u64,
                    mod_revision: e.mod_revision as u64,
                })
            })
        }

        fn scan(&self, prefix: &str, _after: Option<&str>, limit: Option<u32>) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
            let limit = limit.unwrap_or(100) as usize;
            let results: Vec<_> = self.entries.iter()
                .filter(|(k, e)| k.starts_with(prefix) && !e.expires_at_ms.is_some_and(|t| self.now_ms > t))
                .take(limit)
                .map(|(k, e)| KeyValueWithRevision {
                    key: k.clone(), value: e.value.clone(),
                    version: e.version as u64, create_revision: e.create_revision as u64, mod_revision: e.mod_revision as u64,
                })
                .collect();
            Ok(results)
        }

        fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
            Ok(self.entries.values().filter(|e| e.expires_at_ms.is_some_and(|t| self.now_ms > t)).count() as u64)
        }

        fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
            Ok(self.entries.values().filter(|e| e.expires_at_ms.is_some_and(|t| self.now_ms <= t)).count() as u64)
        }

        fn get_expired_keys_with_metadata(&self, batch_limit: u32) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
            let results: Vec<_> = self.entries.iter()
                .filter(|(_, e)| e.expires_at_ms.is_some_and(|t| self.now_ms > t))
                .take(batch_limit as usize)
                .map(|(k, _)| (k.clone(), None))
                .collect();
            Ok(results)
        }
    }

    impl KvStateWrite for TtlAwareKvState {
        fn delete_expired_keys(&self, _batch_limit: u32) -> Result<u32, SharedStorageError> {
            Ok(self.entries.values().filter(|e| e.expires_at_ms.is_some_and(|t| self.now_ms > t)).count() as u32)
        }
    }

    #[test]
    fn test_expired_keys_filtered_on_get() {
        let mut state = TtlAwareKvState::new(2000);
        state.insert_with_ttl("expired", "old", 1, 1000);
        state.insert_with_ttl("alive", "new", 2, 3000);

        assert!(state.get("expired").unwrap().is_none());
        assert!(state.get("alive").unwrap().is_some());
    }

    #[test]
    fn test_expired_keys_filtered_on_scan() {
        let mut state = TtlAwareKvState::new(2000);
        state.insert_with_ttl("ns/expired", "old", 1, 1000);
        state.insert_with_ttl("ns/alive", "new", 2, 3000);

        let results = state.scan("ns/", None, None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "ns/alive");
    }

    #[test]
    fn test_count_expired_keys() {
        let mut state = TtlAwareKvState::new(5000);
        state.insert_with_ttl("a", "v", 1, 1000);
        state.insert_with_ttl("b", "v", 2, 2000);
        state.insert_with_ttl("c", "v", 3, 9000);

        assert_eq!(state.count_expired_keys().unwrap(), 2);
        assert_eq!(state.count_keys_with_ttl().unwrap(), 1);
    }

    #[test]
    fn test_expired_keys_metadata_batch_limit() {
        let mut state = TtlAwareKvState::new(5000);
        for i in 0..10 {
            state.insert_with_ttl(&format!("k{i}"), "v", i as i64, 1000);
        }
        let results = state.get_expired_keys_with_metadata(3).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_delete_expired_returns_count() {
        let mut state = TtlAwareKvState::new(5000);
        state.insert_with_ttl("a", "v", 1, 1000);
        state.insert_with_ttl("b", "v", 2, 2000);
        state.insert_with_ttl("c", "v", 3, 9000);

        assert_eq!(state.delete_expired_keys(100).unwrap(), 2);
    }

    // =========================================================================
    // Scan limit enforcement
    // =========================================================================

    #[test]
    fn test_scan_respects_limit() {
        let mut state = InMemoryKvState::new();
        for i in 0..20 {
            state.insert(&format!("ns/{i:02}"), &format!("v{i}"), i as i64);
        }
        let results = state.scan("ns/", None, Some(5)).unwrap();
        assert!(results.len() <= 5);
    }

    // =========================================================================
    // Lease with attached keys
    // =========================================================================

    #[test]
    fn test_lease_keys_attached() {
        let mut leases = InMemoryLeaseState::new();
        leases.leases.insert(10, (60, 55, vec!["k1".into(), "k2".into()]));

        let keys = leases.get_lease_keys(10).unwrap();
        assert_eq!(keys, vec!["k1", "k2"]);
    }

    #[test]
    fn test_lease_keys_missing_lease() {
        let leases = InMemoryLeaseState::new();
        let keys = leases.get_lease_keys(999).unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_list_leases_multiple() {
        let mut leases = InMemoryLeaseState::new();
        leases.leases.insert(1, (60, 50, vec![]));
        leases.leases.insert(2, (120, 100, vec![]));
        leases.leases.insert(3, (30, 20, vec![]));

        let listed = leases.list_leases().unwrap();
        assert_eq!(listed.len(), 3);
    }

    // =========================================================================
    // Chain integrity: hash change detection
    // =========================================================================

    #[test]
    fn test_chain_tip_genesis() {
        let tip = FixedChainTip(0, [0u8; 32]);
        let (idx, hash) = tip.chain_tip_for_verification().unwrap();
        assert_eq!(idx, 0);
        assert_eq!(hash, [0u8; 32]);
    }

    #[test]
    fn test_chain_tip_different_hashes_detected() {
        let tip_a = FixedChainTip(100, [0xAA; 32]);
        let tip_b = FixedChainTip(100, [0xBB; 32]);

        let (_, hash_a) = tip_a.chain_tip_for_verification().unwrap();
        let (_, hash_b) = tip_b.chain_tip_for_verification().unwrap();
        assert_ne!(hash_a, hash_b);
    }

    // =========================================================================
    // Error-producing fixtures: simulated storage failures
    // =========================================================================

    struct FailingKvState;

    impl KvStateRead for FailingKvState {
        fn get(&self, _key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated read failure".into() })
        }
        fn get_with_revision(&self, _key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated read failure".into() })
        }
        fn scan(&self, _prefix: &str, _after: Option<&str>, _limit: Option<u32>) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated scan failure".into() })
        }
        fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated count failure".into() })
        }
        fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated count failure".into() })
        }
        fn get_expired_keys_with_metadata(&self, _batch_limit: u32) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated metadata failure".into() })
        }
    }

    impl KvStateWrite for FailingKvState {
        fn delete_expired_keys(&self, _batch_limit: u32) -> Result<u32, SharedStorageError> {
            Err(SharedStorageError::Internal { reason: "simulated write failure".into() })
        }
    }

    #[test]
    fn test_failing_kv_state_read_returns_error() {
        let state = FailingKvState;
        assert!(state.get("any").is_err());
        assert!(state.scan("prefix", None, None).is_err());
        assert!(state.count_expired_keys().is_err());
    }

    #[test]
    fn test_failing_kv_state_write_returns_error() {
        let state = FailingKvState;
        assert!(state.delete_expired_keys(100).is_err());
    }

    struct FailingChainRead;

    impl ChainRead for FailingChainRead {
        fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
            Err(SharedStorageError::LockPoisoned { context: "simulated lock failure".into() })
        }
    }

    #[test]
    fn test_failing_chain_read() {
        let chain = FailingChainRead;
        assert!(chain.chain_tip_for_verification().is_err());
    }

    // =========================================================================
    // Compile-time proofs: port traits are independent of OpenRaft/Redb
    // =========================================================================

    fn _assert_kv_ports_independent<S: KvStateRead + KvStateWrite>(_s: &S) {}
    fn _assert_lease_ports_independent<S: LeaseRead + LeaseWrite>(_s: &S) {}
    fn _assert_chain_port_independent<S: ChainRead>(_s: &S) {}

    #[test]
    fn test_ports_compile_without_redb_or_openraft() {
        let kv = InMemoryKvState::new();
        _assert_kv_ports_independent(&kv);

        let leases = InMemoryLeaseState::new();
        _assert_lease_ports_independent(&leases);

        let chain = FixedChainTip(0, [0u8; 32]);
        _assert_chain_port_independent(&chain);
    }
}
