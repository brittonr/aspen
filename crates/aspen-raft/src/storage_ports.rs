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
}
