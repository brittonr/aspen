# Tasks: WASM Calendar & Contacts Plugins

## Phase 1: Calendar Plugin (aspen-plugins repo)

### Task 1.1: Scaffold aspen-calendar-plugin crate ✅

**Repo**: `~/git/aspen-plugins`

Create the crate structure:

- `crates/aspen-calendar-plugin/Cargo.toml` — cdylib, `test = false`, `doctest = false`
- `crates/aspen-calendar-plugin/plugin.json` — manifest with 13 handles, `calendar:` kv prefix, priority 940
- `crates/aspen-calendar-plugin/src/lib.rs` — `AspenPlugin` impl with `info()` and `handle()` dispatch for all 13 calendar operations
- `crates/aspen-calendar-plugin/src/kv.rs` — KV helpers (`get_json`, `put_json`, `delete`, `scan`) using guest SDK host functions (copy pattern from coordination-plugin)
- `crates/aspen-calendar-plugin/src/types.rs` — `CalendarMeta`, `CalendarEvent` structs (port from `aspen-calendar/src/types.rs`)
- `crates/aspen-calendar-plugin/tests/smoke.rs` — minimal test
- Add to workspace members in root `Cargo.toml`

### Task 1.2: Port iCal parsing to calendar plugin ✅

**Repo**: `~/git/aspen-plugins`

Port the iCal parsing/serialization code:

- `src/ical/mod.rs` — module re-exports
- `src/ical/parse.rs` — port from `aspen-calendar/src/ical/parse.rs` (537 lines). Remove `tracing` calls. Replace `KeyValueStore` trait usage with direct host KV calls.
- `src/ical/serialize.rs` — port from `aspen-calendar/src/ical/serialize.rs` (219 lines)
- `src/ical/rrule.rs` — port from `aspen-calendar/src/ical/rrule.rs` (450 lines). Pure computation, no changes needed.

These are pure Rust parsers with no FFI — they compile to wasm32 unchanged aside from removing `tracing`.

### Task 1.3: Port calendar store operations ✅

**Repo**: `~/git/aspen-plugins`

Create `src/store.rs` with all 13 operations, porting from `aspen-calendar/src/store.rs` (835 lines) and `aspen-calendar-handler/src/executor.rs` (432 lines):

Each operation becomes a function that:

1. Takes the request parameters directly (not `ClientRpcRequest`)
2. Uses `kv.rs` helpers for KV access
3. Returns `ClientRpcResponse`

Operations:

- `create_calendar(name, description, timezone)` → `CalendarResult`
- `delete_calendar(calendar_id)` → `CalendarResult`
- `list_calendars(owner)` → `CalendarListResult`
- `get_event(calendar_id, event_id)` → `CalendarEventResult`
- `create_event(calendar_id, ical_data)` → `CalendarEventResult`
- `update_event(calendar_id, event_id, ical_data)` → `CalendarEventResult`
- `delete_event(calendar_id, event_id)` → `CalendarResult`
- `list_events(calendar_id, from_ms, to_ms, limit)` → `CalendarListEventsResult`
- `search_events(calendar_id, query, limit)` → `CalendarSearchResult`
- `import_ical(calendar_id, ical_data)` → `CalendarResult`
- `export_ical(calendar_id)` → `CalendarExportResult`
- `free_busy(calendar_id, from_ms, to_ms)` → `CalendarFreeBusyResult`
- `expand_recurrence(calendar_id, event_id, from_ms, to_ms, max_instances)` → `CalendarExpandResult`

Key difference from native: Replace `self.store.method()` async calls with synchronous `kv::get_json()` / `kv::put_json()` / `kv::scan()` calls.

### Task 1.4: Verify calendar plugin compiles to wasm32 ✅

**Repo**: `~/git/aspen-plugins`

- `cargo build -p aspen-calendar-plugin --target wasm32-unknown-unknown`
- Fix any compilation issues (tracing, async, std dependencies)
- Verify `plugin_info_matches_manifest` test passes
- Verify the resulting `.wasm` file is reasonable size

## Phase 2: Contacts Plugin (aspen-plugins repo)

### Task 2.1: Scaffold aspen-contacts-plugin crate ✅

