# WASM Plugin Hardening Plan

> **Status: ✅ COMPLETE** — All five items implemented as of 2026-02-19.

Five targeted improvements to the WASM plugin host: dispatch performance,
resource limits, namespace isolation, error encoding consistency, and ABI
documentation.

---

## 1. Add `variant_name()` to `ClientRpcRequest` — ✅ DONE

**Problem:** `WasmPluginHandler::can_handle()` calls `marshal::extract_variant_name()`,
which serializes the *entire* request to `serde_json::Value` just to read the
top-level key. On a 275-variant enum with large payloads (e.g. `AddBlob { data }`),
this allocates and immediately discards the full JSON. Every native handler uses
zero-cost `matches!()` on the enum discriminant; the WASM path is the only one
paying this cost, and it runs on every request for every registered plugin.

**Solution:** Add a `variant_name(&self) -> &'static str` method to
`ClientRpcRequest` via a derive macro or manual match, returning the serde
variant name as a `&'static str`. Use it in `can_handle`.

### Files to change

| File | Change |
|------|--------|
| `crates/aspen-client-api/src/messages/mod.rs` | Add `pub fn variant_name(&self) -> &'static str` with a match arm per variant. ~275 lines, mechanical. Pattern: `Self::GetHealth => "GetHealth"`. Can be generated with a proc-macro but a hand-written match is fine for a stable enum. |
| `crates/aspen-wasm-plugin/src/marshal.rs` | Replace `extract_variant_name` body to delegate to `variant_name()`. Keep the function as a thin wrapper for the marshal module boundary, but change return type from `Option<String>` to `&'static str`. |
| `crates/aspen-wasm-plugin/src/handler.rs` | Update `can_handle`: use `extract_variant_name(request)` directly (no `.map()`, no `.unwrap_or(false)`). Simplifies to `self.handles.iter().any(...)`. |
| `crates/aspen-wasm-plugin/src/marshal.rs` (tests) | Update existing tests to assert `&'static str` return instead of `Option<String>`. |

### Acceptance criteria

- `extract_variant_name` is zero-alloc (no serde_json involved).
- All existing `marshal` and `handler` tests pass.
- `cargo test -p aspen-client-api -p aspen-wasm-plugin` green.

---

## 2. Enforce execution time limits — ✅ DONE

**Problem:** `PluginManifest` carries a `fuel_limit: Option<u64>` and
`aspen_constants::wasm` defines `DEFAULT_WASM_FUEL_LIMIT` / `MAX_WASM_FUEL_LIMIT`,
but `load_plugin()` in `registry.rs` never applies them. A misbehaving guest
can loop indefinitely.

**Constraint:** hyperlight-wasm 0.12 does **not** expose a fuel metering API.
It *does* expose `interrupt_handle()` on `LoadedWasmSandbox` and the host's
`SandboxConfiguration` accepts `interrupt_retry_delay`. The practical mechanism
is a wall-clock execution timeout enforced via `tokio::time::timeout` around
the `spawn_blocking` call in `handler.rs`.

**Solution:** Add wall-clock execution timeout to `WasmPluginHandler::handle()`.

### Files to change

| File | Change |
|------|--------|
| `crates/aspen-constants/src/wasm.rs` | Add `pub const DEFAULT_WASM_EXECUTION_TIMEOUT_SECS: u64 = 30;` and `pub const MAX_WASM_EXECUTION_TIMEOUT_SECS: u64 = 300;`. Keep existing fuel constants for forward-compat when hyperlight adds fuel support. |
| `crates/aspen-plugin-api/src/manifest.rs` | Add `pub execution_timeout_secs: Option<u64>` to `PluginManifest` (with `#[serde(default)]`). |
| `crates/aspen-wasm-plugin/src/handler.rs` | Store `execution_timeout: Duration` in `WasmPluginHandler`. Wrap the `spawn_blocking` call with `tokio::time::timeout(self.execution_timeout, ...)`. On timeout, return `anyhow::anyhow!("WASM plugin '{}' exceeded execution timeout of {:?}", ...)`. |
| `crates/aspen-wasm-plugin/src/registry.rs` | Resolve timeout from manifest: `manifest.execution_timeout_secs.unwrap_or(DEFAULT).min(MAX)`. Pass to `WasmPluginHandler::new()`. |
| `crates/aspen-wasm-plugin/src/lib.rs` (test_support) | Update `load_wasm_handler` to accept/pass a timeout. Default to `DEFAULT_WASM_EXECUTION_TIMEOUT_SECS`. |

