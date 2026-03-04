//! Calendar store — CRUD operations backed by `KeyValueStore`.
//!
//! All calendars and events are stored as JSON in the distributed KV store
//! with namespaced key prefixes:
//! - `calendar:cal:{calendar_id}` — calendar metadata
//! - `calendar:event:{calendar_id}:{event_id}` — events

use std::sync::Arc;

use aspen_kv_types::DeleteRequest;
use aspen_kv_types::ReadRequest;
use aspen_kv_types::ScanRequest;
use aspen_kv_types::WriteCommand;
use aspen_kv_types::WriteRequest;
use aspen_traits::KeyValueStore;
use tracing::debug;

use crate::CalendarError;
use crate::CalendarEvent;
use crate::CalendarMeta;
use crate::ical::parse_vcalendar;
use crate::ical::parse_vevent;
use crate::ical::rrule::RecurrenceInstance;
use crate::ical::rrule::expand_rrule;
use crate::ical::serialize_vcalendar;

/// Key prefix for calendars.
const CAL_PREFIX: &str = "calendar:cal:";
/// Key prefix for events.
const EVENT_PREFIX: &str = "calendar:event:";

/// Maximum number of events to return from a list/search.
const MAX_LIST_LIMIT: u32 = 1_000;
/// Default list limit.
const DEFAULT_LIST_LIMIT: u32 = 100;
/// Maximum recurrence instances to expand.
const MAX_RECURRENCE_INSTANCES: u32 = 1_000;

/// Calendar store backed by a distributed `KeyValueStore`.
pub struct CalendarStore<S: KeyValueStore + ?Sized> {
    store: Arc<S>,
}