**Repo**: `~/git/aspen-plugins`

Same pattern as calendar:

- `crates/aspen-contacts-plugin/Cargo.toml`
- `crates/aspen-contacts-plugin/plugin.json` — manifest with 16 handles, `contacts:` kv prefix, priority 940
- `crates/aspen-contacts-plugin/src/lib.rs` — `AspenPlugin` impl dispatching 16 contact operations
- `crates/aspen-contacts-plugin/src/kv.rs` — KV helpers (identical to calendar's)
- `crates/aspen-contacts-plugin/src/types.rs` — `Contact`, `ContactBook`, `ContactGroup` structs
- `crates/aspen-contacts-plugin/tests/smoke.rs`
- Add to workspace members

### Task 2.2: Port vCard parsing to contacts plugin ✅

**Repo**: `~/git/aspen-plugins`

- `src/vcard/mod.rs`
- `src/vcard/parse.rs` — port from `aspen-contacts/src/vcard/parse.rs` (397 lines)
- `src/vcard/serialize.rs` — port from `aspen-contacts/src/vcard/serialize.rs` (139 lines)

Pure Rust parsers, no changes needed except removing `tracing`.

### Task 2.3: Port contacts store operations ✅

**Repo**: `~/git/aspen-plugins`

Create `src/store.rs` with all 16 operations, porting from `aspen-contacts/src/store.rs` (875 lines) and `aspen-contacts-handler/src/executor.rs` (472 lines):

Operations:

- `create_book(name, description)` → `ContactsBookResult`
- `delete_book(book_id)` → `ContactsBookResult`
- `list_books(owner)` → `ContactsBookListResult`
- `create_contact(book_id, vcard_data)` → `ContactsResult`
- `get_contact(contact_id)` → `ContactsResult`
- `update_contact(contact_id, vcard_data)` → `ContactsResult`
- `delete_contact(contact_id)` → `ContactsResult`
- `list_contacts(book_id, limit)` → `ContactsListResult`
- `search_contacts(book_id, query, limit)` → `ContactsSearchResult`
- `import_vcard(book_id, vcard_data)` → `ContactsResult`
- `export_vcard(book_id)` → `ContactsExportResult`
- `create_group(book_id, name)` → `ContactsGroupResult`
- `delete_group(book_id, group_id)` → `ContactsGroupResult`
- `list_groups(book_id)` → `ContactsGroupListResult`
- `add_to_group(book_id, group_id, contact_id)` → `ContactsGroupResult`
- `remove_from_group(book_id, group_id, contact_id)` → `ContactsGroupResult`

### Task 2.4: Verify contacts plugin compiles to wasm32 ✅

**Repo**: `~/git/aspen-plugins`

Same as Task 1.4 but for contacts.

## Phase 3: Remove Native Handlers (aspen main repo)

### Task 3.1: Remove native calendar handler ✅

**Repo**: `~/git/aspen`

- Remove `crates/aspen-calendar-handler/` directory
- Remove from workspace members in root `Cargo.toml`
- Remove `CalendarServiceExecutor` import and registration from `aspen-rpc-handlers`
- Remove `calendar` feature flag references from `aspen-rpc-handlers/Cargo.toml`
- `cargo build` to verify clean compilation

### Task 3.2: Remove native contacts handler ✅

**Repo**: `~/git/aspen`

- Remove `crates/aspen-contacts-handler/` directory
- Remove from workspace members in root `Cargo.toml`
- Remove `ContactsServiceExecutor` import and registration from `aspen-rpc-handlers`
- Remove `contacts` feature flag references from `aspen-rpc-handlers/Cargo.toml`
- `cargo build` to verify clean compilation

### Task 3.3: Build and test ✅

**Repos**: both

- Build both WASM plugins: `cargo build --target wasm32-unknown-unknown -p aspen-calendar-plugin -p aspen-contacts-plugin`
- Build aspen node without calendar/contacts handlers: `cargo build`
- Run existing calendar integration tests: `cargo nextest run -p aspen-calendar`
- Run existing contacts integration tests: `cargo nextest run -p aspen-contacts`
- Run `cargo nextest run --workspace` in aspen-plugins to catch any breakage
