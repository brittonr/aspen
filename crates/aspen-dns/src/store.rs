//! DNS Store - Server-side DNS record management.
//!
//! Provides the `DnsStore` trait and `AspenDnsStore` implementation for
//! managing DNS records through the Aspen KV store.
//!
//! # Architecture
//!
//! Records are stored in the KV store with keys in the format:
//! `dns:{domain}:{type}` (e.g., `dns:example.com:A`)
//!
//! Zone metadata is stored as:
//! `dns:_zone:{zone_name}` (e.g., `dns:_zone:example.com`)
//!
//! All writes go through Raft consensus, and the existing DocsExporter
//! automatically exports changes to iroh-docs for P2P sync.

use std::sync::Arc;

use aspen_core::KeyValueStore;
use aspen_core::ReadRequest;
use aspen_core::ScanRequest;
use aspen_core::WriteCommand;
use aspen_core::WriteRequest;
use async_trait::async_trait;
use tracing::debug;
use tracing::instrument;

use super::constants::DNS_KEY_PREFIX;
use super::constants::DNS_ZONE_PREFIX;
use super::constants::MAX_BATCH_SIZE;
use super::constants::MAX_ZONES;
use super::error::DnsError;
use super::error::DnsResult;
use super::types::DnsRecord;
use super::types::DnsRecordData;
use super::types::RecordType;
use super::types::Zone;
use super::validation::is_wildcard_domain;
use super::validation::validate_record;
use super::validation::wildcard_parent;

// ============================================================================
// DnsStore Trait
// ============================================================================

/// Trait for DNS record management.
///
/// Provides CRUD operations for DNS records and zones, with resolution
/// supporting wildcard matching.
#[async_trait]
pub trait DnsStore: Send + Sync {
    // =========================================================================
    // Record Operations
    // =========================================================================

    /// Create or update a DNS record.
    ///
    /// Validates the record before storing. If a record with the same
    /// domain and type already exists, it is replaced.
    ///
    /// # Errors
    ///
    /// - `InvalidDomain`: Domain name is malformed
    /// - `InvalidRecordData`: Record data fails validation
    /// - `InvalidTtl`: TTL is out of allowed range
    /// - `Storage`: KV store operation failed
    async fn set_record(&self, record: DnsRecord) -> DnsResult<()>;

    /// Get a specific record by domain and type.
    ///
    /// Returns `None` if the record doesn't exist.
    async fn get_record(&self, domain: &str, record_type: RecordType) -> DnsResult<Option<DnsRecord>>;

    /// Get all records for a domain.
    ///
    /// Returns an empty vector if no records exist for the domain.
    async fn get_records(&self, domain: &str) -> DnsResult<Vec<DnsRecord>>;

    /// Delete a specific record.
    ///
    /// Returns `true` if the record existed and was deleted,
    /// `false` if it didn't exist.
    async fn delete_record(&self, domain: &str, record_type: RecordType) -> DnsResult<bool>;

    /// Batch set multiple records atomically.
    ///
    /// All records are validated before any are written.
    /// If validation fails for any record, no records are written.
    async fn set_records(&self, records: Vec<DnsRecord>) -> DnsResult<()>;

    // =========================================================================
    // Zone Operations
    // =========================================================================

    /// Create or update a zone.
    async fn set_zone(&self, zone: Zone) -> DnsResult<()>;

    /// Get zone configuration.
    async fn get_zone(&self, zone_name: &str) -> DnsResult<Option<Zone>>;

    /// List all zones.
    async fn list_zones(&self) -> DnsResult<Vec<Zone>>;

    /// Delete a zone and optionally all its records.
    async fn delete_zone(&self, zone_name: &str, delete_records: bool) -> DnsResult<bool>;

    // =========================================================================
    // Query Operations
    // =========================================================================

    /// Resolve a domain with wildcard matching.
    ///
    /// Lookup order:
    /// 1. Exact match for `domain:type`
    /// 2. Wildcard match for `*.parent:type`
    ///
    /// For MX and SRV records, results are sorted by priority.
    async fn resolve(&self, domain: &str, record_type: RecordType) -> DnsResult<Vec<DnsRecord>>;

