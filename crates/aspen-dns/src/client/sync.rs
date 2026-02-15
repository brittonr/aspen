//! Sync integration for processing iroh-docs entries and status management.

use std::sync::atomic::Ordering;
use std::time::Instant;

use tracing::debug;
use tracing::warn;

use super::super::constants::DNS_KEY_PREFIX;
use super::super::constants::MAX_CACHED_RECORDS;
use super::super::constants::STALENESS_THRESHOLD;
use super::super::error::CacheCapacitySnafu;
use super::super::error::DnsClientError;
use super::super::error::DnsClientResult;
use super::super::types::DnsEvent;
use super::super::types::DnsRecord;
use super::super::types::RecordType;
use super::super::types::SyncStatus;
use super::AspenDnsClient;

impl AspenDnsClient {
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
}
