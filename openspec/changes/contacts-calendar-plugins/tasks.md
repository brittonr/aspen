## 1. Response Types (`aspen-client-api`)

- [x] 1.1 Create `crates/aspen-client-api/src/messages/contacts.rs` with Contact response types: `ContactsBookResponse`, `ContactsResponse`, `ContactsListResponse`, `ContactsSearchResponse`, `ContactsGroupResponse`, `ContactsExportResponse`, `ContactSummary`
- [x] 1.2 Create `crates/aspen-client-api/src/messages/calendar.rs` with Calendar response types: `CalendarResponse`, `CalendarEventResponse`, `CalendarListEventsResponse`, `CalendarSearchResponse`, `CalendarFreeBusyResponse`, `CalendarExpandResponse`, `CalendarExportResponse`, `EventSummary`, `EventInstance`, `BusyPeriod`
- [x] 1.3 Add `pub mod contacts;` and `pub mod calendar;` to `messages/mod.rs`, add `pub use` re-exports
- [x] 1.4 Add Contacts `ClientRpcRequest` variants (16 total: `ContactsCreateBook` through `ContactsRemoveFromGroup`) — place BEFORE the feature-gated section
- [x] 1.5 Add Calendar `ClientRpcRequest` variants (14 total: `CalendarCreate` through `CalendarExpandRecurrence`) — place BEFORE the feature-gated section
- [x] 1.6 Add Contacts `ClientRpcResponse` variants (6 total: `ContactsBookResult` through `ContactsExportResult`) — place BEFORE the feature-gated section
- [x] 1.7 Add Calendar `ClientRpcResponse` variants (7 total: `CalendarResult` through `CalendarExportResult`) — place BEFORE the feature-gated section
- [x] 1.8 Add `variant_name()` arms for all new request variants
- [x] 1.9 Add `domain()` return `Some("contacts")` / `Some("calendar")` for new variants
- [x] 1.10 Add `to_operation()` mappings for new variants in a new `contacts_ops.rs` and `calendar_ops.rs`
- [x] 1.11 Add postcard discriminant stability tests for critical new variants
- [x] 1.12 Verify `cargo nextest run -p aspen-client-api` passes

## 2. Contacts Domain Crate (`aspen-contacts`)

- [x] 2.1 Create `crates/aspen-contacts/Cargo.toml` with deps: `serde`, `serde_json`, `snafu`, `tracing`, `aspen-core`, `aspen-client-api`
- [x] 2.2 Create data model types in `src/types.rs`: `Contact`, `ContactEmail`, `ContactPhone`, `ContactAddress`, `ContactBook`, `ContactGroup`
- [x] 2.3 Create vCard parser in `src/vcard/parse.rs`: line unfolding, property parsing, structured N/ADR fields, multi-value CATEGORIES
- [x] 2.4 Create vCard serializer in `src/vcard/serialize.rs`: `Contact → String` vCard 4.0 output
- [x] 2.5 Create `src/vcard/mod.rs` with `pub use` re-exports and roundtrip unit tests
- [ ] 2.6 Create `src/store.rs` with `ContactStore` struct: CRUD operations via `KeyValueStore` trait, KV key construction, JSON serialization
- [ ] 2.7 Implement `create_book`, `delete_book`, `list_books` on `ContactStore`
- [ ] 2.8 Implement `create_contact` (parse vCard → Contact struct → JSON → KV write), `get_contact`, `update_contact`, `delete_contact`
- [ ] 2.9 Implement `list_contacts` with prefix scan + pagination
- [x] 2.10 Implement `search_contacts` using KV prefix scan with client-side filtering (name, email, phone substring match)
- [x] 2.11 Implement `import_vcard` (parse multi-entry vCard, batch write), `export_vcard` (scan + serialize all)
- [x] 2.12 Implement `create_group`, `delete_group`, `list_groups`, `add_to_group`, `remove_from_group`
- [x] 2.13 Create `src/error.rs` with `ContactsError` enum (snafu): `ParseVcard`, `NotFound`, `InvalidInput`, `StorageError`
- [x] 2.14 Create `src/lib.rs` with module structure and re-exports
- [x] 2.15 Add to workspace `Cargo.toml` members
- [x] 2.16 Write unit tests for vCard parsing (simple contact, full contact, multi-value, structured fields, UTF-8)
- [x] 2.17 Write unit tests for vCard roundtrip (parse → serialize → parse = same)
- [x] 2.18 Write unit tests for `ContactStore` operations (mock KV store from `aspen-testing`)
- [x] 2.19 Verify `cargo nextest run -p aspen-contacts` passes

