//! Calendar service executor — dispatches RPC requests to `CalendarStore`.

use std::sync::Arc;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use anyhow::Result;
use aspen_calendar::CalendarEvent;
use aspen_calendar::CalendarStore;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use aspen_client_api::calendar::BusyPeriod;
use aspen_client_api::calendar::CalendarEventResponse;
use aspen_client_api::calendar::CalendarExpandResponse;
use aspen_client_api::calendar::CalendarExportResponse;
use aspen_client_api::calendar::CalendarFreeBusyResponse;
use aspen_client_api::calendar::CalendarInfo;
use aspen_client_api::calendar::CalendarListEventsResponse;
use aspen_client_api::calendar::CalendarListResponse;
use aspen_client_api::calendar::CalendarResponse;
use aspen_client_api::calendar::CalendarSearchResponse;
use aspen_client_api::calendar::EventInstance;
use aspen_client_api::calendar::EventSummary;
use aspen_traits::KeyValueStore;
use async_trait::async_trait;
use tracing::warn;

/// Handles list for calendar operations.
const HANDLES: &[&str] = &[
    "CalendarCreate",
    "CalendarDelete",
    "CalendarList",
    "CalendarGetEvent",
    "CalendarCreateEvent",
    "CalendarUpdateEvent",
    "CalendarDeleteEvent",
    "CalendarListEvents",
    "CalendarSearchEvents",
    "CalendarImportIcal",
    "CalendarExportIcal",
    "CalendarFreeBusy",
    "CalendarExpandRecurrence",
];

/// Calendar service executor backed by `CalendarStore`.
pub struct CalendarServiceExecutor {
    store: CalendarStore<dyn KeyValueStore>,
}

impl CalendarServiceExecutor {
    /// Create a new executor.
    pub fn new(kv_store: Arc<dyn KeyValueStore>) -> Self {
        Self {
            store: CalendarStore::new(kv_store),
        }
    }
}

#[async_trait]
impl aspen_rpc_core::ServiceExecutor for CalendarServiceExecutor {
    fn service_name(&self) -> &'static str {
        "calendar"
    }

    fn handles(&self) -> &'static [&'static str] {
        HANDLES
    }

    fn priority(&self) -> u32 {
        360
    }

    fn app_id(&self) -> Option<&'static str> {
        Some("calendar")
    }

    async fn execute(&self, request: ClientRpcRequest) -> Result<ClientRpcResponse> {
        match request {
            ClientRpcRequest::CalendarCreate {
                name,
                color,
                timezone,
                description,
            } => self.handle_create(&name, color.as_deref(), timezone.as_deref(), description.as_deref()).await,
            ClientRpcRequest::CalendarDelete { calendar_id } => self.handle_delete(&calendar_id).await,
            ClientRpcRequest::CalendarList { limit } => self.handle_list(limit).await,
            ClientRpcRequest::CalendarGetEvent { event_id } => self.handle_get_event(&event_id).await,
            ClientRpcRequest::CalendarCreateEvent { calendar_id, ical_data } => {
                self.handle_create_event(&calendar_id, &ical_data).await
            }
            ClientRpcRequest::CalendarUpdateEvent { event_id, ical_data } => {
                self.handle_update_event(&event_id, &ical_data).await
            }
            ClientRpcRequest::CalendarDeleteEvent { event_id } => self.handle_delete_event(&event_id).await,
            ClientRpcRequest::CalendarListEvents {
                calendar_id,
                start_ms,
                end_ms,
                limit,
                ..
            } => self.handle_list_events(&calendar_id, start_ms, end_ms, limit).await,
            ClientRpcRequest::CalendarSearchEvents {
                query,
                calendar_id,
                limit,
            } => self.handle_search_events(&query, calendar_id.as_deref(), limit).await,
            ClientRpcRequest::CalendarImportIcal { calendar_id, ical_data } => {
                self.handle_import_ical(&calendar_id, &ical_data).await
            }
            ClientRpcRequest::CalendarExportIcal { calendar_id } => self.handle_export_ical(&calendar_id).await,
            ClientRpcRequest::CalendarFreeBusy {
                calendar_id,
                start_ms,
                end_ms,
            } => self.handle_free_busy(&calendar_id, start_ms, end_ms).await,
            ClientRpcRequest::CalendarExpandRecurrence {
                event_id,
                start_ms,
                end_ms,
                max_instances,
            } => self.handle_expand_recurrence(&event_id, start_ms, end_ms, max_instances).await,
            other => {
                warn!(variant = %other.variant_name(), "unexpected request in calendar executor");
                Ok(ClientRpcResponse::error("INTERNAL_ERROR", "unexpected request"))
            }
        }
    }
}

