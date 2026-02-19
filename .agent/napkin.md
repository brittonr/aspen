# Napkin

## Corrections

| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-18 | self | Told user Phase 2 (Capability Advertisement) wasn't done, but it was fully implemented | Check actual code before assessing roadmap status — grep for types/functions mentioned in the plan |
| 2026-02-18 | self | delegate_task worker reported CLI fix success but changes weren't on disk | Do surgical edits directly — delegate_task doesn't persist file writes reliably |

## User Preferences

- User wants to improve plugin system iteratively — lifecycle + hot-reload first
- When implementing multi-crate changes, do edits directly — delegate_task WILL lose file changes

## Patterns That Work

- Pre-commit hooks run rustfmt + clippy — doc comments must have blank lines before continuation lines (clippy::doc_lazy_continuation)
- Plugin system spans multiple crates: `aspen-plugin-api`, `aspen-wasm-plugin`, `aspen-wasm-guest-sdk`, plus individual plugin crates (`forge`, `hooks`, `service-registry`)
- `docs/planning/` contains architectural planning docs (e.g., `plugin-system.md`)
- Three sandbox backends: Hyperlight micro-VM (`plugins-vm`), hyperlight-wasm (`plugins-wasm`), Cloud Hypervisor full VM (`ci-vm`)
- Job execution also includes local shell and nix build modes
- `crates/aspen-jobs/src/vm_executor/` has the core worker implementations
- `crates/aspen-ci-executor-vm/` has CloudHypervisorWorker
- Feature flags control which backends are compiled in

## Patterns That Don't Work

- delegate_task workers may report success but not persist file changes — always verify with `git diff --stat` or `grep` after delegation
- For multi-file surgical edits, do them directly rather than delegating
- Worker for HOST_ABI.md creation reported success but file wasn't on disk. Confirmed delegate_task unreliability for file creation again.

## Recent Changes (2026-02-18)

### aspen-fuse: Read Cache

- New `cache.rs` module with TTL-bounded `ReadCache` (data, metadata, scan caches)
- `kv_read` → cache-first; `kv_write`/`kv_delete` → invalidate on write-through
- `kv_scan` → cached with 1s TTL; bulk `invalidate_prefix` for renames
- Constants: `CACHE_DATA_TTL` (5s), `CACHE_META_TTL` (2s), `CACHE_SCAN_TTL` (1s)

### aspen-fuse: Connection Pooling

- `ConnectionPool` in `client.rs` reuses QUIC connections across RPCs
- `send_rpc_inner` acquires from pool, falls back to fresh connection on stale
- `POOL_MAX_CONNECTIONS` = 8

### aspen-fuse: Persistent Timestamps

- New `metadata.rs` module with `FileMetadata` (32-byte binary: mtime + ctime)
- Stored as `.meta` suffix companion keys in KV
- `create`, `write`, `mkdir`, `symlink` → store initial timestamps
- `setattr` → handles `MTIME`, `MTIME_NOW`, `SIZE` flags to update timestamps
- `rename` → copies metadata to new key, updates ctime
- `unlink`, `rmdir` → deletes metadata companion keys
- `lookup`, `getattr` → read stored timestamps via `get_size_and_meta` (cached)
- `make_attr_with_meta` / `make_entry_with_meta` for persisted timestamp attrs

### aspen-plugin-api: Plugin Permissions

- New `PluginPermissions` struct: kv_read, kv_write, blob_read, blob_write, cluster_info, randomness, signing, timers
- Added `permissions` field to `PluginManifest` (`#[serde(default)]` for backward compat)
- `PluginPermissions::all()` for trusted plugins, `default()` = all denied

### aspen-wasm-plugin: Permission Enforcement

- `PluginHostContext.permissions` field wired from manifest on load
- `check_permission()` called before every host function (kv_get, kv_put, kv_delete, kv_scan, kv_cas, kv_batch, blob_has, blob_get, blob_put, random_bytes, is_leader, leader_id, sign, public_key_hex, schedule_timer, cancel_timer)
- CLI install defaults to `PluginPermissions::all()` for backward compat

### WASM Plugin Hook Event Subscriptions

- New `SubscriptionCommand` enum in `host.rs` (Subscribe/Unsubscribe), mirrors `SchedulerCommand` pattern
- `PluginHostContext.subscription_requests` — shared `Arc<Mutex<Vec>>` between host context and handler
- Two new host functions: `hook_subscribe(pattern)`, `hook_unsubscribe(pattern)` — enqueue commands
- New `events.rs` module: `PluginEventRouter` per plugin — holds patterns, delivers via `plugin_on_hook_event` guest export
- `WasmPluginHandler.event_router` — `OnceLock<Arc<PluginEventRouter>>`, created in `call_init`
- `new_with_scheduler()` now takes 6 args (added `subscription_requests`)
- Guest SDK: `subscribe_hook_events(pattern)` / `unsubscribe_hook_events(pattern)` safe wrappers
- `AspenPlugin::on_hook_event(topic, event)` trait method with default no-op
- `plugin_on_hook_event` export in `register_plugin!` macro — receives JSON `{"topic": "...", "event": {...}}`
- `PluginPermissions.hooks` — new permission field, default false
- Constants: `MAX_HOOK_SUBSCRIPTIONS_PER_PLUGIN = 16`, `MAX_HOOK_PATTERN_LENGTH = 256`
- Pattern matching: NATS-style `*` (one segment) and `>` (trailing), same as hooks service
- Event delivery: `PluginEventRouter::deliver(topic, event_json)` → `spawn_blocking` → `call_guest_function`

### aspen-secrets-plugin: WASM Secrets Engine