### Acceptance criteria

- A plugin that sleeps or loops forever times out cleanly.
- Timeout is logged with plugin name.
- `cargo test -p aspen-wasm-plugin` green.
- Existing integration tests unaffected (30s default is generous).

---

## 3. Add KV prefix validation — **ALREADY DONE**

**Status:** ✅ Complete. After reading the code, this is fully implemented:

- `PluginHostContext` has `allowed_kv_prefixes: Vec<String>`
- `with_kv_prefixes()` resolves empty to `__plugin:{name}:` default
- `validate_key_prefix()` checks every KV get/put/delete/cas
- `validate_scan_prefix()` checks scan operations
- `load_plugin()` calls `.with_kv_prefixes(manifest.kv_prefixes.clone())`
- 12 unit tests cover: valid keys, rejected keys, empty prefixes, multiple
  prefixes, partial matches, scan validation, error messages

**No work needed.** Skip this item.

---

## 4. Consistent error encoding across all host functions — ✅ DONE

**Problem:** The 20 registered host functions use three different conventions
for encoding success/error results:

| Convention | Used by | How it works |
|-----------|---------|-------------|
| **`\0`/`\x01` prefix** | `blob_put` only | `\0{hash}` = ok, `\x01{msg}` = err |
| **Empty string = success** | `kv_put`, `kv_delete`, `kv_cas` | `""` = ok, `"error msg"` = err |
| **Empty vec = not-found** | `kv_get`, `blob_get` | Empty `Vec<u8>` = None |
| **Silent swallow** | `kv_scan` | Returns empty vec on error (indistinguishable from "no results") |

The `\0`/`\x01` convention is documented in the module header but only used by
`blob_put`. The guest SDK has to handle different error protocols per function.

**Solution:** Standardize on `\0`/`\x01` for all functions returning
`Result`-like values. Leave "returns `Vec<u8>`" functions (kv_get, blob_get)
using the empty-vec convention since they return `Option` semantics (not-found
vs error is a meaningful distinction — but currently both collapse to empty).

### Files to change

| File | Change |
|------|--------|
| `crates/aspen-wasm-plugin/src/host.rs` | **`kv_put` registration:** Change from `Ok(()) => String::new(), Err(e) => e` to `Ok(()) => "\0".to_string(), Err(e) => format!("\x01{e}")`. |
| `crates/aspen-wasm-plugin/src/host.rs` | **`kv_delete` registration:** Same change as `kv_put`. |
| `crates/aspen-wasm-plugin/src/host.rs` | **`kv_cas` registration:** Same change as `kv_put`. |
| `crates/aspen-wasm-plugin/src/host.rs` | **`kv_scan` registration:** Change to return `\0` + JSON bytes on success, `\x01` + error message on failure. Currently silently returns empty vec on error. Requires changing return type from `Vec<u8>` to `String` (with JSON in the payload), or keeping `Vec<u8>` and prepending a tag byte. Recommended: `Vec<u8>` with `[0x00] ++ json_bytes` for ok, `[0x01] ++ error_utf8` for err. |
| `crates/aspen-wasm-plugin/src/host.rs` | **`kv_get` registration:** Change to `Vec<u8>` with tag: `[0x00] ++ value_bytes` for found, `[0x01]` for not-found, `[0x02] ++ error_msg` for error (currently not-found and error both return empty vec). |
| `crates/aspen-wasm-plugin/src/host.rs` | **`blob_get` registration:** Same tag convention as `kv_get`. |
| `crates/aspen-wasm-plugin/src/host.rs` | Update module-level doc comment to reflect the standardized conventions. |
| Guest SDK (aspen-plugin-api or guest crate) | Update guest-side decode helpers to match new encoding. |

### Breaking change

This is a **wire-format breaking change** for existing guest plugins. Must be
coordinated with a guest SDK version bump. Options:

- **Option A (recommended):** Version the ABI. Add a `host_abi_version()` host
  function returning `u32`. Existing guests that don't call it get v1 behavior.
  New guests call it and get v2 encoding.
- **Option B:** Ship it as breaking change with a major version bump of the
  plugin API. All existing plugins need recompilation.

### Acceptance criteria

- All `Result`-returning host functions use `\0`/`\x01` tag encoding.
- All `Option`-returning host functions use `\0`/`\x01`/`\x02` tag encoding.
- Guest SDK decode helpers updated.
- Module-level doc comment updated.
- Existing tests updated and passing.

