# Contacts & Calendar Plugins — Design

## Architecture

Both plugins follow the extracted handler crate pattern established by `aspen-forge-handler`, `aspen-secrets-handler`, etc.

```
aspen-contacts/             Domain logic (vCard, CRUD, search)
aspen-contacts-handler/     RequestHandler impl, handler factory
aspen-calendar/             Domain logic (iCal, CRUD, RRULE)
aspen-calendar-handler/     RequestHandler impl, handler factory
```

All state is stored in the existing KV store with namespaced key prefixes. No new storage backends.

## KV Schema

### Contacts

```
contacts:book:{book_id}                    → ContactBook JSON (name, description, owner)
contacts:entry:{book_id}:{contact_id}      → Contact JSON (all vCard fields)
contacts:group:{book_id}:{group_id}        → ContactGroup JSON (name, member IDs)
contacts:photo:{contact_id}                → blob hash reference
```

**Secondary Indexes** (via `IndexCreate`):

- `contacts_email`: field=`email`, type=string — lookup by email
- `contacts_name`: field=`display_name`, type=string — lookup by name
- `contacts_phone`: field=`phone`, type=string — lookup by phone

### Calendar

```
calendar:cal:{calendar_id}                  → Calendar JSON (name, color, timezone, owner)
calendar:event:{calendar_id}:{event_id}     → CalendarEvent JSON (all iCal fields)
calendar:reminder:{event_id}:{alarm_id}     → Reminder state (scheduled timer name)
```

**Secondary Indexes**:

- `calendar_start`: field=`dtstart_ms`, type=unsignedinteger — range queries by start time
- `calendar_end`: field=`dtend_ms`, type=unsignedinteger — range queries by end time

## Data Models

### Contact

```rust
pub struct Contact {
    pub id: String,              // BLAKE3 of (book_id, uid)
    pub book_id: String,
    pub uid: String,             // vCard UID
    pub display_name: String,    // FN
    pub family_name: Option<String>,  // N family
    pub given_name: Option<String>,   // N given
    pub emails: Vec<ContactEmail>,    // EMAIL
    pub phones: Vec<ContactPhone>,    // TEL
    pub addresses: Vec<ContactAddress>, // ADR
    pub organization: Option<String>,  // ORG
    pub title: Option<String>,        // TITLE
    pub birthday: Option<String>,     // BDAY (ISO 8601 date)
    pub notes: Option<String>,        // NOTE
    pub photo_blob_hash: Option<String>, // blob ref for PHOTO
    pub categories: Vec<String>,      // CATEGORIES
    pub url: Option<String>,          // URL
    pub custom_fields: Vec<(String, String)>, // X- properties
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub revision: u64,           // KV revision for CAS
}

pub struct ContactEmail {
    pub address: String,
    pub label: Option<String>,   // "work", "home", "other"
    pub is_primary: bool,
}

pub struct ContactPhone {
    pub number: String,
    pub label: Option<String>,
    pub is_primary: bool,
}

pub struct ContactAddress {
    pub street: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,  // state/province
    pub postal_code: Option<String>,
    pub country: Option<String>,
    pub label: Option<String>,
}

pub struct ContactBook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub created_at_ms: u64,
}

pub struct ContactGroup {
    pub id: String,
    pub book_id: String,
    pub name: String,
    pub member_ids: Vec<String>,
}
```

### Calendar Event

```rust
pub struct CalendarMeta {
    pub id: String,
    pub name: String,
    pub color: Option<String>,      // hex color
    pub timezone: Option<String>,   // IANA timezone
    pub description: Option<String>,
    pub owner: Option<String>,
    pub created_at_ms: u64,
}

pub struct CalendarEvent {
    pub id: String,                 // BLAKE3 of (calendar_id, uid)
    pub calendar_id: String,
    pub uid: String,                // iCal UID
    pub summary: String,            // SUMMARY
    pub description: Option<String>, // DESCRIPTION
    pub location: Option<String>,   // LOCATION
    pub dtstart_ms: u64,            // DTSTART as unix ms
    pub dtend_ms: Option<u64>,      // DTEND as unix ms
    pub is_all_day: bool,           // DATE vs DATE-TIME
    pub rrule: Option<String>,      // RRULE string
    pub exdates: Vec<u64>,          // EXDATE as unix ms
    pub attendees: Vec<EventAttendee>,
    pub alarms: Vec<EventAlarm>,
    pub status: EventStatus,        // confirmed, tentative, cancelled
    pub categories: Vec<String>,    // CATEGORIES
    pub url: Option<String>,        // URL
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub revision: u64,
    pub sequence: u32,              // SEQUENCE number
}

pub enum EventStatus {
    Confirmed,
    Tentative,
    Cancelled,
}

pub struct EventAttendee {
    pub email: String,
    pub name: Option<String>,
    pub role: AttendeeRole,         // required, optional, chair
    pub status: AttendeeStatus,     // accepted, declined, tentative, needs-action
}

pub struct EventAlarm {
    pub id: String,
    pub trigger_minutes_before: i32, // negative = before, positive = after
    pub action: AlarmAction,        // display, email
    pub description: Option<String>,
}

pub enum AlarmAction {
    Display,
    Email,
}
```