impl<S: KeyValueStore + ?Sized + 'static> CalendarStore<S> {
    /// Create a new calendar store.
    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    // ========================================================================
    // Calendar CRUD
    // ========================================================================

    /// Create a calendar.
    pub async fn create_calendar(
        &self,
        name: &str,
        color: Option<&str>,
        timezone: Option<&str>,
        description: Option<&str>,
        now_ms: u64,
    ) -> Result<CalendarMeta, CalendarError> {
        if name.is_empty() {
            return Err(CalendarError::InvalidInput {
                reason: "calendar name cannot be empty".into(),
            });
        }

        let id = generate_id("cal", name);
        let calendar = CalendarMeta {
            id: id.clone(),
            name: name.to_string(),
            color: color.map(|s| s.to_string()),
            timezone: timezone.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
            owner: None,
            created_at_ms: now_ms,
        };

        let key = format!("{CAL_PREFIX}{id}");
        let value = serde_json::to_string(&calendar).map_err(|e| CalendarError::StorageError {
            reason: format!("serialize calendar: {e}"),
        })?;

        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        debug!(calendar_id = %id, name, "created calendar");
        Ok(calendar)
    }

    /// Delete a calendar and all its events.
    pub async fn delete_calendar(&self, calendar_id: &str) -> Result<(), CalendarError> {
        // Verify the calendar exists.
        self.get_calendar(calendar_id).await?;

        // Delete all events in the calendar.
        let event_prefix = format!("{EVENT_PREFIX}{calendar_id}:");
        let events = self.scan_all(&event_prefix).await?;
        for entry in &events {
            self.store
                .delete(DeleteRequest::new(&entry.key))
                .await
                .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;
        }

        // Delete the calendar itself.
        let key = format!("{CAL_PREFIX}{calendar_id}");
        self.store
            .delete(DeleteRequest::new(&key))
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        debug!(calendar_id, "deleted calendar");
        Ok(())
    }

    /// Get a calendar by ID.
    pub async fn get_calendar(&self, calendar_id: &str) -> Result<CalendarMeta, CalendarError> {
        let key = format!("{CAL_PREFIX}{calendar_id}");
        let result = self.kv_read(&key).await?;
        match result {
            Some(value) => serde_json::from_str(&value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize calendar: {e}"),
            }),
            None => Err(CalendarError::CalendarNotFound {
                id: calendar_id.to_string(),
            }),
        }
    }

    /// List all calendars.
    pub async fn list_calendars(&self, limit: Option<u32>) -> Result<Vec<CalendarMeta>, CalendarError> {
        let limit = clamp_limit(limit);
        let scan = self
            .store
            .scan(ScanRequest {
                prefix: CAL_PREFIX.to_string(),
                limit_results: Some(limit),
                continuation_token: None,
            })
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        let mut calendars = Vec::with_capacity(scan.entries.len());
        for entry in &scan.entries {
            let cal: CalendarMeta = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize calendar: {e}"),
            })?;
            calendars.push(cal);
        }
        Ok(calendars)
    }

    // ========================================================================
    // Event CRUD
    // ========================================================================

    /// Create an event from iCal data.
    pub async fn create_event(
        &self,
        calendar_id: &str,
        ical_data: &str,
        now_ms: u64,
    ) -> Result<CalendarEvent, CalendarError> {
        // Verify calendar exists.
        self.get_calendar(calendar_id).await?;

        let mut event = parse_vevent(ical_data).map_err(|e| CalendarError::ParseIcal { reason: e.to_string() })?;

        // Assign calendar and generate ID.
        event.calendar_id = calendar_id.to_string();
        if event.id.is_empty() {
            let seed = if event.uid.is_empty() {
                format!("{}:{}:{}", calendar_id, event.summary, now_ms)
            } else {
                event.uid.clone()
            };
            event.id = generate_id("event", &seed);
        }
        event.created_at_ms = now_ms;
        event.updated_at_ms = now_ms;

        self.write_event(&event).await?;
        debug!(event_id = %event.id, calendar_id, summary = %event.summary, "created event");
        Ok(event)
    }

    /// Get an event by ID.
    pub async fn get_event(&self, event_id: &str) -> Result<CalendarEvent, CalendarError> {
        // Scan across all calendars since we don't know the calendar_id.
        let scan = self
            .store
            .scan(ScanRequest {
                prefix: EVENT_PREFIX.to_string(),
                limit_results: Some(MAX_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        for entry in &scan.entries {
            let event: CalendarEvent = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize event: {e}"),
            })?;
            if event.id == event_id {
                return Ok(event);
            }
        }

        Err(CalendarError::NotFound {
            id: event_id.to_string(),
        })
    }

    /// Update an event with new iCal data.
    pub async fn update_event(
        &self,
        event_id: &str,
        ical_data: &str,
        now_ms: u64,
    ) -> Result<CalendarEvent, CalendarError> {
        let existing = self.get_event(event_id).await?;

        let mut event = parse_vevent(ical_data).map_err(|e| CalendarError::ParseIcal { reason: e.to_string() })?;

        // Preserve identity fields, increment sequence.
        event.id = existing.id;
        event.calendar_id = existing.calendar_id;
        event.created_at_ms = existing.created_at_ms;
        event.updated_at_ms = now_ms;
        event.sequence = existing.sequence.saturating_add(1);

        self.write_event(&event).await?;
        debug!(event_id, "updated event");
        Ok(event)
    }

    /// Delete an event by ID.
    pub async fn delete_event(&self, event_id: &str) -> Result<(), CalendarError> {
        let event = self.get_event(event_id).await?;
        let key = event_key(&event.calendar_id, &event.id);
        self.store
            .delete(DeleteRequest::new(&key))
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;
        debug!(event_id, "deleted event");
        Ok(())
    }

    // ========================================================================
    // Query operations
    // ========================================================================

    /// List events in a calendar with optional time-range filtering.
    pub async fn list_events(
        &self,
        calendar_id: &str,
        start_ms: Option<u64>,
        end_ms: Option<u64>,
        limit: Option<u32>,
    ) -> Result<(Vec<CalendarEvent>, Option<String>), CalendarError> {
        let limit = clamp_limit(limit);
        let prefix = format!("{EVENT_PREFIX}{calendar_id}:");

        // Scan all events and filter by time range client-side.
        let scan = self
            .store
            .scan(ScanRequest {
                prefix,
                limit_results: Some(MAX_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        let mut events = Vec::new();
        for entry in &scan.entries {
            if events.len() as u32 >= limit {
                break;
            }
            let event: CalendarEvent = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize event: {e}"),
            })?;

            if event_in_range(&event, start_ms, end_ms) {
                events.push(event);
            }
        }

        Ok((events, scan.continuation_token))
    }

    /// Search events by summary/description substring.
    pub async fn search_events(
        &self,
        query: &str,
        calendar_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        let limit = clamp_limit(limit);
        let prefix = match calendar_id {
            Some(cid) => format!("{EVENT_PREFIX}{cid}:"),
            None => EVENT_PREFIX.to_string(),
        };

        let scan = self
            .store
            .scan(ScanRequest {
                prefix,
                limit_results: Some(MAX_LIST_LIMIT),
                continuation_token: None,
            })
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in &scan.entries {
            if results.len() as u32 >= limit {
                break;
            }
            let event: CalendarEvent = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize event: {e}"),
            })?;

            if event_matches_query(&event, &query_lower) {
                results.push(event);
            }
        }

        Ok(results)
    }

    /// Query free/busy periods in a time range.
    pub async fn free_busy_query(
        &self,
        calendar_id: &str,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<Vec<(u64, u64)>, CalendarError> {
        let prefix = format!("{EVENT_PREFIX}{calendar_id}:");
        let entries = self.scan_all(&prefix).await?;

        let mut busy_periods = Vec::new();
        for entry in &entries {
            let event: CalendarEvent = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize event: {e}"),
            })?;

            if !event_in_range(&event, Some(start_ms), Some(end_ms)) {
                continue;
            }

            let event_start = event.dtstart_ms;
            let event_end = event.dtend_ms.unwrap_or(event.dtstart_ms);

            // Clamp to the query range.
            let period_start = event_start.max(start_ms);
            let period_end = event_end.min(end_ms);

            if period_start < period_end {
                busy_periods.push((period_start, period_end));
            }
        }

        // Sort by start time.
        busy_periods.sort_by_key(|(start, _)| *start);
        Ok(busy_periods)
    }

    /// Expand a recurring event's RRULE into individual instances.
    pub async fn expand_recurrence(
        &self,
        event_id: &str,
        start_ms: u64,
        end_ms: u64,
        max_instances: Option<u32>,
    ) -> Result<Vec<RecurrenceInstance>, CalendarError> {
        let event = self.get_event(event_id).await?;

        let rrule = match &event.rrule {
            Some(r) => r,
            None => return Ok(vec![]),
        };

        let max = max_instances.unwrap_or(100).min(MAX_RECURRENCE_INSTANCES);

        let duration_ms = event.dtend_ms.unwrap_or(event.dtstart_ms).saturating_sub(event.dtstart_ms);

        let instances = expand_rrule(event.dtstart_ms, duration_ms, rrule, &event.exdates, start_ms, end_ms, max)
            .map_err(|e| CalendarError::InvalidRrule { reason: e.to_string() })?;

        Ok(instances)
    }

    /// Bulk import events from an iCalendar string.
    pub async fn import_ical(
        &self,
        calendar_id: &str,
        ical_data: &str,
        now_ms: u64,
    ) -> Result<Vec<CalendarEvent>, CalendarError> {
        // Verify calendar exists.
        self.get_calendar(calendar_id).await?;

        let mut events = parse_vcalendar(ical_data).map_err(|e| CalendarError::ParseIcal { reason: e.to_string() })?;

        for (idx, event) in events.iter_mut().enumerate() {
            event.calendar_id = calendar_id.to_string();
            if event.id.is_empty() {
                let seed = if event.uid.is_empty() {
                    format!("{}:{}:{}:{}", calendar_id, event.summary, now_ms, idx)
                } else {
                    event.uid.clone()
                };
                event.id = generate_id("event", &seed);
            }
            event.created_at_ms = now_ms;
            event.updated_at_ms = now_ms;
        }

        // Write all events.
        let pairs: Vec<(String, String)> = events
            .iter()
            .map(|e| {
                let key = event_key(&e.calendar_id, &e.id);
                let value = serde_json::to_string(e).unwrap_or_default();
                (key, value)
            })
            .collect();

        if !pairs.is_empty() {
            self.store
                .write(WriteRequest::from_command(WriteCommand::SetMulti { pairs }))
                .await
                .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;
        }

        debug!(calendar_id, count = events.len(), "imported events from iCal");
        Ok(events)
    }

    /// Export all events in a calendar as an iCalendar string.
    pub async fn export_ical(&self, calendar_id: &str) -> Result<(String, u32), CalendarError> {
        // Verify calendar exists.
        self.get_calendar(calendar_id).await?;

        let prefix = format!("{EVENT_PREFIX}{calendar_id}:");
        let entries = self.scan_all(&prefix).await?;

        let mut events = Vec::with_capacity(entries.len());
        for entry in &entries {
            let event: CalendarEvent = serde_json::from_str(&entry.value).map_err(|e| CalendarError::StorageError {
                reason: format!("deserialize event: {e}"),
            })?;
            events.push(event);
        }

        let count = events.len() as u32;
        let ical_data = serialize_vcalendar(&events);
        Ok((ical_data, count))
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Write an event to the KV store.
    async fn write_event(&self, event: &CalendarEvent) -> Result<(), CalendarError> {
        let key = event_key(&event.calendar_id, &event.id);
        let value = serde_json::to_string(event).map_err(|e| CalendarError::StorageError {
            reason: format!("serialize event: {e}"),
        })?;
        self.store
            .write(WriteRequest::set(&key, &value))
            .await
            .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;
        Ok(())
    }

    /// Read a single value from the KV store, returning None if not found.
    async fn kv_read(&self, key: &str) -> Result<Option<String>, CalendarError> {
        match self.store.read(ReadRequest::new(key)).await {
            Ok(result) => Ok(result.kv.map(|kv| kv.value)),
            Err(aspen_kv_types::KeyValueStoreError::NotFound { .. }) => Ok(None),
            Err(e) => Err(CalendarError::StorageError { reason: e.to_string() }),
        }
    }

    /// Scan all entries matching a prefix (paginating through all results).
    async fn scan_all(&self, prefix: &str) -> Result<Vec<aspen_kv_types::KeyValueWithRevision>, CalendarError> {
        let mut all_entries = Vec::new();
        let mut continuation_token = None;

        loop {
            let scan = self
                .store
                .scan(ScanRequest {
                    prefix: prefix.to_string(),
                    limit_results: Some(MAX_LIST_LIMIT),
                    continuation_token,
                })
                .await
                .map_err(|e| CalendarError::StorageError { reason: e.to_string() })?;

            all_entries.extend(scan.entries);

            if scan.is_truncated {
                continuation_token = scan.continuation_token;
            } else {
                break;
            }
        }

        Ok(all_entries)
    }
}

