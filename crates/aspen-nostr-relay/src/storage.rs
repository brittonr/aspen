//! Event storage backed by Aspen's Raft-replicated KV store.
//!
//! Events are stored as JSON at `nostr:ev:{event_id}`. Secondary indexes
//! enable filter queries by kind, author, and tag.

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

use aspen_kv_types::DeleteRequest;
use aspen_kv_types::KeyValueStoreError;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use nostr::prelude::*;
use tracing::debug;

use crate::constants::*;

/// Errors from event storage operations.
#[derive(Debug)]
pub enum StorageError {
    /// KV store returned an error.
    Kv(String),
    /// Event JSON serialization/deserialization failed.
    Serde(String),
    /// Internal logic error.
    Internal(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kv(msg) => write!(f, "kv error: {msg}"),
            Self::Serde(msg) => write!(f, "serde error: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

/// Trait for Nostr event persistence.
#[async_trait]
pub trait NostrEventStore: Send + Sync {
    /// Store an event. Returns `true` if the event was new, `false` if duplicate.
    async fn store_event(&self, event: &Event) -> Result<bool, StorageError>;

    /// Retrieve an event by its ID.
    async fn get_event(&self, id: &EventId) -> Result<Option<Event>, StorageError>;

    /// Query events matching any of the given filters (OR across filters).
    async fn query_events(&self, filters: &[Filter]) -> Result<Vec<Event>, StorageError>;

    /// Delete an event and its index entries.
    async fn delete_event(&self, id: &EventId) -> Result<(), StorageError>;

    /// Count of stored events.
    async fn event_count(&self) -> Result<u32, StorageError>;
}

/// KV-backed event store implementation.
pub struct KvEventStore<S: ?Sized> {
    kv: Arc<S>,
}

impl<S: ?Sized> KvEventStore<S> {
    /// Create from an existing Arc (works with `Arc<dyn KeyValueStore>`).
    pub fn from_arc(kv: Arc<S>) -> Self {
        Self { kv }
    }
}

impl<S> KvEventStore<S> {
    /// Create from an owned value.
    pub fn new(kv: Arc<S>) -> Self {
        Self { kv }
    }
}

/// Format `created_at` as a 16-digit zero-padded decimal for lexicographic ordering.
fn timestamp_key(ts: Timestamp) -> String {
    format!("{:016}", ts.as_secs())
}

/// Build the event data key.
fn event_key(id: &EventId) -> String {
    format!("{KV_PREFIX_EVENT}{}", id.to_hex())
}

/// Build kind index key.
fn kind_index_key(kind: Kind, created_at: Timestamp, id: &EventId) -> String {
    format!("{}{:06}:{}:{}", KV_PREFIX_KIND, kind.as_u16(), timestamp_key(created_at), id.to_hex())
}

/// Build author index key.
fn author_index_key(author: &PublicKey, created_at: Timestamp, id: &EventId) -> String {
    format!("{}{}:{}:{}", KV_PREFIX_AUTHOR, author.to_hex(), timestamp_key(created_at), id.to_hex())
}

/// Build tag index keys for all indexable tags on an event.
fn tag_index_keys(event: &Event) -> Vec<String> {
    let tag_count = event.tags.len();
    let mut keys = Vec::with_capacity(tag_count);
    for tag in event.tags.iter() {
        let slice = tag.as_slice();
        // Only index single-letter tags with at least one value (NIP-01 generic tag queries)
        if slice.len() >= 2 && slice[0].len() == 1 {
            keys.push(format!("{}{}:{}:{}", KV_PREFIX_TAG, &slice[0], &slice[1], event.id.to_hex()));
        }
    }
    keys
}

/// Check if a kind is replaceable (NIP-01).
fn is_replaceable(kind: Kind) -> bool {
    let k = kind.as_u16();
    k == 0 || k == 3 || (10_000..20_000).contains(&k)
}

/// Check if a kind is parameterized replaceable (NIP-01).
fn is_parameterized_replaceable(kind: Kind) -> bool {
    let k = kind.as_u16();
    (30_000..40_000).contains(&k)
}

/// Extract the `d` tag value from an event (for parameterized replaceable events).
fn d_tag_value(event: &Event) -> Option<String> {
    for tag in event.tags.iter() {
        let s = tag.as_slice();
        if s.len() >= 2 && s[0] == "d" {
            return Some(s[1].clone());
        }
    }
    None
}

#[async_trait]
impl<S: KeyValueStore + ?Sized> NostrEventStore for KvEventStore<S> {
    async fn store_event(&self, event: &Event) -> Result<bool, StorageError> {
        let ek = event_key(&event.id);

        // Check for duplicate
        if let Ok(result) = self.kv.read(ReadRequest::new(ek.clone())).await
            && result.kv.is_some()
        {
            return Ok(false);
        }

        // Handle replaceable events — delete the old one first
        if is_replaceable(event.kind) {
            self.delete_replaceable(event.kind, &event.pubkey, None).await?;
        } else if is_parameterized_replaceable(event.kind) {
            let d = d_tag_value(event).unwrap_or_default();
            self.delete_replaceable(event.kind, &event.pubkey, Some(&d)).await?;
        }

        // Enforce MAX_STORED_EVENTS via eviction
        let count = self.event_count().await?;
        if count >= MAX_STORED_EVENTS {
            self.evict_oldest().await?;
        }

        // Serialize event
        let json = serde_json::to_string(event).map_err(|e| StorageError::Serde(e.to_string()))?;

        // Write event data
        self.kv_set(&ek, &json).await?;

        // Write kind index
        let ki = kind_index_key(event.kind, event.created_at, &event.id);
        self.kv_set(&ki, "").await?;

        // Write author index
        let ai = author_index_key(&event.pubkey, event.created_at, &event.id);
        self.kv_set(&ai, "").await?;

        // Write tag indexes
        for tk in tag_index_keys(event) {
            self.kv_set(&tk, "").await?;
        }

        // Increment counter
        self.increment_count().await?;

        debug!(event_id = %event.id, kind = %event.kind, "stored nostr event");
        Ok(true)
    }

    async fn get_event(&self, id: &EventId) -> Result<Option<Event>, StorageError> {
        let ek = event_key(id);
        match self.kv.read(ReadRequest::new(ek)).await {
            Ok(result) => match result.kv {
                Some(entry) => {
                    let event: Event =
                        serde_json::from_str(&entry.value).map_err(|e| StorageError::Serde(e.to_string()))?;
                    Ok(Some(event))
                }
                None => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }

    async fn query_events(&self, filters: &[Filter]) -> Result<Vec<Event>, StorageError> {
        let mut all_events: Vec<Event> = Vec::with_capacity(filters.len().saturating_mul(16));
        let mut seen_ids: HashSet<String> = HashSet::with_capacity(filters.len().saturating_mul(16));

        for filter in filters {
            let candidates = self.query_single_filter(filter).await?;
            for event in candidates {
                let id_hex = event.id.to_hex();
                if seen_ids.insert(id_hex) {
                    all_events.push(event);
                }
            }
        }

        // Sort by created_at descending
        all_events.sort_by_key(|e| std::cmp::Reverse(e.created_at));

        Ok(all_events)
    }

    async fn delete_event(&self, id: &EventId) -> Result<(), StorageError> {
        // Load the event first to know which index keys to remove
        let event = match self.get_event(id).await? {
            Some(e) => e,
            None => return Ok(()),
        };

        // Delete event data
        self.kv_delete(&event_key(id)).await?;

        // Delete kind index
        self.kv_delete(&kind_index_key(event.kind, event.created_at, id)).await?;

        // Delete author index
        self.kv_delete(&author_index_key(&event.pubkey, event.created_at, id)).await?;

        // Delete tag indexes
        for tk in tag_index_keys(&event) {
            self.kv_delete(&tk).await?;
        }

        // Decrement counter
        self.decrement_count().await?;

        Ok(())
    }

    async fn event_count(&self) -> Result<u32, StorageError> {
        match self.kv.read(ReadRequest::new(KV_EVENT_COUNT.to_string())).await {
            Ok(result) => match result.kv {
                Some(entry) => entry.value.parse::<u32>().map_err(|e| StorageError::Internal(e.to_string())),
                None => Ok(0),
            },
            Err(_) => Ok(0),
        }
    }
}

// Private helpers
impl<S: KeyValueStore + ?Sized> KvEventStore<S> {
    async fn kv_set(&self, key: &str, value: &str) -> Result<(), StorageError> {
        let req = WriteRequest {
            command: WriteCommand::Set {
                key: key.to_string(),
                value: value.to_string(),
            },
        };
        self.kv.write(req).await.map_err(|e| StorageError::Kv(e.to_string()))?;
        Ok(())
    }

    async fn kv_delete(&self, key: &str) -> Result<(), StorageError> {
        let req = DeleteRequest { key: key.to_string() };
        self.kv.delete(req).await.map_err(|e| StorageError::Kv(e.to_string()))?;
        Ok(())
    }

    async fn increment_count(&self) -> Result<(), StorageError> {
        for _ in 0..MAX_CAS_RETRIES {
            let (current_str, expected) = self.read_count_for_cas().await?;
            let count: u32 = current_str
                .as_deref()
                .unwrap_or("0")
                .parse()
                .map_err(|e| StorageError::Internal(format!("bad event count: {e}")))?;
            let new_value = count.saturating_add(1).to_string();
            let req = WriteRequest::compare_and_swap(KV_EVENT_COUNT, expected, &new_value);
            match self.kv.write(req).await {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                Err(e) => return Err(StorageError::Kv(e.to_string())),
            }
        }
        Err(StorageError::Internal("event count CAS retries exhausted".to_string()))
    }

    async fn decrement_count(&self) -> Result<(), StorageError> {
        for _ in 0..MAX_CAS_RETRIES {
            let (current_str, expected) = self.read_count_for_cas().await?;
            let count: u32 = current_str
                .as_deref()
                .unwrap_or("0")
                .parse()
                .map_err(|e| StorageError::Internal(format!("bad event count: {e}")))?;
            let new_value = count.saturating_sub(1).to_string();
            let req = WriteRequest::compare_and_swap(KV_EVENT_COUNT, expected, &new_value);
            match self.kv.write(req).await {
                Ok(_) => return Ok(()),
                Err(KeyValueStoreError::CompareAndSwapFailed { .. }) => continue,
                Err(e) => return Err(StorageError::Kv(e.to_string())),
            }
        }
        Err(StorageError::Internal("event count CAS retries exhausted".to_string()))
    }

    /// Read the current count string and produce the CAS expected value.
    ///
    /// Returns `(Some(value_string), Some(value_string))` if the key exists,
    /// or `(None, None)` if it doesn't. The second element is the `expected`
    /// argument for `WriteRequest::compare_and_swap`.
    async fn read_count_for_cas(&self) -> Result<(Option<String>, Option<String>), StorageError> {
        match self.kv.read(ReadRequest::new(KV_EVENT_COUNT.to_string())).await {
            Ok(result) => match result.kv {
                Some(entry) => Ok((Some(entry.value.clone()), Some(entry.value))),
                None => Ok((None, None)),
            },
            Err(_) => Ok((None, None)),
        }
    }

    /// Delete the existing replaceable event matching (kind, pubkey, d_tag).
    async fn delete_replaceable(
        &self,
        kind: Kind,
        pubkey: &PublicKey,
        d_tag: Option<&str>,
    ) -> Result<(), StorageError> {
        // Scan by author+kind to find candidates
        let prefix = format!("{}{}:", KV_PREFIX_AUTHOR, pubkey.to_hex());
        let scan = ScanRequest {
            prefix,
            limit_results: Some(MAX_STORED_EVENTS),
            continuation_token: None,
        };
        let result = self.kv.scan(scan).await.map_err(|e| StorageError::Kv(e.to_string()))?;

        for entry in &result.entries {
            // Extract event_id from the index key (last segment after final ':')
            let id_hex = match entry.key.rsplit(':').next() {
                Some(h) => h,
                None => continue,
            };
            let event_id = match EventId::from_hex(id_hex) {
                Ok(id) => id,
                Err(_) => continue,
            };
            if let Some(event) = self.get_event(&event_id).await? {
                if event.kind != kind {
                    continue;
                }
                // For parameterized replaceable, check d-tag
                if let Some(expected_d) = d_tag {
                    let actual_d = d_tag_value(&event).unwrap_or_default();
                    if actual_d != expected_d {
                        continue;
                    }
                }
                // Delete the old event
                self.delete_event(&event_id).await?;
            }
        }
        Ok(())
    }

    /// Evict the oldest event by created_at.
    async fn evict_oldest(&self) -> Result<(), StorageError> {
        // Scan kind index (lexicographically ordered by timestamp) to find oldest
        let scan = ScanRequest {
            prefix: KV_PREFIX_KIND.to_string(),
            limit_results: Some(1),
            continuation_token: None,
        };
        let result = self.kv.scan(scan).await.map_err(|e| StorageError::Kv(e.to_string()))?;

        if let Some(entry) = result.entries.first() {
            let id_hex = match entry.key.rsplit(':').next() {
                Some(h) => h,
                None => return Ok(()),
            };
            if let Ok(event_id) = EventId::from_hex(id_hex) {
                self.delete_event(&event_id).await?;
            }
        }
        Ok(())
    }

    /// Query events matching a single filter.
    async fn query_single_filter(&self, filter: &Filter) -> Result<Vec<Event>, StorageError> {
        // Determine scan strategy — pick the most selective index
        let candidate_ids = self.scan_candidates(filter).await?;

        // Load events and apply full filter matching
        let mut matched = Vec::with_capacity(candidate_ids.len());
        for id in &candidate_ids {
            if let Some(event) = self.get_event(id).await?
                && filter.match_event(&event, nostr::filter::MatchEventOptions::new())
            {
                matched.push(event);
            }
        }

        // Sort newest first
        matched.sort_by_key(|e| std::cmp::Reverse(e.created_at));

        // Apply limit
        if let Some(limit) = filter.limit {
            matched.truncate(limit);
        }

        Ok(matched)
    }

    /// Scan KV indexes to find candidate event IDs for a filter.
    ///
    /// Uses timestamp-bounded scan ranges when `since`/`until` are present
    /// on kind and author index scans.
    async fn scan_candidates(&self, filter: &Filter) -> Result<Vec<EventId>, StorageError> {
        use crate::filters::ScanRange;
        use crate::filters::filter_scan_ranges;

        // Direct lookup by IDs
        if let Some(ref ids) = filter.ids {
            return Ok(ids.iter().cloned().collect());
        }

        let ranges = filter_scan_ranges(filter);

        // Kind-scoped scan (possibly time-bounded)
        if filter.kinds.is_some() {
            let mut ids = Vec::with_capacity(ranges.len().saturating_mul(64));
            for range in &ranges {
                ids.extend(self.scan_index_range(range).await?);
            }
            // If we also have authors, intersect
            if let Some(ref authors) = filter.authors {
                let author_ids = self.scan_authors_bounded(authors, filter).await?;
                let author_set: HashSet<_> = author_ids.into_iter().map(|id| id.to_hex()).collect();
                ids.retain(|id| author_set.contains(&id.to_hex()));
            }
            return Ok(ids);
        }

        // Author-scoped scan (possibly time-bounded)
        if filter.authors.is_some() {
            let mut ids = Vec::with_capacity(ranges.len().saturating_mul(64));
            for range in &ranges {
                ids.extend(self.scan_index_range(range).await?);
            }
            return Ok(ids);
        }

        // Tag-scoped scan — no timestamp bounding
        if !filter.generic_tags.is_empty() {
            let mut result_ids: Option<HashSet<String>> = None;
            for range in &ranges {
                let mut tag_ids = HashSet::new();
                for id in self.scan_index_range(range).await? {
                    tag_ids.insert(id.to_hex());
                }
                result_ids = Some(match result_ids {
                    Some(existing) => existing.intersection(&tag_ids).cloned().collect(),
                    None => tag_ids,
                });
            }
            if let Some(ids) = result_ids {
                return ids.iter().filter_map(|h| EventId::from_hex(h).ok()).collect::<Vec<_>>().pipe(Ok);
            }
        }

        // Full scan fallback
        let ids = self
            .scan_index_range(&ScanRange {
                start: KV_PREFIX_EVENT.to_string(),
                end: None,
            })
            .await?;
        Ok(ids)
    }

    async fn scan_authors_bounded(
        &self,
        authors: &std::collections::BTreeSet<PublicKey>,
        filter: &Filter,
    ) -> Result<Vec<EventId>, StorageError> {
        let since_pad = filter.since.map(|t| format!("{:016}", t.as_secs()));
        let until_pad = filter.until.map(|t| format!("{:016}", t.as_secs()));

        let mut ids = Vec::with_capacity(authors.len().saturating_mul(64));
        for author in authors {
            let author_prefix = format!("{}{}:", KV_PREFIX_AUTHOR, author.to_hex());
            let range = crate::filters::bounded_range(&author_prefix, since_pad.as_deref(), until_pad.as_deref());
            ids.extend(self.scan_index_range(&range).await?);
        }
        Ok(ids)
    }

    /// Scan a KV range and extract event IDs from the key suffix.
    ///
    /// Uses `range.start` as the scan prefix. If `range.end` is set,
    /// entries with keys >= end are filtered out during iteration.
    async fn scan_index_range(&self, range: &crate::filters::ScanRange) -> Result<Vec<EventId>, StorageError> {
        let scan = ScanRequest {
            prefix: range.start.clone(),
            limit_results: Some(MAX_STORED_EVENTS),
            continuation_token: None,
        };
        let result = self.kv.scan(scan).await.map_err(|e| StorageError::Kv(e.to_string()))?;

        let mut ids = Vec::with_capacity(result.entries.len());
        for entry in &result.entries {
            // Apply end bound if present
            if let Some(ref end) = range.end
                && entry.key.as_str() >= end.as_str()
            {
                break; // Keys are sorted; everything after is out of range
            }
            // For event keys: `nostr:ev:{event_id}` → event_id is after prefix
            // For index keys: `nostr:ki:...:{event_id}` → event_id is last segment
            let id_hex = if range.start == KV_PREFIX_EVENT {
                entry.key.strip_prefix(KV_PREFIX_EVENT).unwrap_or_default()
            } else {
                entry.key.rsplit(':').next().unwrap_or_default()
            };
            if let Ok(event_id) = EventId::from_hex(id_hex) {
                ids.push(event_id);
            }
        }
        Ok(ids)
    }
}

/// Pipe trait for inline transformation.
trait Pipe: Sized {
    fn pipe<F, R>(self, f: F) -> R
    where F: FnOnce(Self) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}