impl CalendarServiceExecutor {
    fn now_ms() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
    }

    async fn handle_create(
        &self,
        name: &str,
        color: Option<&str>,
        timezone: Option<&str>,
        description: Option<&str>,
    ) -> Result<ClientRpcResponse> {
        match self.store.create_calendar(name, color, timezone, description, Self::now_ms()).await {
            Ok(cal) => Ok(ClientRpcResponse::CalendarResult(CalendarResponse {
                is_success: true,
                calendar_id: Some(cal.id),
                name: Some(cal.name),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarResult(CalendarResponse {
                is_success: false,
                calendar_id: None,
                name: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_delete(&self, calendar_id: &str) -> Result<ClientRpcResponse> {
        match self.store.delete_calendar(calendar_id).await {
            Ok(()) => Ok(ClientRpcResponse::CalendarResult(CalendarResponse {
                is_success: true,
                calendar_id: Some(calendar_id.to_string()),
                name: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarResult(CalendarResponse {
                is_success: false,
                calendar_id: Some(calendar_id.to_string()),
                name: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_list(&self, limit: Option<u32>) -> Result<ClientRpcResponse> {
        match self.store.list_calendars(limit).await {
            Ok(cals) => Ok(ClientRpcResponse::CalendarListResult(CalendarListResponse {
                is_success: true,
                calendars: cals
                    .into_iter()
                    .map(|c| CalendarInfo {
                        id: c.id,
                        name: c.name,
                        color: c.color,
                        timezone: c.timezone,
                        event_count: 0, // Would need a separate count query
                    })
                    .collect(),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarListResult(CalendarListResponse {
                is_success: false,
                calendars: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_create_event(&self, calendar_id: &str, ical_data: &str) -> Result<ClientRpcResponse> {
        match self.store.create_event(calendar_id, ical_data, Self::now_ms()).await {
            Ok(event) => {
                let ical = aspen_calendar::serialize_vevent(&event);
                Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                    is_success: true,
                    event_id: Some(event.id),
                    ical_data: Some(ical),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                is_success: false,
                event_id: None,
                ical_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_get_event(&self, event_id: &str) -> Result<ClientRpcResponse> {
        match self.store.get_event(event_id).await {
            Ok(event) => {
                let ical = aspen_calendar::serialize_vevent(&event);
                Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                    is_success: true,
                    event_id: Some(event.id),
                    ical_data: Some(ical),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                is_success: false,
                event_id: Some(event_id.to_string()),
                ical_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_update_event(&self, event_id: &str, ical_data: &str) -> Result<ClientRpcResponse> {
        match self.store.update_event(event_id, ical_data, Self::now_ms()).await {
            Ok(event) => {
                let ical = aspen_calendar::serialize_vevent(&event);
                Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                    is_success: true,
                    event_id: Some(event.id),
                    ical_data: Some(ical),
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                is_success: false,
                event_id: Some(event_id.to_string()),
                ical_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_delete_event(&self, event_id: &str) -> Result<ClientRpcResponse> {
        match self.store.delete_event(event_id).await {
            Ok(()) => Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                is_success: true,
                event_id: Some(event_id.to_string()),
                ical_data: None,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarEventResult(CalendarEventResponse {
                is_success: false,
                event_id: Some(event_id.to_string()),
                ical_data: None,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_list_events(
        &self,
        calendar_id: &str,
        start_ms: Option<u64>,
        end_ms: Option<u64>,
        limit: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        match self.store.list_events(calendar_id, start_ms, end_ms, limit).await {
            Ok((events, token)) => {
                let total = events.len() as u32;
                Ok(ClientRpcResponse::CalendarListEventsResult(CalendarListEventsResponse {
                    is_success: true,
                    events: events.into_iter().map(to_event_summary).collect(),
                    continuation_token: token,
                    total,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarListEventsResult(CalendarListEventsResponse {
                is_success: false,
                events: vec![],
                continuation_token: None,
                total: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_search_events(
        &self,
        query: &str,
        calendar_id: Option<&str>,
        limit: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        match self.store.search_events(query, calendar_id, limit).await {
            Ok(events) => {
                let total = events.len() as u32;
                Ok(ClientRpcResponse::CalendarSearchResult(CalendarSearchResponse {
                    is_success: true,
                    events: events.into_iter().map(to_event_summary).collect(),
                    total,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarSearchResult(CalendarSearchResponse {
                is_success: false,
                events: vec![],
                total: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_free_busy(&self, calendar_id: &str, start_ms: u64, end_ms: u64) -> Result<ClientRpcResponse> {
        match self.store.free_busy_query(calendar_id, start_ms, end_ms).await {
            Ok(periods) => Ok(ClientRpcResponse::CalendarFreeBusyResult(CalendarFreeBusyResponse {
                is_success: true,
                busy_periods: periods
                    .into_iter()
                    .map(|(start, end)| BusyPeriod {
                        start_ms: start,
                        end_ms: end,
                        summary: None,
                    })
                    .collect(),
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarFreeBusyResult(CalendarFreeBusyResponse {
                is_success: false,
                busy_periods: vec![],
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_expand_recurrence(
        &self,
        event_id: &str,
        start_ms: u64,
        end_ms: u64,
        max_instances: Option<u32>,
    ) -> Result<ClientRpcResponse> {
        match self.store.expand_recurrence(event_id, start_ms, end_ms, max_instances).await {
            Ok(instances) => {
                let total = instances.len() as u32;
                Ok(ClientRpcResponse::CalendarExpandResult(CalendarExpandResponse {
                    is_success: true,
                    instances: instances
                        .into_iter()
                        .map(|i| EventInstance {
                            dtstart_ms: i.dtstart_ms,
                            dtend_ms: i.dtend_ms,
                            is_exception: false,
                        })
                        .collect(),
                    total,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarExpandResult(CalendarExpandResponse {
                is_success: false,
                instances: vec![],
                total: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_import_ical(&self, calendar_id: &str, ical_data: &str) -> Result<ClientRpcResponse> {
        match self.store.import_ical(calendar_id, ical_data, Self::now_ms()).await {
            Ok(events) => {
                let count = events.len() as u32;
                Ok(ClientRpcResponse::CalendarExportResult(CalendarExportResponse {
                    is_success: true,
                    ical_data: None,
                    count,
                    error: None,
                }))
            }
            Err(e) => Ok(ClientRpcResponse::CalendarExportResult(CalendarExportResponse {
                is_success: false,
                ical_data: None,
                count: 0,
                error: Some(e.to_string()),
            })),
        }
    }

    async fn handle_export_ical(&self, calendar_id: &str) -> Result<ClientRpcResponse> {
        match self.store.export_ical(calendar_id).await {
            Ok((data, count)) => Ok(ClientRpcResponse::CalendarExportResult(CalendarExportResponse {
                is_success: true,
                ical_data: Some(data),
                count,
                error: None,
            })),
            Err(e) => Ok(ClientRpcResponse::CalendarExportResult(CalendarExportResponse {
                is_success: false,
                ical_data: None,
                count: 0,
                error: Some(e.to_string()),
            })),
        }
    }
}

fn to_event_summary(event: CalendarEvent) -> EventSummary {
    EventSummary {
        id: event.id,
        calendar_id: event.calendar_id,
        summary: event.summary,
        dtstart_ms: event.dtstart_ms,
        dtend_ms: event.dtend_ms,
        is_all_day: event.is_all_day,
        location: event.location,
    }
}