    /// Scan all records with a domain prefix.
    ///
    /// Useful for zone management and bulk operations.
    async fn scan_records(&self, prefix: &str, limit: u32) -> DnsResult<Vec<DnsRecord>>;
}

// ============================================================================
// AspenDnsStore Implementation
// ============================================================================

/// DNS store implementation using Aspen's KeyValueStore.
///
/// Provides DNS record management on top of the distributed KV store.
/// All writes go through Raft consensus and are automatically exported
/// to iroh-docs by the DocsExporter.
pub struct AspenDnsStore<KV: KeyValueStore + ?Sized> {
    /// The underlying key-value store.
    kv: Arc<KV>,
}

impl<KV: KeyValueStore + ?Sized> AspenDnsStore<KV> {
    /// Create a new DNS store backed by the given KeyValueStore.
    pub fn new(kv: Arc<KV>) -> Self {
        Self { kv }
    }

    /// Build the KV key for a DNS record.
    fn record_key(domain: &str, record_type: RecordType) -> String {
        format!("{}{}:{}", DNS_KEY_PREFIX, domain, record_type)
    }

    /// Build the KV key for a zone.
    fn zone_key(zone_name: &str) -> String {
        format!("{}{}", DNS_ZONE_PREFIX, zone_name)
    }
}