---

## 5. Document the Host ABI — `docs/HOST_ABI.md` — ✅ DONE

**Problem:** The 20 host functions have no formal ABI contract. The type
encoding rules are buried in a code comment in `host.rs`. Guest authors must
read Rust source to understand the interface.

**Solution:** Write `docs/HOST_ABI.md` — a spec-grade reference document.

### Content outline

```
# WASM Plugin Host ABI Reference

## Overview
- Sandbox: hyperlight-wasm 0.12, primitive mode
- All host functions registered via `ProtoWasmSandbox::register()`
- Guest calls host functions by name with typed parameters

## Type Encoding (Primitive Mode)
- String ↔ UTF-8
- Vec<u8> ↔ raw bytes
- u32/u64/i32/i64/f32/f64 ↔ native WASM types
- bool ↔ i32 (0/1)
- Result<T, E> → tagged string/bytes (see Error Encoding)

## Error Encoding Convention
- String results: \0 prefix = ok, \x01 prefix = err
- Vec<u8> results: 0x00 prefix = ok, 0x01 = not-found, 0x02 = error
- Empty string/vec conventions for simple cases

## Host Functions

### Logging
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| log_info | msg: String | () | Info-level log |
| log_debug | msg: String | () | Debug-level log |
| log_warn | msg: String | () | Warn-level log |

### Clock
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| now_ms | () | u64 | Wall-clock ms since epoch |
| hlc_now | () | u64 | HLC timestamp as ms since epoch |

### KV Store
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| kv_get | key: String | Vec<u8> | Get value (empty = not found) |
| kv_put | key: String, value: Vec<u8> | String | Put value (tagged result) |
| kv_delete | key: String | String | Delete key (tagged result) |
| kv_scan | prefix: String, limit: u32 | Vec<u8> | Scan keys (JSON-encoded) |
| kv_cas | key: String, expected: Vec<u8>, new: Vec<u8> | String | CAS (tagged result) |

### Blob Store
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| blob_has | hash: String | bool | Check existence |
| blob_get | hash: String | Vec<u8> | Get blob (empty = not found) |
| blob_put | data: Vec<u8> | String | Store blob (tagged: \0hash or \x01err) |

### Identity & Cluster
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| node_id | () | u64 | Host node ID |
| is_leader | () | bool | Current node is Raft leader? |
| leader_id | () | u64 | Leader node ID (0 = unknown) |

### Crypto
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| sign | data: Vec<u8> | Vec<u8> | Ed25519 sign (empty = no key) |
| verify | key: String, data: Vec<u8>, sig: Vec<u8> | bool | Ed25519 verify |
| public_key_hex | () | String | Node's public key (hex) |

### Randomness
| Function | Params | Returns | Description |
|----------|--------|---------|-------------|
| random_bytes | count: u32 | Vec<u8> | CSPRNG bytes (max 4096) |

## Namespace Isolation
- Every KV key validated against plugin's allowed_kv_prefixes
- Default prefix: __plugin:{name}:
- Violations logged and rejected silently (get returns empty, put returns error)

## Resource Limits
- Memory: manifest.memory_limit (default 256MB, max 1GB)
- Execution timeout: manifest.execution_timeout_secs (default 30s, max 300s)
- Scan results: max 10,000 entries per kv_scan
- Random bytes: max 4,096 per call
```

### Files to create/change

| File | Change |
|------|--------|
| `docs/HOST_ABI.md` | New file with the content above, fleshed out with full details. |
| `crates/aspen-wasm-plugin/src/host.rs` | Add `//! See [HOST_ABI.md](../../../docs/HOST_ABI.md)` reference to module doc. |

### Acceptance criteria

- Document covers all 20 host functions with exact signatures and encoding.
- Document is accurate relative to current code (or post-item-4 code).
- Cross-referenced from `host.rs` module docs.

---

## Implementation order

```
1. variant_name()           — zero dependencies, pure addition, easy test
2. HOST_ABI.md              — document current state before changing encoding
3. execution timeout        — independent of encoding changes
4. error encoding           — breaking change, requires guest SDK coordination
   (item 3 is already done — KV prefix validation)
```

Items 1-3 can be done in parallel. Item 4 should be last since it's a
breaking wire-format change and the ABI doc (item 5→2) should capture
the *current* state first, then be updated alongside the encoding change.