- New crate: `crates/aspen-secrets-plugin/` — cdylib WASM plugin
- KV prefix: `__secrets:`, priority: 940, app_id: `secrets`
- Handles 18 request types: SecretsKv{Read,Write,Delete,Destroy,Undelete,List,Metadata,UpdateMetadata,DeleteMetadata} + SecretsTransit{CreateKey,Encrypt,Decrypt,Sign,Verify,RotateKey,ListKeys,Rewrap,Datakey}
- KV v2: Versioned secrets with soft/hard delete, CAS, metadata — pure KV operations
- Transit: BLAKE3-hash-based symmetric encryption (keystream XOR + MAC), Ed25519 signing via host `sign()`/`verify()`
- Key storage: `__secrets:kv:{mount}:data:{path}:v{version}`, `__secrets:kv:{mount}:meta:{path}`, `__secrets:transit:{mount}:key:{name}`
- Wire format: `aspen:v{version}:{base64(nonce ++ ciphertext ++ mac)}`
- Permissions: kv_read, kv_write, blob_read, blob_write, randomness, signing
- Skips PKI (X.509/rcgen too complex for WASM) and NixCache (lower priority)

### aspen-automerge-plugin: WASM Automerge CRDT Plugin

- New crate: `crates/aspen-automerge-plugin/` — cdylib WASM plugin
- KV prefix: `automerge:`, priority: 935, app_id: `automerge`
- Handles 11 request types: Automerge{Create,Get,Save,Delete,ApplyChanges,Merge,List,GetMetadata,Exists,GenerateSyncMessage,ReceiveSyncMessage}
- Document content stored as base64 in `automerge:{doc_id}`, metadata as JSON in `automerge:_meta:{doc_id}`
- Sync state per peer: `automerge:_sync:{doc_id}:{peer_id}` — simplified hash-based sync (full snapshots)
- Does NOT link the `automerge` crate — documents are opaque base64 blobs, CRDT logic is client-side
- Permissions: kv_read, kv_write, blob_read, blob_write, randomness
- `aspen-client-api` automerge types are behind `#[cfg(feature = "automerge")]` — plugin Cargo.toml must enable it

## Domain Notes

- Aspen is a Rust project with a WASM plugin system using `hyperlight-wasm`
- Three-tier plugin architecture: native, WASM, gRPC/IPC
- Plugin priority range: 900-999 (WASM plugins)
- Follows FoundationDB "unbundled database" / stateless layers philosophy
- Plugins store state in core KV/blob primitives with strict key prefix namespacing
- Two separate trait hierarchies: `Worker` (aspen-jobs, job execution) and `RequestHandler` (aspen-rpc-core, RPC dispatch) — NOT unified
- `AspenPlugin` (guest SDK) bridges into `RequestHandler` via `WasmPluginHandler`
- `HandlerFactory` + `inventory` crate used for self-registration of RequestHandlers at link time
- `WorkerPool` routes jobs to Workers by `job_types()`
- KV namespace isolation: `PluginHostContext.allowed_kv_prefixes` + `validate_key_prefix()`/`validate_scan_prefix()` in `host.rs`
- Plugin KV prefixes: forge uses `forge:`, hooks uses `__hooks:`, service-registry uses `__service:`
- Empty `kv_prefixes` in manifest → auto-scoped to `__plugin:{name}:` via `with_kv_prefixes()`
- CLI `plugin install` supports `--kv-prefixes` flag and reads from plugin.json manifest
- Echo plugin example is at `examples/plugins/echo-plugin/`
- Phase 2 (Capability Advertisement) is fully implemented: AppManifest, AppRegistry, ClusterAnnouncement, required_app(), CapabilityUnavailable, handler app_id(), federation discovery
- Phase 4 (Cross-Cluster Proxying) added: ProxyConfig, ProxyService, proxy_hops on AuthenticatedRequest, dispatch tries proxy before CapabilityUnavailable
- `HandlerRegistry::dispatch()` takes 3 args: (request, ctx, proxy_hops)
- Federation discovery is behind `#[cfg(all(feature = "forge", feature = "global-discovery"))]`
- All plugin crates have `plugin_info_matches_manifest` tests that check code ↔ plugin.json consistency
- ~~Pre-existing: `aspen-constants` has a broken doctest~~ FIXED 2026-02-18
- ~~Pre-existing: `aspen-cli` has unresolved `aspen_forge` import errors~~ FIXED 2026-02-18
- Pre-commit hooks: shellcheck warnings on scripts/ are pre-existing, not blockers
- `HandlerRegistry` now uses `ArcSwap` for hot-reload — field access is `self.handlers.load()` not `self.handlers`
- `add_handlers()` takes `&self` not `&mut self` (ArcSwap enables interior mutability)
- `load_wasm_plugins()` still takes `&mut self` because it stores the LivePluginRegistry
- `PluginReload` request handled directly in `dispatch()` — not via a separate handler (avoids circular dependency)
- pijul unused import warning in CLI is pre-existing, not from our changes
- Guest SDK `kv_get_value`, `kv_scan_prefix`, `blob_get_data` now return `Result` — plugin call sites use `.ok()??` for Option-returning helpers or `match Ok(Some)/Ok(None)/Err` for handlers
- Plugin kv.rs wrappers don't have direct `aspen_plugin_api` dep — use `aspen_wasm_guest_sdk::KvBatchOp` re-export
- `PluginHostContext.scheduler_requests` is `Arc<Mutex<Vec<SchedulerCommand>>>` — shared between host context and handler
- Handler has `new_with_scheduler()` for registry to pass the shared queue
- `PluginScheduler` created in `call_init()` success path via `OnceLock`, processes commands after each guest call
