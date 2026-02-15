//! Cache management operations for the DNS client.

use super::super::types::RecordType;
use super::AspenDnsClient;

impl AspenDnsClient {
    /// Invalidate a specific record from the cache.
    ///
    /// The record will be refreshed on the next sync.
    pub async fn invalidate(&self, domain: &str, record_type: RecordType) {
        let mut cache = self.cache.write().await;
        let key = Self::cache_key(domain, record_type);
        cache.remove(&key);
    }

    /// Invalidate all records for a domain.
    pub async fn invalidate_domain(&self, domain: &str) {
        let mut cache = self.cache.write().await;
        let domain_lower = domain.to_lowercase();
        cache.retain(|k, _| !k.starts_with(&format!("{}:", domain_lower)));
    }

    /// Clear the entire cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the total number of cached records.
    pub async fn cache_size(&self) -> usize {
        self.cache.read().await.len()
    }
}