// ============================================================================
// Free functions
// ============================================================================

/// Build the KV key for an event.
fn event_key(calendar_id: &str, event_id: &str) -> String {
    format!("{EVENT_PREFIX}{calendar_id}:{event_id}")
}

/// Generate a deterministic ID from a seed.
fn generate_id(prefix: &str, seed: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hash;
    use std::hash::Hasher;

    let mut hasher = DefaultHasher::new();
    prefix.hash(&mut hasher);
    seed.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Clamp a limit to the valid range.
fn clamp_limit(limit: Option<u32>) -> u32 {
    limit.unwrap_or(DEFAULT_LIST_LIMIT).min(MAX_LIST_LIMIT)
}

/// Check if an event falls within a time range.
fn event_in_range(event: &CalendarEvent, start_ms: Option<u64>, end_ms: Option<u64>) -> bool {
    let event_end = event.dtend_ms.unwrap_or(event.dtstart_ms);

    if let Some(start) = start_ms {
        // Event ends before the range starts — exclude.
        if event_end < start {
            return false;
        }
    }
    if let Some(end) = end_ms {
        // Event starts after the range ends — exclude.
        if event.dtstart_ms > end {
            return false;
        }
    }
    true
}

/// Check if an event matches a search query (summary/description substring).
fn event_matches_query(event: &CalendarEvent, query_lower: &str) -> bool {
    if event.summary.to_lowercase().contains(query_lower) {
        return true;
    }
    if let Some(desc) = &event.description
        && desc.to_lowercase().contains(query_lower) {
            return true;
        }
    if let Some(loc) = &event.location
        && loc.to_lowercase().contains(query_lower) {
            return true;
        }
    false
}

#[cfg(test)]
mod tests {
    use aspen_testing_core::DeterministicKeyValueStore;

    use super::*;

    fn make_store() -> CalendarStore<DeterministicKeyValueStore> {
        CalendarStore::new(DeterministicKeyValueStore::new())
    }

    #[tokio::test]
    async fn test_create_and_get_calendar() {
        let store = make_store();
        let cal = store.create_calendar("Work", Some("#4285f4"), Some("America/New_York"), None, 1000).await.unwrap();
        assert_eq!(cal.name, "Work");
        assert_eq!(cal.color.as_deref(), Some("#4285f4"));
        assert_eq!(cal.timezone.as_deref(), Some("America/New_York"));
        assert_eq!(cal.created_at_ms, 1000);

        let fetched = store.get_calendar(&cal.id).await.unwrap();
        assert_eq!(fetched.name, "Work");
    }

    #[tokio::test]
    async fn test_create_calendar_empty_name_fails() {
        let store = make_store();
        let result = store.create_calendar("", None, None, None, 1000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_calendars() {
        let store = make_store();
        store.create_calendar("Work", None, None, None, 1000).await.unwrap();
        store.create_calendar("Personal", None, None, None, 2000).await.unwrap();

        let cals = store.list_calendars(None).await.unwrap();
        assert_eq!(cals.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_calendar_cascades() {
        let store = make_store();
        let cal = store.create_calendar("Temp", None, None, None, 1000).await.unwrap();

        let ical = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Meeting\r\nDTSTART:20260315T100000Z\r\nDTEND:20260315T110000Z\r\nEND:VEVENT\r\n";
        store.create_event(&cal.id, ical, 2000).await.unwrap();

        store.delete_calendar(&cal.id).await.unwrap();

        assert!(store.get_calendar(&cal.id).await.is_err());
        let (events, _) = store.list_events(&cal.id, None, None, None).await.unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn test_create_and_get_event() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        let ical = "BEGIN:VEVENT\r\nUID:meeting-001\r\nSUMMARY:Team Meeting\r\nDESCRIPTION:Weekly sync\r\nLOCATION:Room 42\r\nDTSTART:20260315T100000Z\r\nDTEND:20260315T110000Z\r\nEND:VEVENT\r\n";
        let event = store.create_event(&cal.id, ical, 2000).await.unwrap();

        assert_eq!(event.summary, "Team Meeting");
        assert_eq!(event.description.as_deref(), Some("Weekly sync"));
        assert_eq!(event.location.as_deref(), Some("Room 42"));
        assert_eq!(event.created_at_ms, 2000);

        let fetched = store.get_event(&event.id).await.unwrap();
        assert_eq!(fetched.summary, "Team Meeting");
    }

    #[tokio::test]
    async fn test_update_event() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        let ical1 = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Meeting\r\nDTSTART:20260315T100000Z\r\nEND:VEVENT\r\n";
        let event = store.create_event(&cal.id, ical1, 2000).await.unwrap();
        assert_eq!(event.sequence, 0);

        let ical2 = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Updated Meeting\r\nDTSTART:20260315T100000Z\r\nEND:VEVENT\r\n";
        let updated = store.update_event(&event.id, ical2, 3000).await.unwrap();

        assert_eq!(updated.summary, "Updated Meeting");
        assert_eq!(updated.id, event.id);
        assert_eq!(updated.calendar_id, cal.id);
        assert_eq!(updated.created_at_ms, 2000);
        assert_eq!(updated.updated_at_ms, 3000);
        assert_eq!(updated.sequence, 1);
    }

    #[tokio::test]
    async fn test_delete_event() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        let ical = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Temp\r\nDTSTART:20260315T100000Z\r\nEND:VEVENT\r\n";
        let event = store.create_event(&cal.id, ical, 2000).await.unwrap();

        store.delete_event(&event.id).await.unwrap();
        assert!(store.get_event(&event.id).await.is_err());
    }

    #[tokio::test]
    async fn test_list_events_time_range() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        // Event 1: March 15, 10:00-11:00
        let ical1 = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Morning\r\nDTSTART:20260315T100000Z\r\nDTEND:20260315T110000Z\r\nEND:VEVENT\r\n";
        // Event 2: March 16, 14:00-15:00
        let ical2 = "BEGIN:VEVENT\r\nUID:e2\r\nSUMMARY:Afternoon\r\nDTSTART:20260316T140000Z\r\nDTEND:20260316T150000Z\r\nEND:VEVENT\r\n";
        store.create_event(&cal.id, ical1, 2000).await.unwrap();
        store.create_event(&cal.id, ical2, 2001).await.unwrap();

        // Query for March 15 only (in ms): 2026-03-15 00:00:00 to 2026-03-15 23:59:59
        let start = 1773_547_200_000u64; // 2026-03-15T00:00:00Z
        let end = 1773_633_599_000u64; // 2026-03-15T23:59:59Z
        let (events, _) = store.list_events(&cal.id, Some(start), Some(end), None).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].summary, "Morning");
    }

    #[tokio::test]
    async fn test_search_events() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        let ical1 = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Team Standup\r\nDTSTART:20260315T100000Z\r\nEND:VEVENT\r\n";
        let ical2 = "BEGIN:VEVENT\r\nUID:e2\r\nSUMMARY:Lunch Break\r\nDTSTART:20260315T120000Z\r\nEND:VEVENT\r\n";
        store.create_event(&cal.id, ical1, 2000).await.unwrap();
        store.create_event(&cal.id, ical2, 2001).await.unwrap();

        let results = store.search_events("standup", Some(&cal.id), None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].summary, "Team Standup");
    }

    #[tokio::test]
    async fn test_free_busy_query() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        // 10:00-11:00
        let ical1 = "BEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Meeting 1\r\nDTSTART:20260315T100000Z\r\nDTEND:20260315T110000Z\r\nEND:VEVENT\r\n";
        // 14:00-15:00
        let ical2 = "BEGIN:VEVENT\r\nUID:e2\r\nSUMMARY:Meeting 2\r\nDTSTART:20260315T140000Z\r\nDTEND:20260315T150000Z\r\nEND:VEVENT\r\n";
        store.create_event(&cal.id, ical1, 2000).await.unwrap();
        store.create_event(&cal.id, ical2, 2001).await.unwrap();

        let start = 1773_547_200_000u64; // 2026-03-15T00:00:00Z
        let end = 1773_633_600_000u64; // 2026-03-16T00:00:00Z
        let periods = store.free_busy_query(&cal.id, start, end).await.unwrap();
        assert_eq!(periods.len(), 2);
    }

    #[tokio::test]
    async fn test_import_export_ical() {
        let store = make_store();
        let cal = store.create_calendar("Work", None, None, None, 1000).await.unwrap();

        let ical = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:e1\r\nSUMMARY:Event 1\r\nDTSTART:20260315T100000Z\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nUID:e2\r\nSUMMARY:Event 2\r\nDTSTART:20260316T100000Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let imported = store.import_ical(&cal.id, ical, 2000).await.unwrap();
        assert_eq!(imported.len(), 2);

        let (exported, count) = store.export_ical(&cal.id).await.unwrap();
        assert_eq!(count, 2);
        assert!(exported.contains("Event 1"));
        assert!(exported.contains("Event 2"));
    }

    #[tokio::test]
    async fn test_get_nonexistent_event() {
        let store = make_store();
        assert!(store.get_event("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_get_nonexistent_calendar() {
        let store = make_store();
        assert!(store.get_calendar("nonexistent").await.is_err());
    }

    #[test]
    fn test_event_in_range() {
        let event = CalendarEvent {
            id: "e1".into(),
            calendar_id: "c1".into(),
            uid: "u1".into(),
            summary: "Test".into(),
            description: None,
            location: None,
            dtstart_ms: 1000,
            dtend_ms: Some(2000),
            is_all_day: false,
            rrule: None,
            exdates: vec![],
            attendees: vec![],
            alarms: vec![],
            status: crate::EventStatus::Confirmed,
            categories: vec![],
            url: None,
            created_at_ms: 0,
            updated_at_ms: 0,
            sequence: 0,
        };

        // No range — always in range.
        assert!(event_in_range(&event, None, None));
        // Within range.
        assert!(event_in_range(&event, Some(500), Some(2500)));
        // Exact match.
        assert!(event_in_range(&event, Some(1000), Some(2000)));
        // Before range.
        assert!(!event_in_range(&event, Some(3000), Some(4000)));
        // After range.
        assert!(!event_in_range(&event, Some(0), Some(500)));
    }

    #[test]
    fn test_event_matches_query() {
        let event = CalendarEvent {
            id: "e1".into(),
            calendar_id: "c1".into(),
            uid: "u1".into(),
            summary: "Team Meeting".into(),
            description: Some("Weekly sync with engineering".into()),
            location: Some("Conference Room A".into()),
            dtstart_ms: 0,
            dtend_ms: None,
            is_all_day: false,
            rrule: None,
            exdates: vec![],
            attendees: vec![],
            alarms: vec![],
            status: crate::EventStatus::Confirmed,
            categories: vec![],
            url: None,
            created_at_ms: 0,
            updated_at_ms: 0,
            sequence: 0,
        };

        assert!(event_matches_query(&event, "meeting"));
        assert!(event_matches_query(&event, "engineering"));
        assert!(event_matches_query(&event, "conference"));
        assert!(!event_matches_query(&event, "lunch"));
    }
}