## 3. Calendar Domain Crate (`aspen-calendar`)

- [x] 3.1 Create `crates/aspen-calendar/Cargo.toml` with deps: `serde`, `serde_json`, `snafu`, `tracing`, `aspen-core`, `aspen-client-api`
- [x] 3.2 Create data model types in `src/types.rs`: `CalendarMeta`, `CalendarEvent`, `EventAttendee`, `EventAlarm`, `EventStatus`, `AttendeeRole`, `AttendeeStatus`, `AlarmAction`
- [x] 3.3 Create iCal parser in `src/ical/parse.rs`: line unfolding, VCALENDAR/VEVENT parsing, DTSTART/DTEND date-time parsing (DATE and DATE-TIME formats), VALARM parsing
- [x] 3.4 Create iCal serializer in `src/ical/serialize.rs`: `CalendarEvent → String` iCalendar output
- [x] 3.5 Create RRULE expander in `src/ical/rrule.rs`: parse RRULE string, expand with FREQ (DAILY/WEEKLY/MONTHLY/YEARLY), INTERVAL, BYDAY, BYMONTHDAY, COUNT, UNTIL, EXDATE filtering. Bounded by `max_instances`.
- [x] 3.6 Create `src/ical/mod.rs` with re-exports and roundtrip unit tests
- [ ] 3.7 Create `src/store.rs` with `CalendarStore` struct: CRUD via `KeyValueStore` trait
- [ ] 3.8 Implement `create_calendar`, `delete_calendar`, `list_calendars`
- [ ] 3.9 Implement `create_event` (parse iCal → CalendarEvent → JSON → KV write), `get_event`, `update_event`, `delete_event`
- [x] 3.10 Implement `list_events` with time-range filtering: scan `calendar:event:{cal_id}:*`, filter by `dtstart_ms`/`dtend_ms` range, support pagination
- [x] 3.11 Implement `search_events` using prefix scan + summary/description substring match
- [x] 3.12 Implement `import_ical` (parse multi-event iCal, batch write), `export_ical` (scan + serialize all)
- [x] 3.13 Implement `free_busy_query`: scan events in range, return busy periods (start_ms, end_ms pairs)
- [x] 3.14 Implement `expand_recurrence`: parse event's RRULE, generate instances within range, apply EXDATE exclusions
- [x] 3.15 Create `src/error.rs` with `CalendarError` enum (snafu): `ParseIcal`, `NotFound`, `InvalidInput`, `StorageError`, `InvalidRrule`
- [x] 3.16 Create `src/lib.rs` with module structure and re-exports
- [x] 3.17 Add to workspace `Cargo.toml` members
- [x] 3.18 Write unit tests for iCal parsing (simple event, all-day event, with attendees, with alarms, multi-event)
- [x] 3.19 Write unit tests for iCal roundtrip
- [x] 3.20 Write RRULE expansion tests: daily, weekly with BYDAY, monthly with BYMONTHDAY, yearly, COUNT limit, UNTIL limit, EXDATE exclusion, INTERVAL
- [x] 3.21 Write unit tests for `CalendarStore` operations
- [x] 3.22 Verify `cargo nextest run -p aspen-calendar` passes

## 4. Contacts Handler Crate (`aspen-contacts-handler`)

