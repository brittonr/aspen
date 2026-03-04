# Proposal: WASM Calendar & Contacts Plugins

## What

Migrate `aspen-calendar` and `aspen-contacts` from native `ServiceExecutor` crates to WASM plugins in the `aspen-plugins` repo, matching the established plugin pattern (coordination, secrets, forge, etc.).

## Why

Aspen's architecture mandates WASM plugins for all application-layer services. Calendar and contacts are currently the only remaining application services implemented as native `ServiceExecutor` crates. They need to be WASM plugins for:

- **Architectural consistency** — every other application service is already a WASM plugin
- **Hot-reload** — update calendar/contacts logic without restarting cluster nodes
- **Isolation** — KV namespace sandboxing and permission enforcement via plugin manifest
- **Reduced node binary size** — pull ~3,200 lines of domain logic out of the compiled node

## Scope

**In scope:**

- Create `aspen-calendar-plugin` WASM crate in `aspen-plugins` repo
- Create `aspen-contacts-plugin` WASM crate in `aspen-plugins` repo
- Port iCal parsing/serialization and vCard parsing/serialization to WASM-compatible code
- Port CalendarStore and ContactStore KV operations to use guest SDK host functions
- Remove native `CalendarServiceExecutor` and `ContactsServiceExecutor` from `aspen-rpc-handlers`
- Remove `aspen-calendar-handler` and `aspen-contacts-handler` crates from workspace
- Add smoke tests for both plugins
- Add plugin.json manifests

**Out of scope:**

- `aspen-sops` (CLI binary with heavy crypto — separate effort)
- Changes to `ClientRpcRequest`/`ClientRpcResponse` enum variants (they stay in `aspen-client-api`)
- Changes to `aspen-calendar` or `aspen-contacts` core library crates (parsing logic stays)

## Key Decisions

- **iCal/vCard parsing compiled into WASM** — the parsing libraries are pure Rust with no FFI, so they compile to wasm32 cleanly
- **KV operations via guest SDK** — replace `Arc<dyn KeyValueStore>` with `kv_get_value`, `kv_put_value`, `kv_scan_prefix`, `kv_delete_key` host calls
- **JSON encoding for KV values** — matches existing pattern (coordination plugin stores JSON)
- **KV prefixes** — `calendar:` and `contacts:` (matching current prefixes)
- **Priority 940** — between coordination (930) and core infrastructure (<500)