## RPC Variants

### Contacts — ClientRpcRequest

```
ContactsCreateBook { name, description }
ContactsDeleteBook { book_id }
ContactsListBooks { limit }
ContactsCreateContact { book_id, vcard_data }     // accepts raw vCard string
ContactsGetContact { contact_id }
ContactsUpdateContact { contact_id, vcard_data }
ContactsDeleteContact { contact_id }
ContactsListContacts { book_id, limit, continuation_token }
ContactsSearchContacts { query, book_id, limit }   // searches name, email, phone
ContactsImportVcard { book_id, vcard_data }         // bulk import
ContactsExportVcard { book_id }                     // bulk export
ContactsCreateGroup { book_id, name, member_ids }
ContactsDeleteGroup { group_id }
ContactsListGroups { book_id }
ContactsAddToGroup { group_id, contact_id }
ContactsRemoveFromGroup { group_id, contact_id }
```

### Calendar — ClientRpcRequest

```
CalendarCreate { name, color, timezone, description }
CalendarDelete { calendar_id }
CalendarList { limit }
CalendarGetEvent { event_id }
CalendarCreateEvent { calendar_id, ical_data }     // accepts raw iCal string
CalendarUpdateEvent { event_id, ical_data }
CalendarDeleteEvent { event_id }
CalendarListEvents { calendar_id, start_ms, end_ms, limit }  // time range query
CalendarSearchEvents { query, calendar_id, limit }
CalendarImportIcal { calendar_id, ical_data }      // bulk import
CalendarExportIcal { calendar_id }                 // bulk export
CalendarFreeBusy { calendar_id, start_ms, end_ms } // free/busy query
CalendarExpandRecurrence { event_id, start_ms, end_ms, max_instances }
```

### Response Types

```rust
// Contacts
ContactsBookResult { book_id, name, success, error }
ContactsResult { contact_id, vcard_data, success, error }     // vcard_data for export
ContactsListResult { contacts: Vec<ContactSummary>, continuation_token, total }
ContactsSearchResult { contacts: Vec<ContactSummary>, total }
ContactsGroupResult { group_id, success, error }
ContactsExportResult { vcard_data, count }

// Calendar
CalendarResult { calendar_id, name, success, error }
CalendarEventResult { event_id, ical_data, success, error }
CalendarListEventsResult { events: Vec<EventSummary>, continuation_token, total }
CalendarSearchResult { events: Vec<EventSummary>, total }
CalendarFreeBusyResult { busy_periods: Vec<BusyPeriod>, success, error }
CalendarExpandResult { instances: Vec<EventInstance>, total }
CalendarExportResult { ical_data, count }
```

## vCard & iCal Parsing Strategy

### vCard (RFC 6350)

Hand-rolled parser in `aspen-contacts`. vCard is a simple line-oriented format:

```
BEGIN:VCARD
VERSION:4.0
FN:John Doe
N:Doe;John;;;
EMAIL;TYPE=work:john@example.com
TEL;TYPE=cell:+1-555-0123
BDAY:1990-01-15
END:VCARD
```

Parsing rules:

- Line unfolding (lines starting with space/tab are continuations)
- Property parsing: `NAME;PARAM=VALUE:CONTENT`
- Structured values (`;`-separated in N, ADR)
- Multi-valued properties (`,`-separated in CATEGORIES)
- No external crate needed — ~200 lines of parser code

### iCalendar (RFC 5545)

Similar line-oriented format but more complex due to RRULE:

```
BEGIN:VCALENDAR
BEGIN:VEVENT
UID:event-001@example.com
DTSTART:20260315T100000Z
DTEND:20260315T110000Z
SUMMARY:Team Meeting
RRULE:FREQ=WEEKLY;BYDAY=MO,WE,FR;UNTIL=20260601T000000Z
VALARM:
TRIGGER:-PT15M
ACTION:DISPLAY
END:VALARM
END:VEVENT
END:VCALENDAR
```