- [ ] 4.1 Create `crates/aspen-contacts-handler/Cargo.toml` with deps: `aspen-rpc-core`, `aspen-contacts`, `aspen-client-api`, `async-trait`, `anyhow`, `tracing`
- [ ] 4.2 Create `src/handler.rs` with `ContactsHandler` implementing `RequestHandler`: `can_handle()` for all 16 Contacts variants, `handle()` dispatch
- [ ] 4.3 Create `src/lib.rs` with `ContactsHandlerFactory` implementing `HandlerFactory` (priority 350, app_id "contacts"), `submit_handler_factory!`
- [ ] 4.4 Implement handler functions for book operations: `handle_create_book`, `handle_delete_book`, `handle_list_books`
- [ ] 4.5 Implement handler functions for contact CRUD: `handle_create_contact`, `handle_get_contact`, `handle_update_contact`, `handle_delete_contact`
- [ ] 4.6 Implement handler functions for list/search: `handle_list_contacts`, `handle_search_contacts`
- [ ] 4.7 Implement handler functions for import/export: `handle_import_vcard`, `handle_export_vcard`
- [ ] 4.8 Implement handler functions for groups: `handle_create_group`, `handle_delete_group`, `handle_list_groups`, `handle_add_to_group`, `handle_remove_from_group`
- [ ] 4.9 Add to workspace `Cargo.toml` members
- [ ] 4.10 Write `can_handle` unit tests
- [ ] 4.11 Verify `cargo nextest run -p aspen-contacts-handler` passes

## 5. Calendar Handler Crate (`aspen-calendar-handler`)

- [ ] 5.1 Create `crates/aspen-calendar-handler/Cargo.toml` with deps: `aspen-rpc-core`, `aspen-calendar`, `aspen-client-api`, `async-trait`, `anyhow`, `tracing`
- [ ] 5.2 Create `src/handler.rs` with `CalendarHandler` implementing `RequestHandler`: `can_handle()` for all 14 Calendar variants, `handle()` dispatch
- [ ] 5.3 Create `src/lib.rs` with `CalendarHandlerFactory` implementing `HandlerFactory` (priority 360, app_id "calendar"), `submit_handler_factory!`
- [ ] 5.4 Implement handler functions for calendar CRUD: `handle_create_calendar`, `handle_delete_calendar`, `handle_list_calendars`
- [ ] 5.5 Implement handler functions for event CRUD: `handle_create_event`, `handle_get_event`, `handle_update_event`, `handle_delete_event`
- [ ] 5.6 Implement handler functions for queries: `handle_list_events`, `handle_search_events`, `handle_free_busy`, `handle_expand_recurrence`
- [ ] 5.7 Implement handler functions for import/export: `handle_import_ical`, `handle_export_ical`
- [ ] 5.8 Add to workspace `Cargo.toml` members
- [ ] 5.9 Write `can_handle` unit tests
- [ ] 5.10 Verify `cargo nextest run -p aspen-calendar-handler` passes

## 6. Wire Up Feature Flags & Registry

- [ ] 6.1 Add `contacts` and `calendar` features to `aspen-rpc-handlers/Cargo.toml` with optional deps on handler crates
- [ ] 6.2 Add `contacts` and `calendar` features to root `Cargo.toml` (node binary) feature list
- [ ] 6.3 Verify `cargo build --features contacts,calendar` compiles
- [ ] 6.4 Verify handlers auto-register via `submit_handler_factory!` (test with `cargo run --features contacts,calendar --bin aspen-node`)

## 7. CLI Commands

- [ ] 7.1 Create `crates/aspen-cli/src/commands/contacts.rs` with subcommands: `create-book`, `list-books`, `add`, `get`, `list`, `search`, `import`, `export`, `delete`, `create-group`, `list-groups`
- [ ] 7.2 Create `crates/aspen-cli/src/commands/calendar.rs` with subcommands: `create`, `list`, `add-event`, `get-event`, `list-events`, `search`, `import`, `export`, `free-busy`, `delete-event`
- [ ] 7.3 Wire into CLI main command enum
- [ ] 7.4 Verify `cargo build -p aspen-cli` compiles
- [ ] 7.5 Write CLI parse tests for new subcommands

## 8. Integration Tests

- [ ] 8.1 Write end-to-end contacts test: create book → add contacts (vCard) → list → search by email → export vCard → verify roundtrip
- [ ] 8.2 Write end-to-end calendar test: create calendar → add events (iCal) → list by time range → free/busy query → export iCal → verify roundtrip
- [ ] 8.3 Write RRULE integration test: create recurring event → expand recurrence → verify instance count and dates
- [ ] 8.4 Write contact groups test: create group → add members → list group → remove member
- [ ] 8.5 Verify `cargo nextest run --workspace --features contacts,calendar` passes with no regressions
