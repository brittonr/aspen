//! Calendar CLI commands.
//!
//! Commands for managing calendars, events, recurring events,
//! free/busy queries, and iCal import/export.

use anyhow::Result;
use aspen_client_api::ClientRpcRequest;
use aspen_client_api::ClientRpcResponse;
use clap::Args;
use clap::Subcommand;

use crate::client::AspenClient;
use crate::output::Outputable;
use crate::output::print_output;

/// Calendar operations.
#[derive(Subcommand)]
pub enum CalendarCommand {
    /// Create a calendar.
    Create(CreateCalArgs),

    /// List calendars.
    List(ListCalArgs),

    /// Delete a calendar and all its events.
    Delete(DeleteCalArgs),

    /// Add an event to a calendar.
    AddEvent(AddEventArgs),

    /// Get an event by ID (returns iCal).
    GetEvent(GetEventArgs),

    /// Update an event with new iCal data.
    UpdateEvent(UpdateEventArgs),

    /// Delete an event.
    DeleteEvent(DeleteEventArgs),

    /// List events in a calendar (with optional time range).
    ListEvents(ListEventsArgs),

    /// Search events by summary or description.
    Search(SearchEventsArgs),

    /// Query free/busy periods.
    FreeBusy(FreeBusyArgs),

    /// Import events from an iCal file.
    Import(ImportIcalArgs),

    /// Export events as iCalendar format.
    Export(ExportIcalArgs),
}

#[derive(Args)]
pub struct CreateCalArgs {
    /// Calendar name.
    pub name: String,
    /// Display color (hex, e.g. "#4285f4").
    #[arg(long)]
    pub color: Option<String>,
    /// IANA timezone (e.g. "America/New_York").
    #[arg(long)]
    pub timezone: Option<String>,
    /// Description.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct ListCalArgs {
    /// Maximum number of calendars to return.
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Args)]
pub struct DeleteCalArgs {
    /// Calendar ID.
    pub calendar_id: String,
}

#[derive(Args)]
pub struct AddEventArgs {
    /// Calendar ID.
    #[arg(long)]
    pub calendar: String,
    /// Event summary/title.
    #[arg(long)]
    pub summary: String,
    /// Start time (ISO 8601 datetime, e.g. "2026-03-15T10:00").
    #[arg(long)]
    pub start: String,
    /// End time (ISO 8601 datetime).
    #[arg(long)]
    pub end: Option<String>,
    /// Location.
    #[arg(long)]
    pub location: Option<String>,
    /// Description.
    #[arg(long)]
    pub description: Option<String>,
}

#[derive(Args)]
pub struct GetEventArgs {
    /// Event ID.
    pub event_id: String,
}

#[derive(Args)]
pub struct UpdateEventArgs {
    /// Event ID.
    pub event_id: String,
    /// iCalendar data string.
    #[arg(long)]
    pub ical: String,
}

#[derive(Args)]
pub struct DeleteEventArgs {
    /// Event ID.
    pub event_id: String,
}

#[derive(Args)]
pub struct ListEventsArgs {
    /// Calendar ID.
    #[arg(long)]
    pub calendar: String,
    /// Start of time range (Unix ms).
    #[arg(long)]
    pub start: Option<u64>,
    /// End of time range (Unix ms).
    #[arg(long)]
    pub end: Option<u64>,
    /// Maximum number of events.
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Args)]
pub struct SearchEventsArgs {
    /// Search query (matches summary and description).
    pub query: String,
    /// Restrict to a specific calendar.
    #[arg(long)]
    pub calendar: Option<String>,
    /// Maximum number of results.
    #[arg(long)]
    pub limit: Option<u32>,
}

#[derive(Args)]
pub struct FreeBusyArgs {
    /// Calendar ID.
    #[arg(long)]
    pub calendar: String,
    /// Start of query range (Unix ms).
    #[arg(long)]
    pub start: u64,
    /// End of query range (Unix ms).
    #[arg(long)]
    pub end: u64,
}

#[derive(Args)]
pub struct ImportIcalArgs {
    /// Calendar ID.
    #[arg(long)]
    pub calendar: String,
    /// Path to iCal (.ics) file.
    pub file: String,
}

#[derive(Args)]
pub struct ExportIcalArgs {
    /// Calendar ID.
    #[arg(long)]
    pub calendar: String,
}

impl CalendarCommand {
    pub async fn run(self, client: &AspenClient, json: bool) -> Result<()> {
        match self {
            Self::Create(a) => create_calendar(client, a, json).await,
            Self::List(a) => list_calendars(client, a, json).await,
            Self::Delete(a) => delete_calendar(client, a, json).await,
            Self::AddEvent(a) => add_event(client, a, json).await,
            Self::GetEvent(a) => get_event(client, a, json).await,
            Self::UpdateEvent(a) => update_event(client, a, json).await,
            Self::DeleteEvent(a) => delete_event(client, a, json).await,
            Self::ListEvents(a) => list_events(client, a, json).await,
            Self::Search(a) => search_events(client, a, json).await,
            Self::FreeBusy(a) => free_busy(client, a, json).await,
            Self::Import(a) => import_ical(client, a, json).await,
            Self::Export(a) => export_ical(client, a, json).await,
        }
    }
}

