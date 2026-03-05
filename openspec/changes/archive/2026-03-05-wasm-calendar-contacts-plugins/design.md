# Design: WASM Calendar & Contacts Plugins

## Architecture

Both plugins follow the exact same pattern as `aspen-coordination-plugin`:

```
aspen-plugins/crates/
├── aspen-calendar-plugin/
│   ├── Cargo.toml          # cdylib, no tests
│   ├── plugin.json         # manifest
│   ├── src/
│   │   ├── lib.rs          # AspenPlugin impl + dispatch
│   │   ├── kv.rs           # KV helpers (reuse coordination pattern)
│   │   ├── store.rs        # Calendar CRUD via host KV calls
│   │   ├── ical/
│   │   │   ├── mod.rs
│   │   │   ├── parse.rs    # iCal parsing (ported from aspen-calendar)
│   │   │   ├── serialize.rs
│   │   │   └── rrule.rs    # Recurrence expansion
│   │   └── types.rs        # CalendarMeta, CalendarEvent structs
│   └── tests/
│       └── smoke.rs        # Integration test stub
│
├── aspen-contacts-plugin/
│   ├── Cargo.toml
│   ├── plugin.json
│   ├── src/
│   │   ├── lib.rs          # AspenPlugin impl + dispatch
│   │   ├── kv.rs           # KV helpers
│   │   ├── store.rs        # Contact CRUD via host KV calls
│   │   ├── vcard/
│   │   │   ├── mod.rs
│   │   │   ├── parse.rs    # vCard parsing (ported from aspen-contacts)
│   │   │   └── serialize.rs
│   │   └── types.rs        # Contact, ContactBook, ContactGroup structs
│   └── tests/
│       └── smoke.rs
```

## Plugin Dispatch Pattern

Each plugin implements `AspenPlugin` with:

1. **`info()`** — returns `PluginInfo` matching `plugin.json`
2. **`handle()`** — matches `ClientRpcRequest` variants → calls store functions → returns `ClientRpcResponse`

```rust
// Calendar plugin dispatch (13 operations)
impl AspenPlugin for CalendarPlugin {
    fn handle(request: ClientRpcRequest) -> ClientRpcResponse {
        match request {
            ClientRpcRequest::CalendarCreate { name, description, timezone } =>
                store::create_calendar(name, description, timezone),
            ClientRpcRequest::CalendarDelete { calendar_id } =>
                store::delete_calendar(calendar_id),
            // ... 11 more variants
            _ => error_response("UNHANDLED_REQUEST", "..."),
        }
    }
}
```

## KV Translation

The native `CalendarStore` uses `Arc<dyn KeyValueStore>` with `ReadRequest`/`WriteRequest`/`ScanRequest` types. The WASM plugin uses guest SDK host functions directly:

| Native (CalendarStore)              | WASM Plugin (kv.rs)               |
|-------------------------------------|-----------------------------------|
| `kv.read(ReadRequest::key(k))`      | `kv_get_value(k)`                 |
| `kv.write(WriteRequest::set(k,v))`  | `kv_put_value(k, v.as_bytes())`   |
| `kv.delete(DeleteRequest::key(k))`  | `kv_delete_key(k)`                |
| `kv.scan(ScanRequest{prefix,...})`  | `kv_scan_prefix(prefix, limit)`   |

The `kv.rs` helper module follows coordination-plugin's pattern: `get_json`, `put_json`, `delete`, `scan`.

## KV Key Layout (unchanged)

Calendar:

- `calendar:cal:{calendar_id}` → `CalendarMeta` (JSON)
- `calendar:event:{calendar_id}:{event_id}` → `CalendarEvent` (JSON)

Contacts:

- `contacts:book:{book_id}` → `ContactBook` (JSON)
- `contacts:entry:{book_id}:{contact_id}` → `Contact` (JSON)
- `contacts:group:{book_id}:{group_id}` → `ContactGroup` (JSON)

## Plugin Manifests

### Calendar (plugin.json)

```json
{
  "name": "calendar",
  "version": "0.1.0",
  "handles": [
    "CalendarCreate", "CalendarDelete", "CalendarList",
    "CalendarGetEvent", "CalendarCreateEvent", "CalendarUpdateEvent",
    "CalendarDeleteEvent", "CalendarListEvents", "CalendarSearchEvents",
    "CalendarImportIcal", "CalendarExportIcal",
    "CalendarFreeBusy", "CalendarExpandRecurrence"
  ],
  "priority": 940,
  "app_id": "calendar",
  "kv_prefixes": ["calendar:"],
  "permissions": { "kv_read": true, "kv_write": true, "timers": true }
}
```

### Contacts (plugin.json)

```json
{
  "name": "contacts",
  "version": "0.1.0",
  "handles": [
    "ContactsCreateBook", "ContactsDeleteBook", "ContactsListBooks",
    "ContactsCreateContact", "ContactsGetContact", "ContactsUpdateContact",
    "ContactsDeleteContact", "ContactsListContacts", "ContactsSearchContacts",
    "ContactsImportVcard", "ContactsExportVcard",
    "ContactsCreateGroup", "ContactsDeleteGroup", "ContactsListGroups",
    "ContactsAddToGroup", "ContactsRemoveFromGroup"
  ],
  "priority": 940,
  "app_id": "contacts",
  "kv_prefixes": ["contacts:"],
  "permissions": { "kv_read": true, "kv_write": true }
}
```

## What Changes in aspen (main repo)

1. **Remove** `aspen-calendar-handler` and `aspen-contacts-handler` crates
2. **Remove** their `ServiceExecutor` registrations from `aspen-rpc-handlers`
3. **Remove** `calendar` and `contacts` feature flags from `aspen-rpc-handlers`
4. **Keep** `aspen-calendar` and `aspen-contacts` core crates (they still hold types and parsing logic used by tests)
5. **Keep** all `ClientRpcRequest`/`ClientRpcResponse` variants in `aspen-client-api` (WASM plugins reference them via `aspen-client-api`)

## What Doesn't Change

- Client API enum variants — untouched
- CLI commands — untouched (they send the same RPCs)
- KV key layout — identical prefixes
- Wire format — same postcard serialization
- Core parsing libraries — stay in `aspen-calendar`/`aspen-contacts` (can be used as deps by the WASM plugin crates if they compile to wasm32, otherwise we inline the parsing code)

## Risk: iCal/vCard Dependencies

The parsing code in `aspen-calendar` and `aspen-contacts` is hand-written (no external crate deps beyond serde). It compiles to wasm32 without issues. The main risk is `tracing` calls in the store layer — WASM plugins can't use `tracing`, so those become no-ops or are removed.
