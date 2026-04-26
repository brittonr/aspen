//! Storage port trait implementations for `SharedRedbStorage`.
//!
//! These implementations bridge `SharedRedbStorage` to the Aspen-owned
//! port traits defined in `crate::storage_ports`, delegating to existing
//! inherent methods. This allows domain logic (TTL cleanup, lease cleanup,
//! chain verification) to depend on narrow port traits rather than the
//! concrete Redb storage type.

use crate::storage_ports::{ChainRead, KvStateRead, KvStateWrite, LeaseRead, LeaseWrite};
use crate::verified::ChainHash;

use super::SharedRedbStorage;
use super::SharedStorageError;

use aspen_kv_types::KeyValueWithRevision;
use aspen_storage_types::KvEntry;

impl KvStateRead for SharedRedbStorage {
    fn get(&self, key: &str) -> Result<Option<KvEntry>, SharedStorageError> {
        SharedRedbStorage::get(self, key)
    }

    fn get_with_revision(&self, key: &str) -> Result<Option<KeyValueWithRevision>, SharedStorageError> {
        SharedRedbStorage::get_with_revision(self, key)
    }

    fn scan(
        &self,
        prefix: &str,
        after_key: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<KeyValueWithRevision>, SharedStorageError> {
        SharedRedbStorage::scan(self, prefix, after_key, limit)
    }

    fn count_expired_keys(&self) -> Result<u64, SharedStorageError> {
        SharedRedbStorage::count_expired_keys(self)
    }

    fn count_keys_with_ttl(&self) -> Result<u64, SharedStorageError> {
        SharedRedbStorage::count_keys_with_ttl(self)
    }

    fn get_expired_keys_with_metadata(
        &self,
        batch_limit: u32,
    ) -> Result<Vec<(String, Option<u64>)>, SharedStorageError> {
        SharedRedbStorage::get_expired_keys_with_metadata(self, batch_limit)
    }
}

impl KvStateWrite for SharedRedbStorage {
    fn delete_expired_keys(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        SharedRedbStorage::delete_expired_keys(self, batch_limit)
    }
}

impl LeaseRead for SharedRedbStorage {
    fn get_lease(&self, lease_id: u64) -> Result<Option<(u32, u32)>, SharedStorageError> {
        SharedRedbStorage::get_lease(self, lease_id)
    }

    fn get_lease_keys(&self, lease_id: u64) -> Result<Vec<String>, SharedStorageError> {
        SharedRedbStorage::get_lease_keys(self, lease_id)
    }

    fn list_leases(&self) -> Result<Vec<(u64, u32, u32)>, SharedStorageError> {
        SharedRedbStorage::list_leases(self)
    }

    fn count_expired_leases(&self) -> Result<u64, SharedStorageError> {
        SharedRedbStorage::count_expired_leases(self)
    }

    fn count_active_leases(&self) -> Result<u64, SharedStorageError> {
        SharedRedbStorage::count_active_leases(self)
    }
}

impl LeaseWrite for SharedRedbStorage {
    fn delete_expired_leases(&self, batch_limit: u32) -> Result<u32, SharedStorageError> {
        SharedRedbStorage::delete_expired_leases(self, batch_limit)
    }
}

impl ChainRead for SharedRedbStorage {
    fn chain_tip_for_verification(&self) -> Result<(u64, ChainHash), SharedStorageError> {
        SharedRedbStorage::chain_tip_for_verification(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_redb_implements_kv_state_read() {
        fn _assert_kv_state_read<T: KvStateRead>() {}
        _assert_kv_state_read::<SharedRedbStorage>();
    }

    #[test]
    fn test_shared_redb_implements_kv_state_write() {
        fn _assert_kv_state_write<T: KvStateWrite>() {}
        _assert_kv_state_write::<SharedRedbStorage>();
    }

    #[test]
    fn test_shared_redb_implements_lease_read() {
        fn _assert_lease_read<T: LeaseRead>() {}
        _assert_lease_read::<SharedRedbStorage>();
    }

    #[test]
    fn test_shared_redb_implements_lease_write() {
        fn _assert_lease_write<T: LeaseWrite>() {}
        _assert_lease_write::<SharedRedbStorage>();
    }

    #[test]
    fn test_shared_redb_implements_chain_read() {
        fn _assert_chain_read<T: ChainRead>() {}
        _assert_chain_read::<SharedRedbStorage>();
    }

    #[test]
    fn test_port_traits_work_with_arc() {
        fn _assert_arc_kv_read<T: KvStateRead>() {}
        fn _assert_arc_lease_read<T: LeaseRead>() {}
        fn _assert_arc_chain_read<T: ChainRead>() {}

        // Arc<SharedRedbStorage> doesn't auto-impl, but &SharedRedbStorage does via deref.
        // The cleanup tasks take Arc<SharedRedbStorage> and call methods on &*storage.
        // This is fine because the port traits are on SharedRedbStorage itself.
        _assert_arc_kv_read::<SharedRedbStorage>();
        _assert_arc_lease_read::<SharedRedbStorage>();
        _assert_arc_chain_read::<SharedRedbStorage>();
    }
}
