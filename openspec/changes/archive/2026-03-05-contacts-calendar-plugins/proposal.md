## Why

Aspen has all the distributed primitives to be a personal data platform — KV store, blob storage, CRDT sync, hooks, timers, secondary indexes — but currently no user-facing applications are built on them. Contacts and Calendar are the foundational PIM (Personal Information Management) apps. They're small, well-specified (vCard RFC 6350, iCalendar RFC 5545), and exercise the full plugin stack: KV for structured data, secondary indexes for queries, timers for reminders, hooks for event notifications, and blobs for photo storage.

Building these as native handler crates (not WASM plugins) provides:

- A reference architecture for other PIM apps (tasks, notes, bookmarks)
- CalDAV/CardDAV protocol compatibility for syncing with existing clients (iOS, Android, Thunderbird, macOS)
- Proof that Aspen's primitives can replace Nextcloud/Radicale for self-hosted PIM

## What Changes

- **New crate `aspen-contacts`**: Domain logic for contact management — vCard parsing/serialization, contact CRUD, group management, birthday extraction, photo storage (blobs), search by name/email/phone via secondary indexes.

- **New crate `aspen-contacts-handler`**: RPC handler mapping `ClientRpcRequest::Contacts*` variants to `aspen-contacts` operations. Follows the extracted handler crate pattern (like `aspen-forge-handler`).

- **New crate `aspen-calendar`**: Domain logic for calendar/event management — iCalendar parsing/serialization, event CRUD, recurring event expansion (RRULE), alarm/reminder scheduling via timers, free/busy queries.

- **New crate `aspen-calendar-handler`**: RPC handler mapping `ClientRpcRequest::Calendar*` variants to `aspen-calendar` operations.

- **New `ClientRpcRequest` variants** in `aspen-client-api`: ~15 Contacts variants + ~15 Calendar variants added before the feature-gated section.

- **New `ClientRpcResponse` variants** in `aspen-client-api`: Corresponding response types.

- **Feature flags**: `contacts` and `calendar` features in `aspen-rpc-handlers`, wired through to the node binary.

- **CLI commands**: `aspen-cli contacts` and `aspen-cli calendar` subcommands for basic operations.

## Capabilities

### New Capabilities

**Contacts**:

- Create, read, update, delete contacts with vCard fields (name, email, phone, address, org, birthday, notes, photo)
- Contact groups/lists
- Search by name, email, phone via secondary indexes
- Import/export vCard format
- Birthday extraction for reminders
- Photo storage via blob store

**Calendar**:

- Create, read, update, delete calendars
- Create, read, update, delete events with iCalendar fields (summary, dtstart, dtend, location, description, attendees)
- Recurring event support (RRULE parsing + expansion)
- Event reminders via timer system
- Time-range queries (events between start/end)
- Free/busy queries
- Import/export iCalendar format

### Modified Capabilities

- `ClientRpcRequest` / `ClientRpcResponse` enums gain ~30 new variants (non-feature-gated, placed before the gated section)
- Handler registry gains two new handler factories
- CLI gains two new subcommand groups

## Impact

- **New files**: 4 new crate directories (~8-12 source files each), CLI command files, response types
- **Modified files**: `aspen-client-api/src/messages/mod.rs` (new variants), `Cargo.toml` (workspace members), `aspen-rpc-handlers/Cargo.toml` (feature flags), CLI main
- **Dependencies**: `icalendar` crate (iCal parsing, no_std compatible), vCard parsing (hand-rolled or `vcard4` crate)
- **Risk**: Low — purely additive, no changes to existing handlers or storage
- **Wire format**: New enum variants must be placed correctly for postcard discriminant stability
