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

mod builder;
mod cache;
mod lookup;
mod sync;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::time::Instant;

pub use builder::DnsClientBuilder;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

use super::constants::MAX_SUBSCRIBERS;
use super::error::DnsClientError;
use super::error::DnsClientResult;
use super::ticket::DnsClientTicket;
use super::types::DnsEvent;
use super::types::DnsRecord;
use super::types::RecordType;
use super::types::SyncStatus;

/// Statistics about the DNS client cache.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cached records.
    pub record_count: u32,
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
    pub(crate) cache: RwLock<HashMap<String, DnsRecord>>,
    /// Event broadcaster for record changes.
    pub(crate) event_tx: broadcast::Sender<DnsEvent>,
    /// Current sync status.
    pub(crate) status: RwLock<SyncStatus>,
    /// Connection ticket (for reconnection).
    pub(crate) ticket: Option<DnsClientTicket>,
    /// Cache hit counter (atomic for lock-free reads).
    pub(crate) hits: AtomicU64,
    /// Cache miss counter (atomic for lock-free reads).
    pub(crate) misses: AtomicU64,
    /// Sync entries counter.
    pub(crate) sync_entries: AtomicU64,
    /// Last sync time.
    pub(crate) last_sync: RwLock<Option<Instant>>,
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
            record_count: cache.len() as u32,
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

    /// Generate a cache key for a domain/type pair.
    pub(crate) fn cache_key(domain: &str, record_type: RecordType) -> String {
        format!("{}:{}", domain.to_lowercase(), record_type)
    }
}

impl Default for AspenDnsClient {
    fn default() -> Self {
        Self::new()
    }
}

// Note: spawn_dns_sync_listener has been moved to the main aspen crate
// as it requires integration with aspen::docs and aspen::blob modules.

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::super::types::DnsRecordData;
    use super::super::types::MxRecord;
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
        assert!(matches!(event, DnsEvent::SyncStatusChanged {
            status: SyncStatus::Synced
        }));
    }

    #[tokio::test]
    async fn test_builder() {
        let ticket = DnsClientTicket::new("test-cluster", "namespace123", vec![]);
        let client = AspenDnsClient::builder().ticket(ticket).build();

        assert!(client.ticket.is_some());
        assert_eq!(client.status().await, SyncStatus::Disconnected);
    }
}
