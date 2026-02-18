# WASM Plugin Host ABI Reference

> Formal contract for the 20 host functions available to WASM handler plugins
> running in hyperlight-wasm 0.12 primitive-mode sandboxes.

## Overview

WASM handler plugins execute inside hyperlight-wasm micro-VM sandboxes.
The host registers functions on a `ProtoWasmSandbox` before loading the
guest module. Guests call these functions by name using hyperlight's
primitive-mode FFI — no WIT/component-model bindings are needed.

**Runtime:** hyperlight-wasm 0.12, primitive mode
**Registration:** `crates/aspen-wasm-plugin/src/host.rs` → `register_plugin_host_functions()`
**Guest SDK:** `crates/aspen-wasm-guest-sdk/src/host.rs`

## Type Encoding (Primitive Mode)

Only these types cross the host/guest boundary:

| Rust Type | WASM Encoding | Notes |
|-----------|---------------|-------|
| `String` | UTF-8 bytes | Hyperlight handles alloc/dealloc |
| `Vec<u8>` | Raw bytes | Hyperlight handles alloc/dealloc |
| `u32` / `u64` / `i32` / `i64` | Native WASM integers | |
| `f32` / `f64` | Native WASM floats | |
| `bool` | `i32` (0 = false, 1 = true) | |
| `()` | Void (no return value) | |

Complex types are encoded within these primitives:

| Logical Type | Wire Encoding |
|-------------|---------------|
| `Result<(), String>` | `String`: `\0` prefix = ok, `\x01` prefix + message = err |
| `Result<String, String>` | `String`: `\0` prefix + value = ok, `\x01` prefix + message = err |
| `Option<Vec<u8>>` | `Vec<u8>`: `[0x00]` + data = found, `[0x01]` = not-found, `[0x02]` + error = error |
| `Result<Vec<u8>, String>` | `Vec<u8>`: `[0x00]` + payload = ok, `[0x01]` + error = error |

## Error Encoding Convention

All host functions returning results use a **tag prefix** convention.

### String-based results

Used by: `kv_put`, `kv_delete`, `kv_cas`, `blob_put`

| First byte | Meaning | Payload |
|-----------|---------|---------|
| `\0` (0x00) | Success | Optional value after the tag |
| `\x01` (0x01) | Error | Error message (UTF-8) after the tag |

### Vec-based option results

Used by: `kv_get`, `blob_get`

| First byte | Meaning | Payload |
|-----------|---------|---------|
| `0x00` | Found | Value bytes after the tag |
| `0x01` | Not found | No payload |
| `0x02` | Error | Error message (UTF-8) after the tag |

### Vec-based results

Used by: `kv_scan`

| First byte | Meaning | Payload |
|-----------|---------|---------|
| `0x00` | Success | JSON-encoded `Vec<(String, Vec<u8>)>` after the tag |
| `0x01` | Error | Error message (UTF-8) after the tag |

The guest SDK's `decode_tagged_unit_result()` and `decode_tagged_option_result()`
handle backwards-compatible decoding (no-tag-prefix → legacy behavior).

## Host Functions

### Logging

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `log_info` | `msg: String` | `()` | Info-level structured log with plugin name context |
| `log_debug` | `msg: String` | `()` | Debug-level structured log |
| `log_warn` | `msg: String` | `()` | Warn-level structured log |

### Clock

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `now_ms` | _(none)_ | `u64` | Wall-clock time: milliseconds since Unix epoch |
| `hlc_now` | _(none)_ | `u64` | HLC timestamp as milliseconds since epoch. Falls back to wall clock if HLC not configured. |

### KV Store

