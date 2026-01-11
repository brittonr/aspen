//! DNS client for read-only synchronization and local queries.
//!
//! The DNS client maintains a local cache of DNS records synced from
//! an Aspen cluster via iroh-docs. It provides fast, local-first DNS
//! resolution without network round-trips.
//!
//! # Architecture
//!
//! ```text
//! Aspen Cluster (Raft KV with dns:* keys)
//!         |
//!         v
//! DocsExporter (exports to iroh-docs namespace)
//!         |
//!         v
//! iroh-docs P2P sync (range-based set reconciliation)
//!         |
//!         v
//! AspenDnsClient (local cache + event stream)
//!         |
//!         v
//! Application (instant local lookups)
//! ```
//!
//! # Example
//!
//! ```ignore
//! use aspen::dns::{AspenDnsClient, DnsClientTicket, RecordType};
//!
//! // Connect using a ticket
//! let ticket = DnsClientTicket::deserialize("aspendns...")?;
//! let client = AspenDnsClient::builder()
//!     .ticket(ticket)
//!     .build()
//!     .await?;
//!
//! // Wait for initial sync
//! client.wait_synced().await?;
//!
//! // Fast local lookups
//! if let Some(addrs) = client.lookup_a("api.example.com").await {
//!     println!("A records: {:?}", addrs);
//! }
//! ```

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio::sync::broadcast;
use tracing::debug;
use tracing::warn;

use super::constants::DNS_KEY_PREFIX;
use super::constants::MAX_CACHED_RECORDS;
use super::constants::MAX_SUBSCRIBERS;
use super::constants::STALENESS_THRESHOLD;
use super::error::CacheCapacitySnafu;
use super::error::DnsClientError;
use super::error::DnsClientResult;
use super::ticket::DnsClientTicket;
use super::types::DnsEvent;
use super::types::DnsRecord;
use super::types::DnsRecordData;
use super::types::MxRecord;
use super::types::RecordType;
use super::types::SrvRecord;
use super::types::SyncStatus;
use super::validation::wildcard_parent;

/// Statistics about the DNS client cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cached records.
    pub record_count: usize,
    /// Number of cache hits (successful lookups).
    pub hits: u64,
    /// Number of cache misses (record not found).
    pub misses: u64,
    /// Number of entries received from sync.
    pub sync_entries: u64,
    /// Last sync timestamp (milliseconds since Unix epoch).
    pub last_sync_ms: u64,
}

/// DNS client for read-only synchronization and local queries.
///
/// Maintains a local cache of DNS records synced from an Aspen cluster.
/// Provides instant local lookups without network round-trips.
///
/// Tiger Style: Bounded cache, explicit status tracking, fail-fast on errors.
pub struct AspenDnsClient {
    /// Local cache of DNS records.
    /// Key: "{domain}:{record_type}" (normalized lowercase)
    cache: RwLock<HashMap<String, DnsRecord>>,
    /// Event broadcaster for record changes.
    event_tx: broadcast::Sender<DnsEvent>,
    /// Current sync status.
    status: RwLock<SyncStatus>,
    /// Connection ticket (for reconnection).
    ticket: Option<DnsClientTicket>,
    /// Cache hit counter (atomic for lock-free reads).
    hits: AtomicU64,
    /// Cache miss counter (atomic for lock-free reads).
    misses: AtomicU64,
    /// Sync entries counter.
    sync_entries: AtomicU64,
    /// Last sync time.
    last_sync: RwLock<Option<Instant>>,
}

impl AspenDnsClient {
    /// Create a new DNS client builder.
    pub fn builder() -> DnsClientBuilder {
        DnsClientBuilder::new()
    }

    /// Create a new DNS client with default settings.
    ///
    /// For more control, use `AspenDnsClient::builder()`.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(256);