async fn create_calendar(client: &AspenClient, args: CreateCalArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarCreate {
            name: args.name,
            color: args.color,
            timezone: args.timezone,
            description: args.description,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.calendar_id,
                    name: r.name,
                    error: r.error,
                    label: "calendar",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_calendars(client: &AspenClient, args: ListCalArgs, json: bool) -> Result<()> {
    let resp = client.send(ClientRpcRequest::CalendarList { limit: args.limit }).await?;
    match resp {
        ClientRpcResponse::CalendarListResult(r) => {
            let output = CalListOutput {
                calendars: r.calendars,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_calendar(client: &AspenClient, args: DeleteCalArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarDelete {
            calendar_id: args.calendar_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.calendar_id,
                    name: None,
                    error: r.error,
                    label: "calendar",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn add_event(client: &AspenClient, args: AddEventArgs, json: bool) -> Result<()> {
    // Build a minimal VEVENT from CLI args.
    let dtstart = args.start.replace(['-', ':'], "");
    let mut ical = format!("BEGIN:VEVENT\r\nSUMMARY:{}\r\nDTSTART:{dtstart}\r\n", args.summary);
    if let Some(end) = &args.end {
        let dtend = end.replace(['-', ':'], "");
        ical.push_str(&format!("DTEND:{dtend}\r\n"));
    }
    if let Some(loc) = &args.location {
        ical.push_str(&format!("LOCATION:{loc}\r\n"));
    }
    if let Some(desc) = &args.description {
        ical.push_str(&format!("DESCRIPTION:{desc}\r\n"));
    }
    ical.push_str(&format!(
        "UID:cli-{}\r\nEND:VEVENT\r\n",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()
    ));

    let resp = client
        .send(ClientRpcRequest::CalendarCreateEvent {
            calendar_id: args.calendar,
            ical_data: ical,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.event_id,
                    name: None,
                    error: r.error,
                    label: "event",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn get_event(client: &AspenClient, args: GetEventArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarGetEvent {
            event_id: args.event_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            if json {
                print_output(
                    &IcalOutput {
                        ical_data: r.ical_data,
                        event_id: r.event_id,
                        error: r.error,
                    },
                    json,
                );
            } else if let Some(ical) = &r.ical_data {
                println!("{ical}");
            } else if let Some(err) = &r.error {
                eprintln!("Error: {err}");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn update_event(client: &AspenClient, args: UpdateEventArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarUpdateEvent {
            event_id: args.event_id,
            ical_data: args.ical,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.event_id,
                    name: None,
                    error: r.error,
                    label: "event",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn delete_event(client: &AspenClient, args: DeleteEventArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarDeleteEvent {
            event_id: args.event_id,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarEventResult(r) => {
            print_output(
                &SimpleResult {
                    is_success: r.is_success,
                    id: r.event_id,
                    name: None,
                    error: r.error,
                    label: "event",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn list_events(client: &AspenClient, args: ListEventsArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarListEvents {
            calendar_id: args.calendar,
            start_ms: args.start,
            end_ms: args.end,
            limit: args.limit,
            continuation_token: None,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarListEventsResult(r) => {
            let output = EventListOutput {
                events: r.events,
                total: r.total,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn search_events(client: &AspenClient, args: SearchEventsArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarSearchEvents {
            query: args.query,
            calendar_id: args.calendar,
            limit: args.limit,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarSearchResult(r) => {
            let output = EventListOutput {
                events: r.events,
                total: r.total,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn free_busy(client: &AspenClient, args: FreeBusyArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarFreeBusy {
            calendar_id: args.calendar,
            start_ms: args.start,
            end_ms: args.end,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarFreeBusyResult(r) => {
            let output = FreeBusyOutput {
                periods: r.busy_periods,
                error: r.error,
            };
            print_output(&output, json);
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn import_ical(client: &AspenClient, args: ImportIcalArgs, json: bool) -> Result<()> {
    let ical_data =
        std::fs::read_to_string(&args.file).map_err(|e| anyhow::anyhow!("failed to read {}: {}", args.file, e))?;
    let resp = client
        .send(ClientRpcRequest::CalendarImportIcal {
            calendar_id: args.calendar,
            ical_data,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarExportResult(r) => {
            print_output(
                &ImportExportOutput {
                    count: r.count,
                    is_success: r.is_success,
                    error: r.error,
                    op: "imported",
                },
                json,
            );
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

async fn export_ical(client: &AspenClient, args: ExportIcalArgs, json: bool) -> Result<()> {
    let resp = client
        .send(ClientRpcRequest::CalendarExportIcal {
            calendar_id: args.calendar,
        })
        .await?;
    match resp {
        ClientRpcResponse::CalendarExportResult(r) => {
            if json {
                print_output(
                    &ImportExportOutput {
                        count: r.count,
                        is_success: r.is_success,
                        error: r.error,
                        op: "exported",
                    },
                    json,
                );
            } else if let Some(data) = &r.ical_data {
                print!("{data}");
            } else if let Some(err) = &r.error {
                eprintln!("Error: {err}");
            }
            Ok(())
        }
        ClientRpcResponse::Error(e) => anyhow::bail!("{}: {}", e.code, e.message),
        _ => anyhow::bail!("unexpected response"),
    }
}

// =========================================================================
// Output types
// =========================================================================

struct SimpleResult {
    is_success: bool,
    id: Option<String>,
    name: Option<String>,
    error: Option<String>,
    label: &'static str,
}

impl Outputable for SimpleResult {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "success": self.is_success, "id": self.id, "name": self.name, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.is_success {
            let id = self.id.as_deref().unwrap_or("?");
            match &self.name {
                Some(n) => format!("{} '{}' ({})", self.label, n, id),
                None => format!("{} {}", self.label, id),
            }
        } else {
            format!("Error: {}", self.error.as_deref().unwrap_or("unknown"))
        }
    }
}

struct IcalOutput {
    ical_data: Option<String>,
    event_id: Option<String>,
    error: Option<String>,
}

impl Outputable for IcalOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "event_id": self.event_id, "ical": self.ical_data, "error": self.error })
    }
    fn to_human(&self) -> String {
        self.ical_data
            .clone()
            .unwrap_or_else(|| format!("Error: {}", self.error.as_deref().unwrap_or("not found")))
    }
}

struct CalListOutput {
    calendars: Vec<aspen_client_api::calendar::CalendarInfo>,
    error: Option<String>,
}

impl Outputable for CalListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "calendars": self.calendars.iter().map(|c| serde_json::json!({"id": c.id, "name": c.name, "color": c.color, "timezone": c.timezone, "event_count": c.event_count})).collect::<Vec<_>>(), "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.calendars.is_empty() {
            return "No calendars found.".to_string();
        }
        let mut lines = vec![format!("{:<20} {:<24} {:<10} {}", "NAME", "ID", "COLOR", "EVENTS")];
        for c in &self.calendars {
            lines.push(format!(
                "{:<20} {:<24} {:<10} {}",
                c.name,
                c.id,
                c.color.as_deref().unwrap_or("-"),
                c.event_count
            ));
        }
        lines.join("\n")
    }
}

struct EventListOutput {
    events: Vec<aspen_client_api::calendar::EventSummary>,
    total: u32,
    error: Option<String>,
}

impl Outputable for EventListOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "events": self.events.iter().map(|e| serde_json::json!({"id": e.id, "summary": e.summary, "dtstart_ms": e.dtstart_ms, "dtend_ms": e.dtend_ms, "is_all_day": e.is_all_day})).collect::<Vec<_>>(), "total": self.total, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.events.is_empty() {
            return "No events found.".to_string();
        }
        let mut lines = vec![format!(
            "{:<30} {:<18} {:<18} {}",
            "SUMMARY", "START (ms)", "END (ms)", "ID"
        )];
        for e in &self.events {
            let end = e.dtend_ms.map(|v| v.to_string()).unwrap_or_else(|| "-".into());
            lines.push(format!("{:<30} {:<18} {:<18} {}", e.summary, e.dtstart_ms, end, e.id));
        }
        lines.join("\n")
    }
}

struct FreeBusyOutput {
    periods: Vec<aspen_client_api::calendar::BusyPeriod>,
    error: Option<String>,
}

impl Outputable for FreeBusyOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "busy_periods": self.periods.iter().map(|p| serde_json::json!({"start_ms": p.start_ms, "end_ms": p.end_ms, "summary": p.summary})).collect::<Vec<_>>(), "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.periods.is_empty() {
            return "No busy periods.".to_string();
        }
        let mut lines = vec![format!("{:<18} {:<18} {}", "START (ms)", "END (ms)", "SUMMARY")];
        for p in &self.periods {
            lines.push(format!("{:<18} {:<18} {}", p.start_ms, p.end_ms, p.summary.as_deref().unwrap_or("-")));
        }
        lines.join("\n")
    }
}

struct ImportExportOutput {
    count: u32,
    is_success: bool,
    error: Option<String>,
    op: &'static str,
}

impl Outputable for ImportExportOutput {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({ "success": self.is_success, "count": self.count, "error": self.error })
    }
    fn to_human(&self) -> String {
        if self.is_success {
            format!("{} {} events", self.op, self.count)
        } else {
            format!("Error: {}", self.error.as_deref().unwrap_or("unknown"))
        }
    }
}