All KV operations enforce namespace isolation via prefix validation.
See [Namespace Isolation](#namespace-isolation) below.

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `kv_get` | `key: String` | `Vec<u8>` | Get value by key. Tagged: `[0x00]+data` = found, `[0x01]` = not-found, `[0x02]+msg` = error/namespace violation. |
| `kv_put` | `key: String, value: Vec<u8>` | `String` | Put key-value pair. Value must be valid UTF-8. Tagged result: `\0` = ok, `\x01msg` = err. |
| `kv_delete` | `key: String` | `String` | Delete key. Tagged result: `\0` = ok, `\x01msg` = err. |
| `kv_scan` | `prefix: String, limit: u32` | `Vec<u8>` | Scan keys by prefix. Tagged: `[0x00]+JSON` = ok (JSON-serialized `Vec<(String, Vec<u8>)>`), `[0x01]+msg` = error. Limit 0 = default (1,000); max 10,000. |
| `kv_cas` | `key: String, expected: Vec<u8>, new_value: Vec<u8>` | `String` | Compare-and-swap. Empty `expected` = create-if-absent. Both must be valid UTF-8. Tagged result: `\0` = ok, `\x01msg` = err. |

### Blob Store

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `blob_has` | `hash: String` | `bool` | Check if blob exists. `hash` is hex-encoded BLAKE3. Returns `false` on invalid hash or error. |
| `blob_get` | `hash: String` | `Vec<u8>` | Get blob bytes. Tagged: `[0x00]+data` = found, `[0x01]` = not-found, `[0x02]+msg` = error/invalid hash. |
| `blob_put` | `data: Vec<u8>` | `String` | Store blob, returns tagged result: `\0{hex_hash}` = ok, `\x01{msg}` = err. |

### Identity

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `node_id` | _(none)_ | `u64` | Numeric node ID of the host. |

### Cluster

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `is_leader` | _(none)_ | `bool` | Whether current node is the Raft leader. |
| `leader_id` | _(none)_ | `u64` | Current Raft leader's node ID. Returns `0` if unknown. |

### Crypto

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `sign` | `data: Vec<u8>` | `Vec<u8>` | Ed25519 sign with host's secret key. Returns 64-byte signature. **Empty vec = no key configured.** |
| `verify` | `key: String, data: Vec<u8>, sig: Vec<u8>` | `bool` | Ed25519 verify. `key` is hex-encoded 32-byte public key. `sig` is 64 bytes. Returns `false` on any decode error. |
| `public_key_hex` | _(none)_ | `String` | Host's Ed25519 public key as hex string. **Empty string = no key configured.** |

### Randomness

| Function | Parameters | Returns | Description |
|----------|-----------|---------|-------------|
| `random_bytes` | `count: u32` | `Vec<u8>` | CSPRNG bytes via `getrandom`. **Capped at 4,096 bytes per call.** Returns zeroed bytes if CSPRNG fails. |

## Namespace Isolation

Every KV operation validates the key (or scan prefix) against the plugin's
`allowed_kv_prefixes` list:

- **Explicit prefixes:** Set via `kv_prefixes` in the plugin manifest.
- **Default prefix:** If `kv_prefixes` is empty, auto-scoped to `__plugin:{name}:`.
- **Validation:** `key.starts_with(prefix)` for any allowed prefix.
- **On violation:** KV get returns `[0x02]+msg` (error tag); KV put/delete/cas return
  `\x01msg` (error tag); KV scan returns `[0x01]+msg` (error tag). All violations
  are logged at warn level.

```
Plugin "forge" with kv_prefixes: ["forge:", "forge-cobs:"]
  ✓ kv_get("forge:repos:abc")       — matches "forge:"
  ✓ kv_put("forge-cobs:data", ...)  — matches "forge-cobs:"
  ✗ kv_get("__hooks:config")        — namespace violation
```

## Resource Limits

| Resource | Default | Maximum | Enforced By |
|----------|---------|---------|-------------|
| Guest memory | 256 MB | 1 GB | `SandboxBuilder::with_guest_heap_size()` |
| Execution timeout | 30 s | 300 s | `tokio::time::timeout` around `spawn_blocking` |
| Fuel metering | 100M | 10B | **NOT ENFORCED** — hyperlight-wasm 0.12 lacks fuel API |
| WASM binary size | — | 50 MB | Validated in `load_plugin()` |
| kv_scan results | 1,000 | 10,000 | Clamped in `kv_scan` host function |
| random_bytes | — | 4,096 | Clamped in `random_bytes` host function |
| Plugin count | — | 100 | `MAX_PLUGINS` constant |

## Guest Exports

Plugins must export these functions:

| Export | Signature | Description |
|--------|----------|-------------|
| `plugin_info` | `() -> Vec<u8>` | Returns JSON-serialized `PluginInfo` struct |
| `handle_request` | `Vec<u8> -> Vec<u8>` | Receives JSON `ClientRpcRequest`, returns JSON `ClientRpcResponse` |

## Version History

| Version | Date | Changes |
|---------|------|---------|
| v1 | Initial | Mixed error encoding: blob_put used `\0`/`\x01`, kv_put/delete/cas used empty-string convention, kv_get/blob_get returned empty vec for both not-found and error |
| v2 | 2026-02-18 | Standardized all Result-returning functions to `\0`/`\x01` tag prefix. Added execution timeout. Guest SDK `decode_tagged_unit_result()` handles both v1 and v2. |
| v3 | 2026-02-18 | Standardized kv_get/blob_get to `[0x00]/[0x01]/[0x02]` tagged encoding (found/not-found/error). Standardized kv_scan to `[0x00]/[0x01]` tagged encoding (ok/error). Guest SDK `decode_tagged_option_result()` handles backwards compat. |