RRULE expansion is the only complex part. Support these recurrence types:

- `FREQ`: DAILY, WEEKLY, MONTHLY, YEARLY
- `INTERVAL`: every N periods
- `BYDAY`: day-of-week filters (MO, TU, WE, etc.)
- `BYMONTHDAY`: day-of-month filters
- `UNTIL` / `COUNT`: termination
- `EXDATE`: excluded dates

Bounded expansion: always require `max_instances` parameter (default 100, max 1000).

## Timer Integration (Reminders)

When an event with alarms is created:

1. Compute trigger time: `dtstart_ms - (trigger_minutes_before * 60 * 1000)`
2. If trigger time is in the future, call `schedule_timer_on_host()` with name `calendar:alarm:{event_id}:{alarm_id}`
3. On timer fire (`plugin_on_timer`), emit a hook event `hooks.calendar.reminder` with event details
4. Max 16 timers per plugin — only schedule the next N upcoming reminders, re-evaluate periodically

For native handlers (not WASM), timers are managed via a background task that scans `calendar:reminder:*` keys periodically.

## Handler Registration

Both handlers use `submit_handler_factory!` for auto-registration:

```rust
// aspen-contacts-handler/src/lib.rs
use aspen_rpc_core::submit_handler_factory;

submit_handler_factory!(ContactsHandlerFactory);

pub struct ContactsHandlerFactory;

impl HandlerFactory for ContactsHandlerFactory {
    fn name(&self) -> &'static str { "ContactsHandler" }
    fn priority(&self) -> u32 { 350 }  // application layer
    fn app_id(&self) -> Option<&'static str> { Some("contacts") }

    fn create(&self, ctx: &ClientProtocolContext) -> Option<Arc<dyn RequestHandler>> {
        // Always available — uses core KV store
        Some(Arc::new(ContactsHandler::new(ctx.kv_store.clone())))
    }
}
```

## Feature Flags

```toml
# aspen-rpc-handlers/Cargo.toml
[features]
contacts = ["dep:aspen-contacts-handler"]
calendar = ["dep:aspen-calendar-handler"]
```

Both are opt-in. The node binary includes them via `--features contacts,calendar`.

## CLI Commands

```bash
# Contacts
aspen-cli contacts create-book "Personal"
aspen-cli contacts add --book <id> --name "John Doe" --email john@example.com --phone +1-555-0123
aspen-cli contacts list --book <id>
aspen-cli contacts search "john"
aspen-cli contacts get <contact-id>
aspen-cli contacts import --book <id> contacts.vcf
aspen-cli contacts export --book <id> > contacts.vcf
aspen-cli contacts delete <contact-id>

# Calendar
aspen-cli calendar create "Work" --color "#4285f4" --timezone "America/New_York"
aspen-cli calendar add-event --calendar <id> --summary "Meeting" --start "2026-03-15T10:00" --end "2026-03-15T11:00"
aspen-cli calendar list-events --calendar <id> --start "2026-03-01" --end "2026-03-31"
aspen-cli calendar search "meeting"
aspen-cli calendar get-event <event-id>
aspen-cli calendar import --calendar <id> events.ics
aspen-cli calendar export --calendar <id> > events.ics
aspen-cli calendar free-busy --calendar <id> --start "2026-03-15" --end "2026-03-16"
aspen-cli calendar delete-event <event-id>
```

## Testing Strategy

1. **Unit tests** in domain crates: vCard parsing/serialization roundtrips, iCal parsing, RRULE expansion edge cases, data model validation
2. **Handler tests** in handler crates: `can_handle()` coverage, mock KV responses
3. **Integration tests**: End-to-end via `TestContextBuilder` — create book, add contacts, search, export
4. **Wire format tests**: Postcard discriminant stability for new variants
5. **Property tests**: Proptest for vCard roundtrip (parse → serialize → parse = identity)

## Future: CalDAV/CardDAV Bridge

Not in this change, but the data model is designed to support it later:

- A thin HTTP/WebDAV proxy (separate binary, like `git-remote-aspen`) that translates CalDAV/CardDAV requests into Aspen RPC calls
- Uses `uid` field for CalDAV `REPORT` queries
- `revision` enables `If-Match` / `If-None-Match` ETags
- `sequence` number tracks iCalendar SEQUENCE for proper client sync