        Self {
            cache: RwLock::new(HashMap::new()),
            event_tx,
            status: RwLock::new(SyncStatus::Disconnected),
            ticket: None,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            sync_entries: AtomicU64::new(0),
            last_sync: RwLock::new(None),
        }
    }

    /// Get the current sync status.
    pub async fn status(&self) -> SyncStatus {
        *self.status.read().await
    }

    /// Check if the client is synced and ready for queries.
    pub async fn is_synced(&self) -> bool {
        matches!(*self.status.read().await, SyncStatus::Synced)
    }

    /// Get cache statistics.
    pub async fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let last_sync = self.last_sync.read().await;

        CacheStats {
            record_count: cache.len(),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            sync_entries: self.sync_entries.load(Ordering::Relaxed),
            last_sync_ms: last_sync.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
        }
    }

    /// Subscribe to DNS record change events.
    ///
    /// Returns a receiver that will receive events for all record changes.
    ///
    /// # Errors
    /// Returns error if maximum subscribers exceeded.
    pub fn subscribe(&self) -> DnsClientResult<broadcast::Receiver<DnsEvent>> {
        if self.event_tx.receiver_count() >= MAX_SUBSCRIBERS {
            return Err(DnsClientError::MaxSubscribers { max: MAX_SUBSCRIBERS });
        }
        Ok(self.event_tx.subscribe())
    }

    // ========================================================================
    // Record Resolution
    // ========================================================================

    /// Resolve a DNS record from the local cache.
    ///
    /// This is an instant local lookup - no network I/O.
    /// Supports wildcard matching (tries exact match first, then `*.parent`).
    ///
    /// # Arguments
    /// * `domain` - The domain to resolve (case-insensitive)
    /// * `record_type` - The record type to look up
    ///
    /// # Returns
    /// * `Some(DnsRecord)` - If a matching record exists
    /// * `None` - If no matching record found
    pub async fn resolve(&self, domain: &str, record_type: RecordType) -> Option<DnsRecord> {
        let cache = self.cache.read().await;
        let cache_key = Self::cache_key(domain, record_type);

        // Try exact match first
        if let Some(record) = cache.get(&cache_key) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Some(record.clone());
        }

        // Try wildcard match
        // wildcard_parent("api.example.com") returns "*.example.com"
        if let Some(wildcard_domain) = wildcard_parent(domain) {
            let wildcard_key = Self::cache_key(&wildcard_domain, record_type);
            if let Some(record) = cache.get(&wildcard_key) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(record.clone());
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Look up A records (IPv4 addresses) for a domain.
    ///
    /// Convenience method that returns just the addresses.
    pub async fn lookup_a(&self, domain: &str) -> Option<Vec<Ipv4Addr>> {
        self.resolve(domain, RecordType::A).await.and_then(|r| {
            if let DnsRecordData::A { addresses } = r.data {
                Some(addresses)
            } else {
                None
            }
        })
    }

    /// Look up AAAA records (IPv6 addresses) for a domain.
    ///
    /// Convenience method that returns just the addresses.
    pub async fn lookup_aaaa(&self, domain: &str) -> Option<Vec<Ipv6Addr>> {
        self.resolve(domain, RecordType::AAAA).await.and_then(|r| {
            if let DnsRecordData::AAAA { addresses } = r.data {
                Some(addresses)
            } else {
                None
            }
        })
    }

    /// Look up CNAME record for a domain.
    ///
    /// Convenience method that returns just the target.
    pub async fn lookup_cname(&self, domain: &str) -> Option<String> {
        self.resolve(domain, RecordType::CNAME).await.and_then(|r| {
            if let DnsRecordData::CNAME { target } = r.data {
                Some(target)
            } else {
                None
            }
        })
    }

    /// Look up MX records for a domain.
    ///
    /// Returns records sorted by priority (lowest first).
    pub async fn lookup_mx(&self, domain: &str) -> Vec<MxRecord> {
        self.resolve(domain, RecordType::MX)
            .await
            .map(|r| {
                if let DnsRecordData::MX { mut records } = r.data {
                    records.sort_by_key(|r| r.priority);
                    records
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up TXT records for a domain.
    ///
    /// Returns all TXT strings for the domain.
    pub async fn lookup_txt(&self, domain: &str) -> Vec<String> {
        self.resolve(domain, RecordType::TXT)
            .await
            .map(|r| {
                if let DnsRecordData::TXT { strings } = r.data {
                    strings
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up SRV records for a domain.
    ///
    /// Returns records sorted by priority, then weight (for load balancing).
    pub async fn lookup_srv(&self, domain: &str) -> Vec<SrvRecord> {
        self.resolve(domain, RecordType::SRV)
            .await
            .map(|r| {
                if let DnsRecordData::SRV { mut records } = r.data {
                    records.sort_by_key(|r| (r.priority, std::cmp::Reverse(r.weight)));
                    records
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up NS records for a domain.
    ///
    /// Returns the list of nameservers.
    pub async fn lookup_ns(&self, domain: &str) -> Vec<String> {
        self.resolve(domain, RecordType::NS)
            .await
            .map(|r| {
                if let DnsRecordData::NS { nameservers } = r.data {
                    nameservers
                } else {
                    vec![]
                }
            })
            .unwrap_or_default()
    }

    /// Look up PTR record for a domain.
    ///
    /// Used for reverse DNS lookups.
    pub async fn lookup_ptr(&self, domain: &str) -> Option<String> {
        self.resolve(domain, RecordType::PTR).await.and_then(|r| {
            if let DnsRecordData::PTR { target } = r.data {
                Some(target)
            } else {
                None
            }
        })
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

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

    // ========================================================================
    // Sync Integration
    // ========================================================================

    /// Process a synced entry from iroh-docs.
    ///
    /// Called by the sync layer when new entries are received.
    /// Parses DNS records from the KV key/value format.
    ///
    /// # Arguments
    /// * `key` - The KV key (must start with "dns:")
    /// * `value` - The JSON-serialized DnsRecord
    ///
    /// # Returns
    /// Ok if the entry was processed (even if ignored), Err on parse failure.
    pub async fn process_sync_entry(&self, key: &[u8], value: &[u8]) -> DnsClientResult<()> {
        let key_str = String::from_utf8_lossy(key);

        // Only process dns: keys
        if !key_str.starts_with(DNS_KEY_PREFIX) {
            return Ok(());
        }

        // Check zone filter if configured
        if let Some(ticket) = &self.ticket {
            // Extract domain from key (dns:{domain}:{type})
            let parts: Vec<&str> = key_str.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let domain = parts[1];
                if !ticket.should_sync_domain(domain) {
                    debug!(domain, "skipping domain due to zone filter");
                    return Ok(());
                }
            }
        }

        // Parse the record
        let record: DnsRecord = serde_json::from_slice(value).map_err(|e| DnsClientError::Internal {
            reason: format!("failed to parse DNS record: {}", e),
        })?;

        // Check cache capacity
        let cache_size = self.cache.read().await.len();
        if cache_size >= MAX_CACHED_RECORDS {
            return CacheCapacitySnafu {
                max: MAX_CACHED_RECORDS,
            }
            .fail();
        }

        // Update cache
        let cache_key = Self::cache_key(&record.domain, record.record_type());
        let is_new = {
            let mut cache = self.cache.write().await;
            let is_new = !cache.contains_key(&cache_key);
            cache.insert(cache_key, record.clone());
            is_new
        };

        // Update stats
        self.sync_entries.fetch_add(1, Ordering::Relaxed);
        *self.last_sync.write().await = Some(Instant::now());

        // Emit event
        let event = if is_new {
            DnsEvent::RecordAdded {
                domain: record.domain.clone(),
                record_type: record.record_type(),
                record,
            }
        } else {
            // For now, we don't track old values - emit as a simple update
            DnsEvent::RecordAdded {
                domain: record.domain.clone(),
                record_type: record.record_type(),
                record,
            }
        };
        let _ = self.event_tx.send(event);

        Ok(())
    }

    /// Process a deleted entry from iroh-docs.
    ///
    /// Called by the sync layer when entries are deleted.
    pub async fn process_sync_delete(&self, key: &[u8]) -> DnsClientResult<()> {
        let key_str = String::from_utf8_lossy(key);

        // Only process dns: keys
        if !key_str.starts_with(DNS_KEY_PREFIX) {
            return Ok(());
        }

        // Parse domain and type from key (dns:{domain}:{type})
        let parts: Vec<&str> = key_str.splitn(3, ':').collect();
        if parts.len() < 3 {
            return Ok(());
        }

        let domain = parts[1];
        let record_type_str = parts[2];

        let record_type = record_type_str.parse::<RecordType>().map_err(|_| DnsClientError::Internal {
            reason: format!("invalid record type in key: {}", record_type_str),
        })?;

        // Remove from cache
        let removed = {
            let mut cache = self.cache.write().await;
            let cache_key = Self::cache_key(domain, record_type);
            cache.remove(&cache_key)
        };

        // Emit event if record was removed
        if let Some(record) = removed {
            let _ = self.event_tx.send(DnsEvent::RecordDeleted {
                domain: record.domain,
                record_type,
            });
        }

        Ok(())
    }

    /// Update the sync status.
    ///
    /// Called by the sync layer to update connection status.
    pub async fn set_status(&self, status: SyncStatus) {
        let mut current = self.status.write().await;
        let old_status = *current;
        *current = status;

        // Emit status change event
        if old_status != status {
            let _ = self.event_tx.send(DnsEvent::SyncStatusChanged { status });
        }
    }

    /// Check for staleness and update status if needed.
    ///
    /// Called periodically to detect sync issues.
    pub async fn check_staleness(&self) {
        let last_sync = self.last_sync.read().await;
        if let Some(last) = *last_sync
            && last.elapsed() > STALENESS_THRESHOLD
        {
            let mut status = self.status.write().await;
            if *status == SyncStatus::Synced {
                warn!("DNS cache is stale (no updates for {:?})", STALENESS_THRESHOLD);
                *status = SyncStatus::Stale;
                let _ = self.event_tx.send(DnsEvent::SyncStatusChanged {
                    status: SyncStatus::Stale,
                });
            }
        }
    }

    // ========================================================================
    // Internal Helpers
    // ========================================================================

    /// Generate a cache key for a domain/type pair.
    fn cache_key(domain: &str, record_type: RecordType) -> String {
        format!("{}:{}", domain.to_lowercase(), record_type)
    }
}

impl Default for AspenDnsClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring and creating an AspenDnsClient.
#[derive(Default)]
pub struct DnsClientBuilder {
    ticket: Option<DnsClientTicket>,
}

impl DnsClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the connection ticket.
    pub fn ticket(mut self, ticket: DnsClientTicket) -> Self {
        self.ticket = Some(ticket);
        self
    }

    /// Build the DNS client.
    ///
    /// Note: This creates the client but does not start sync.
    /// Use the returned client with your sync infrastructure.
    pub fn build(self) -> AspenDnsClient {
        let (event_tx, _) = broadcast::channel(256);

        AspenDnsClient {
            cache: RwLock::new(HashMap::new()),
            event_tx,
            status: RwLock::new(SyncStatus::Disconnected),
            ticket: self.ticket,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            sync_entries: AtomicU64::new(0),
            last_sync: RwLock::new(None),
        }
    }
}

// Note: spawn_dns_sync_listener has been moved to the main aspen crate
// as it requires integration with aspen::docs and aspen::blob modules.

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;

    fn create_test_a_record(domain: &str, addrs: Vec<Ipv4Addr>) -> DnsRecord {
        DnsRecord {
            domain: domain.to_string(),
            ttl_seconds: 300,
            data: DnsRecordData::A { addresses: addrs },
            updated_at_ms: 0,
        }
    }

    #[tokio::test]
    async fn test_client_new() {
        let client = AspenDnsClient::new();

        assert_eq!(client.status().await, SyncStatus::Disconnected);
        assert_eq!(client.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_process_sync_entry() {
        let client = AspenDnsClient::new();

        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();

        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        assert_eq!(client.cache_size().await, 1);

        let resolved = client.resolve("example.com", RecordType::A).await;
        assert!(resolved.is_some());
    }

    #[tokio::test]
    async fn test_lookup_a() {
        let client = AspenDnsClient::new();

        let addrs = vec![Ipv4Addr::new(192, 168, 1, 1), Ipv4Addr::new(192, 168, 1, 2)];
        let record = create_test_a_record("example.com", addrs.clone());
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();

        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        let result = client.lookup_a("example.com").await;
        assert_eq!(result, Some(addrs));
    }

    #[tokio::test]
    async fn test_lookup_mx() {
        let client = AspenDnsClient::new();

        let mx_records = vec![
            MxRecord {
                priority: 20,
                exchange: "backup.mail.com".to_string(),
            },
            MxRecord {
                priority: 10,
                exchange: "primary.mail.com".to_string(),
            },
        ];

        let record = DnsRecord {
            domain: "example.com".to_string(),
            ttl_seconds: 300,
            data: DnsRecordData::MX { records: mx_records },
            updated_at_ms: 0,
        };

        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        let result = client.lookup_mx("example.com").await;
        assert_eq!(result.len(), 2);
        // Should be sorted by priority (lowest first)
        assert_eq!(result[0].priority, 10);
        assert_eq!(result[1].priority, 20);
    }

    #[tokio::test]
    async fn test_wildcard_resolution() {
        let client = AspenDnsClient::new();

        // Add wildcard record
        let record = create_test_a_record("*.example.com", vec![Ipv4Addr::new(10, 0, 0, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // Should resolve via wildcard
        let result = client.lookup_a("api.example.com").await;
        assert_eq!(result, Some(vec![Ipv4Addr::new(10, 0, 0, 1)]));
    }

    #[tokio::test]
    async fn test_exact_match_over_wildcard() {
        let client = AspenDnsClient::new();

        // Add wildcard record
        let wildcard = create_test_a_record("*.example.com", vec![Ipv4Addr::new(10, 0, 0, 1)]);
        let key = wildcard.kv_key();
        let value = serde_json::to_vec(&wildcard).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // Add exact record
        let exact = create_test_a_record("api.example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = exact.kv_key();
        let value = serde_json::to_vec(&exact).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // Should resolve to exact match, not wildcard
        let result = client.lookup_a("api.example.com").await;
        assert_eq!(result, Some(vec![Ipv4Addr::new(192, 168, 1, 1)]));
    }

    #[tokio::test]
    async fn test_case_insensitive_lookup() {
        let client = AspenDnsClient::new();

        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // All case variants should resolve
        assert!(client.lookup_a("example.com").await.is_some());
        assert!(client.lookup_a("EXAMPLE.COM").await.is_some());
        assert!(client.lookup_a("Example.Com").await.is_some());
    }

    #[tokio::test]
    async fn test_invalidate_record() {
        let client = AspenDnsClient::new();

        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        assert!(client.lookup_a("example.com").await.is_some());

        client.invalidate("example.com", RecordType::A).await;

        assert!(client.lookup_a("example.com").await.is_none());
    }

    #[tokio::test]
    async fn test_process_sync_delete() {
        let client = AspenDnsClient::new();

        // Add a record
        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        assert!(client.lookup_a("example.com").await.is_some());

        // Delete it
        client.process_sync_delete(key.as_bytes()).await.unwrap();

        assert!(client.lookup_a("example.com").await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let client = AspenDnsClient::new();

        // Add a record
        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // Hit
        client.lookup_a("example.com").await;
        // Miss
        client.lookup_a("nonexistent.com").await;

        let stats = client.cache_stats().await;
        assert_eq!(stats.record_count, 1);
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.sync_entries, 1);
    }

    #[tokio::test]
    async fn test_subscribe_events() {
        let client = AspenDnsClient::new();
        let mut rx = client.subscribe().unwrap();

        // Add a record
        let record = create_test_a_record("example.com", vec![Ipv4Addr::new(192, 168, 1, 1)]);
        let key = record.kv_key();
        let value = serde_json::to_vec(&record).unwrap();
        client.process_sync_entry(key.as_bytes(), &value).await.unwrap();

        // Should receive add event
        let event = rx.try_recv().unwrap();
        assert!(matches!(event, DnsEvent::RecordAdded { .. }));
    }

    #[tokio::test]
    async fn test_status_change_event() {
        let client = AspenDnsClient::new();
        let mut rx = client.subscribe().unwrap();

        client.set_status(SyncStatus::Synced).await;

        let event = rx.try_recv().unwrap();
        assert!(matches!(
            event,
            DnsEvent::SyncStatusChanged {
                status: SyncStatus::Synced
            }
        ));
    }

    #[tokio::test]
    async fn test_builder() {
        let ticket = DnsClientTicket::new("test-cluster", "namespace123", vec![]);
        let client = AspenDnsClient::builder().ticket(ticket).build();

        assert!(client.ticket.is_some());
        assert_eq!(client.status().await, SyncStatus::Disconnected);
    }
}