#[async_trait]
impl<KV: KeyValueStore + ?Sized + 'static> DnsStore for AspenDnsStore<KV> {
    #[instrument(skip(self, record), fields(domain = %record.domain, record_type = %record.record_type()))]
    async fn set_record(&self, record: DnsRecord) -> DnsResult<()> {
        // Validate the record
        validate_record(&record)?;

        let key = Self::record_key(&record.domain, record.record_type());
        let value = String::from_utf8_lossy(&record.to_json_bytes()).to_string();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        debug!("stored DNS record");
        Ok(())
    }

    #[instrument(skip(self), fields(domain = %domain, record_type = %record_type))]
    async fn get_record(&self, domain: &str, record_type: RecordType) -> DnsResult<Option<DnsRecord>> {
        let key = Self::record_key(domain, record_type);

        let result = match self.kv.read(ReadRequest::new(key)).await {
            Ok(r) => r,
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        match result.kv {
            Some(kv) => {
                let record = DnsRecord::from_json_bytes(kv.value.as_bytes());
                Ok(record)
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self), fields(domain = %domain))]
    async fn get_records(&self, domain: &str) -> DnsResult<Vec<DnsRecord>> {
        let prefix = format!("{}{}:", DNS_KEY_PREFIX, domain);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix,
                limit: Some(100), // MAX_RECORDS_PER_DOMAIN
                continuation_token: None,
            })
            .await?;

        let records: Vec<DnsRecord> = result
            .entries
            .iter()
            .filter_map(|entry| DnsRecord::from_json_bytes(entry.value.as_bytes()))
            .collect();

        Ok(records)
    }

    #[instrument(skip(self), fields(domain = %domain, record_type = %record_type))]
    async fn delete_record(&self, domain: &str, record_type: RecordType) -> DnsResult<bool> {
        let key = Self::record_key(domain, record_type);

        // Check if record exists first
        let exists = match self.kv.read(ReadRequest::new(key.clone())).await {
            Ok(r) => r.kv.is_some(),
            Err(aspen_core::KeyValueStoreError::NotFound { .. }) => false,
            Err(e) => return Err(e.into()),
        };

        if !exists {
            return Ok(false);
        }

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Delete { key },
            })
            .await?;

        debug!("deleted DNS record");
        Ok(true)
    }

    #[instrument(skip(self, records), fields(count = records.len()))]
    async fn set_records(&self, records: Vec<DnsRecord>) -> DnsResult<()> {
        if records.is_empty() {
            return Ok(());
        }

        if records.len() > MAX_BATCH_SIZE as usize {
            return Err(DnsError::BatchSizeExceeded {
                size: records.len() as u32,
                max: MAX_BATCH_SIZE,
            });
        }

        // Validate all records first
        for record in &records {
            validate_record(record)?;
        }

        // Build key-value pairs
        let pairs: Vec<(String, String)> = records
            .iter()
            .map(|r| {
                let key = Self::record_key(&r.domain, r.record_type());
                let value = String::from_utf8_lossy(&r.to_json_bytes()).to_string();
                (key, value)
            })
            .collect();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::SetMulti { pairs },
            })
            .await?;

        debug!("stored {} DNS records in batch", records.len());
        Ok(())
    }

    #[instrument(skip(self, zone), fields(zone = %zone.name))]
    async fn set_zone(&self, zone: Zone) -> DnsResult<()> {
        // Check zone count limit
        let zones = self.list_zones().await?;
        let zone_exists = zones.iter().any(|z| z.name == zone.name);

        if !zone_exists && zones.len() >= MAX_ZONES as usize {
            return Err(DnsError::MaxZonesExceeded { max: MAX_ZONES });
        }

        let key = Self::zone_key(&zone.name);
        let value = String::from_utf8_lossy(&zone.to_json_bytes()).to_string();

        self.kv
            .write(WriteRequest {
                command: WriteCommand::Set { key, value },
            })
            .await?;

        debug!("stored zone");
        Ok(())
    }

    #[instrument(skip(self), fields(zone = %zone_name))]
    async fn get_zone(&self, zone_name: &str) -> DnsResult<Option<Zone>> {
        let key = Self::zone_key(zone_name);

        let result = self.kv.read(ReadRequest::new(key)).await?;

        match result.kv {
            Some(kv) => {
                let zone = Zone::from_json_bytes(kv.value.as_bytes());
                Ok(zone)
            }
            None => Ok(None),
        }
    }

    #[instrument(skip(self))]
    async fn list_zones(&self) -> DnsResult<Vec<Zone>> {
        let result = self
            .kv
            .scan(ScanRequest {
                prefix: DNS_ZONE_PREFIX.to_string(),
                limit: Some(MAX_ZONES),
                continuation_token: None,
            })
            .await?;

        let zones: Vec<Zone> =
            result.entries.iter().filter_map(|entry| Zone::from_json_bytes(entry.value.as_bytes())).collect();

        Ok(zones)
    }

    #[instrument(skip(self), fields(zone = %zone_name, delete_records = %delete_records))]
    async fn delete_zone(&self, zone_name: &str, delete_records: bool) -> DnsResult<bool> {
        let zone_key = Self::zone_key(zone_name);

        // Check if zone exists
        let result = self.kv.read(ReadRequest::new(zone_key.clone())).await?;

        if result.kv.is_none() {
            return Ok(false);
        }

        if delete_records {
            // Delete all records in the zone
            // Scan for all records with domain ending in zone_name
            let prefix = DNS_KEY_PREFIX.to_string();
            let mut continuation_token: Option<String> = None;

            loop {
                let result = self
                    .kv
                    .scan(ScanRequest {
                        prefix: prefix.clone(),
                        limit: Some(MAX_BATCH_SIZE),
                        continuation_token: continuation_token.clone(),
                    })
                    .await?;

                // Filter keys that are in this zone
                let keys_to_delete: Vec<String> = result
                    .entries
                    .iter()
                    .filter(|entry| {
                        // Parse domain from key
                        if let Some((domain, _)) = DnsRecord::parse_kv_key(&entry.key) {
                            domain.ends_with(zone_name) || domain == zone_name
                        } else {
                            false
                        }
                    })
                    .map(|entry| entry.key.clone())
                    .collect();

                if !keys_to_delete.is_empty() {
                    self.kv
                        .write(WriteRequest {
                            command: WriteCommand::DeleteMulti { keys: keys_to_delete },
                        })
                        .await?;
                }

                if !result.is_truncated {
                    break;
                }
                continuation_token = result.continuation_token;
            }
        }

        // Delete the zone itself
        self.kv
            .write(WriteRequest {
                command: WriteCommand::Delete { key: zone_key },
            })
            .await?;

        debug!("deleted zone");
        Ok(true)
    }

    #[instrument(skip(self), fields(domain = %domain, record_type = %record_type))]
    async fn resolve(&self, domain: &str, record_type: RecordType) -> DnsResult<Vec<DnsRecord>> {
        // First, try exact match
        if let Some(record) = self.get_record(domain, record_type).await? {
            let mut records = vec![record];
            sort_records_by_priority(&mut records);
            return Ok(records);
        }

        // If no exact match and not already a wildcard, try wildcard match
        if !is_wildcard_domain(domain)
            && let Some(wildcard_domain) = wildcard_parent(domain)
            && let Some(record) = self.get_record(&wildcard_domain, record_type).await?
        {
            // Create a copy with the original domain for the response
            let resolved = record.clone();
            // Keep the original queried domain, not the wildcard
            // This matches standard DNS resolver behavior
            let mut records = vec![resolved];
            sort_records_by_priority(&mut records);
            return Ok(records);
        }

        Ok(vec![])
    }

    #[instrument(skip(self), fields(prefix = %prefix, limit = %limit))]
    async fn scan_records(&self, prefix: &str, limit: u32) -> DnsResult<Vec<DnsRecord>> {
        let scan_prefix = format!("{}{}", DNS_KEY_PREFIX, prefix);

        let result = self
            .kv
            .scan(ScanRequest {
                prefix: scan_prefix,
                limit: Some(limit),
                continuation_token: None,
            })
            .await?;

        let records: Vec<DnsRecord> = result
            .entries
            .iter()
            .filter_map(|entry| DnsRecord::from_json_bytes(entry.value.as_bytes()))
            .collect();

        Ok(records)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Sort records by priority (for MX and SRV).
fn sort_records_by_priority(records: &mut [DnsRecord]) {
    for record in records.iter_mut() {
        match &mut record.data {
            DnsRecordData::MX { records } => {
                records.sort_by_key(|r| r.priority);
            }
            DnsRecordData::SRV { records } => {
                // SRV: sort by priority, then by weight (descending for load balancing)
                records.sort_by(|a, b| a.priority.cmp(&b.priority).then_with(|| b.weight.cmp(&a.weight)));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use aspen_testing::DeterministicKeyValueStore;

    use super::*;

    fn create_test_store() -> AspenDnsStore<DeterministicKeyValueStore> {
        let kv = DeterministicKeyValueStore::new();
        AspenDnsStore::new(kv)
    }

    #[tokio::test]
    async fn test_set_and_get_record() {
        let store = create_test_store();

        let record = DnsRecord::new("example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap()));

        store.set_record(record.clone()).await.unwrap();

        let retrieved = store.get_record("example.com", RecordType::A).await.unwrap();
        assert!(retrieved.is_some());

        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.domain, "example.com");
        assert_eq!(retrieved.ttl_seconds, 3600);
    }

    #[tokio::test]
    async fn test_get_records_for_domain() {
        let store = create_test_store();

        // Add multiple record types for the same domain
        store
            .set_record(DnsRecord::new("example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())))
            .await
            .unwrap();

        store
            .set_record(DnsRecord::new("example.com", 3600, DnsRecordData::mx(10, "mail.example.com")))
            .await
            .unwrap();

        let records = store.get_records("example.com").await.unwrap();
        assert_eq!(records.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_record() {
        let store = create_test_store();

        let record = DnsRecord::new("example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap()));

        store.set_record(record).await.unwrap();

        let deleted = store.delete_record("example.com", RecordType::A).await.unwrap();
        assert!(deleted);

        let retrieved = store.get_record("example.com", RecordType::A).await.unwrap();
        assert!(retrieved.is_none());

        // Deleting non-existent record returns false
        let deleted = store.delete_record("example.com", RecordType::A).await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_set_records_batch() {
        let store = create_test_store();

        let records = vec![
            DnsRecord::new("a.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())),
            DnsRecord::new("b.example.com", 3600, DnsRecordData::a("192.168.1.2".parse().unwrap())),
            DnsRecord::new("c.example.com", 3600, DnsRecordData::a("192.168.1.3".parse().unwrap())),
        ];

        store.set_records(records).await.unwrap();

        assert!(store.get_record("a.example.com", RecordType::A).await.unwrap().is_some());
        assert!(store.get_record("b.example.com", RecordType::A).await.unwrap().is_some());
        assert!(store.get_record("c.example.com", RecordType::A).await.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_zone_operations() {
        let store = create_test_store();

        let zone = Zone::new("example.com").with_default_ttl_secs(7200).with_description("Test zone");

        store.set_zone(zone.clone()).await.unwrap();

        let retrieved = store.get_zone("example.com").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().default_ttl_secs, 7200);

        let zones = store.list_zones().await.unwrap();
        assert_eq!(zones.len(), 1);

        let deleted = store.delete_zone("example.com", false).await.unwrap();
        assert!(deleted);

        let zones = store.list_zones().await.unwrap();
        assert!(zones.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_exact_match() {
        let store = create_test_store();

        store
            .set_record(DnsRecord::new("www.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())))
            .await
            .unwrap();

        let records = store.resolve("www.example.com", RecordType::A).await.unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].domain, "www.example.com");
    }

    #[tokio::test]
    async fn test_resolve_wildcard_match() {
        let store = create_test_store();

        // Set wildcard record
        store
            .set_record(DnsRecord::new("*.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())))
            .await
            .unwrap();

        // Query for subdomain - should match wildcard
        let records = store.resolve("foo.example.com", RecordType::A).await.unwrap();
        assert_eq!(records.len(), 1);

        // Query for exact wildcard domain
        let records = store.resolve("*.example.com", RecordType::A).await.unwrap();
        assert_eq!(records.len(), 1);

        // Query for non-matching domain
        let records = store.resolve("foo.other.com", RecordType::A).await.unwrap();
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn test_resolve_exact_before_wildcard() {
        let store = create_test_store();

        // Set both exact and wildcard records
        store
            .set_record(DnsRecord::new("www.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())))
            .await
            .unwrap();

        store
            .set_record(DnsRecord::new("*.example.com", 3600, DnsRecordData::a("192.168.1.2".parse().unwrap())))
            .await
            .unwrap();

        // Exact match should take precedence
        let records = store.resolve("www.example.com", RecordType::A).await.unwrap();
        assert_eq!(records.len(), 1);
        if let DnsRecordData::A { addresses } = &records[0].data {
            assert_eq!(addresses[0], "192.168.1.1".parse::<std::net::Ipv4Addr>().unwrap());
        } else {
            panic!("expected A record");
        }
    }

    #[tokio::test]
    async fn test_scan_records() {
        let store = create_test_store();

        store
            .set_record(DnsRecord::new("a.example.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap())))
            .await
            .unwrap();

        store
            .set_record(DnsRecord::new("b.example.com", 3600, DnsRecordData::a("192.168.1.2".parse().unwrap())))
            .await
            .unwrap();

        store
            .set_record(DnsRecord::new("a.other.com", 3600, DnsRecordData::a("192.168.2.1".parse().unwrap())))
            .await
            .unwrap();

        // Scan for example.com records
        let records = store.scan_records("", 100).await.unwrap();
        assert_eq!(records.len(), 3);
    }

    #[tokio::test]
    async fn test_mx_priority_sorting() {
        let store = create_test_store();

        use crate::types::MxRecord;

        store
            .set_record(DnsRecord::new("example.com", 3600, DnsRecordData::MX {
                records: vec![
                    MxRecord::new(30, "mail3.example.com"),
                    MxRecord::new(10, "mail1.example.com"),
                    MxRecord::new(20, "mail2.example.com"),
                ],
            }))
            .await
            .unwrap();

        let records = store.resolve("example.com", RecordType::MX).await.unwrap();
        assert_eq!(records.len(), 1);

        if let DnsRecordData::MX { records: mx_records } = &records[0].data {
            assert_eq!(mx_records[0].priority, 10);
            assert_eq!(mx_records[1].priority, 20);
            assert_eq!(mx_records[2].priority, 30);
        } else {
            panic!("expected MX record");
        }
    }

    #[tokio::test]
    async fn test_invalid_record_rejected() {
        let store = create_test_store();

        // Invalid domain
        let record = DnsRecord::new("-invalid.com", 3600, DnsRecordData::a("192.168.1.1".parse().unwrap()));

        let result = store.set_record(record).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_size_limit() {
        let store = create_test_store();

        // Create more records than MAX_BATCH_SIZE
        let records: Vec<DnsRecord> = (0..MAX_BATCH_SIZE + 10)
            .map(|i| {
                DnsRecord::new(
                    format!("host{}.example.com", i),
                    3600,
                    DnsRecordData::a(format!("192.168.1.{}", i % 256).parse().unwrap()),
                )
            })
            .collect();

        let result = store.set_records(records).await;
        assert!(result.is_err());
    }
}
