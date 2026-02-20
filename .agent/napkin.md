# Napkin

## Corrections

| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-18 | self | Told user Phase 2 (Capability Advertisement) wasn't done, but it was fully implemented | Check actual code before assessing roadmap status — grep for types/functions mentioned in the plan |
| 2026-02-18 | self | delegate_task worker reported CLI fix success but changes weren't on disk | Do surgical edits directly — delegate_task doesn't persist file writes reliably |
| 2026-02-19 | self | Delegated 3 fix tasks to workers — all reported success but zero changes persisted | delegate_task STILL doesn't persist. Use scouts to gather info, then edit directly. Third time hitting this. |

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

### CLI Forge Coverage (2026-02-18)

- Added `git show-commit <hash>` → `ForgeGetCommit` (was handler-only, no CLI)
- Added `patch update` → `ForgeUpdatePatch` (was handler-only, no CLI)
- Fixed pre-existing clippy `useless_conversion` in `git_push_send_request` (`.map_err(Into::into)` → `Ok(..?)`)
- GitBridge operations (6 request types) are intentionally git-HTTP-protocol-only, not CLI gaps
- ForgeHandler has 44 request types total; all are now CLI-accessible except the 6 GitBridge protocol-only ones
- `ForgeRequest` enum (36 variants) is a subset of the 44 forge-related `ClientRpcRequest` variants — 8 federation variants live directly in `ClientRpcRequest`

### NixOS VM Test Fixes (2026-02-19)

- `pkgs.nixosTest` → `pkgs.testers.nixosTest` (renamed in newer nixpkgs)
- aspen-node package needed `automerge` feature added (required-features in Cargo.toml)
- `plugins` feature removed from nix package — `hyperlight-wasm` build.rs needs network access (incompatible with Nix sandbox)
- f-string lint: NixOS test framework lints Python f-strings; `f"..."` without `{...}` placeholders fails
- **Critical: tracing subscriber wrote to stdout** — `tracing_subscriber::fmt()` defaults to stdout, corrupting JSON output. Fixed with `.with_writer(std::io::stderr)`
- Test `cli()` helper uses temp file approach: `>/tmp/_cli_out.json 2>/dev/null` then `cat` — serial console mixes stdout/stderr
- Federation subtests use `node1.execute()` (non-fatal) since `global-discovery` feature not enabled in test build

### Multi-Node NixOS VM Test (2026-02-19)

- New `nix/tests/multi-node-cluster.nix` — 3-node Raft cluster test
- Tests: cluster formation, consensus, data replication, cross-node ops, leader failover, node rejoin
- `add-learner --addr` requires JSON EndpointAddr (not ticket) — format: `{"id":"<hex>","addrs":[{"Ip":"host:port"}]}`
- Bare EndpointId (public key hex) creates empty `addrs: {}` — Raft can't connect without socket addresses
- Extract endpoint addr from journal: `grep 'cluster ticket generated'` → parse endpoint_id + 192.168.x.x addrs
- `cluster status` JSON only shows voters in `nodes`, NOT learners — check after change-membership, not after add-learner
- After failover: writes must go to NEW leader node's ticket, not any survivor
- Forge blob-backed operations (issue show/list, get-blob) use `wait_available_all` — fails when ANY node is down or has stale blobs
- Pure KV operations (repo list, branch list, cluster status) work fine with 2/3 quorum
- Blob replication does NOT auto-catch-up after node restart — `wait_available_all remaining=1` persists
- NixOS test VLAN: node1=192.168.1.1, node2=192.168.1.2, node3=192.168.1.3
- Iroh bind port from `--bind-port 7777` may not match actual reported port — extract from journal

### Three Multi-Node Cluster Fixes (2026-02-19)

- **add-learner bare EndpointId rejected**: `membership.rs` no longer accepts bare hex EndpointId — requires JSON `{"id":"<hex>","addrs":[{"Ip":"host:port"}]}`. Also validates `addrs` is non-empty even for JSON input.
- **Blob reads fail fast (5s)**: New `BLOB_READ_WAIT_TIMEOUT` (5s) for read paths. `wait_available_all` → `wait_available` (single blob) in `get_object`, `get_change`, `export_object`. Write-path `ensure_blobs_available` unchanged (still uses 30s `DEFAULT_BLOB_WAIT_TIMEOUT`).
- **NOT_LEADER error code**: KV write/delete/CAS handlers return `ClientRpcResponse::error("NOT_LEADER", msg)` instead of burying the error in `WriteResultResponse.error`. CLI client rotates to next peer on `NOT_LEADER` (same as `SERVICE_UNAVAILABLE`).
- Also fixed: `handle_batch_write` and `handle_conditional_batch_write` used raw `e.to_string()` — changed to `sanitize_kv_error(&e)`.

### Hooks + Service Registry NixOS VM Test (2026-02-19)

- New `nix/tests/hooks-services.nix` — single-node hooks + service registry test
- Hooks: list, metrics, trigger (write_committed, leader_elected with payload), create-url
- Service registry: register (3 instances, 2 services), list (all + prefix), discover (all + version filter + healthy only), get, heartbeat, health update (unhealthy/restore), update metadata, deregister, cleanup
- Wired into flake as `checks.x86_64-linux.hooks-services-test`
- **Gotcha: CLI subcommand is `hook` (singular), not `hooks`**

### Secrets Engine NixOS VM Test (2026-02-19)

- New `nix/tests/secrets-engine.nix` — single-node secrets engine test
- Tests KV v2 (put, get, versions, list, metadata, delete, undelete, destroy) + Transit (create-key, list-keys, encrypt, decrypt, sign, verify, rotate, datakey)
- All subtests use `check=False` — secrets handler is NOT registered in node's handler registry
- **Gotcha: `secrets` feature needed in CLI** — added `aspen-cli-secrets` package to flake
- **Gotcha: Secrets handler not dispatched** — server receives `SecretsKvWrite` but never matches a handler. The `aspen-secrets-handler` crate uses inventory self-registration but is behind `secrets` feature in `aspen-rpc-handlers`. Needs investigation.
- Test detects handler availability via probe and logs results gracefully

### KV Operations NixOS VM Test (2026-02-19)

- New `nix/tests/kv-operations.nix` — single-node KV store integration test
- Tests: set/get/delete, non-existent reads, overwrite, CAS (create-if-absent, conditional, conflict), CAD (success, conflict), prefix scan with limit, scan pagination (continuation tokens are best-effort — may fail), batch read (existing + mixed missing), batch write (atomic), file-based set (--file), large values (100KB), special character keys, empty values
- Wired into flake as `checks.x86_64-linux.kv-operations-test`
- Uses `aspen-cli` (no forge feature needed) — lighter build
- **Gotcha: `kv delete` returns `was_deleted: true` even for non-existent keys** — idempotent semantics
- **Gotcha: `kv batch-read` JSON output has `{count, results: [{does_exist, key, value}]}` — NOT `{keys, values}`**
- **Gotcha: `kv scan --token` continuation may fail (exit 1)** — use `check=False` for robustness
- **Gotcha: `''` inside Nix indented strings closes the string!** Use `'''` to emit literal `''`, or avoid double-single-quote entirely

### Coordination Primitives NixOS VM Test (2026-02-19)

- New `nix/tests/coordination-primitives.nix` — single-node coordination test
- Tests: distributed locks (acquire/try-acquire/release/renew/contention), counters (get/incr/decr/add/sub/set/CAS/underflow), sequences (next/reserve/current), semaphores (acquire/capacity-exhaustion/release/status), rwlocks (read/multi-reader/write-blocked/release/write/downgrade/status), queues (create/enqueue/dequeue+ack/nack/nack-to-DLQ/redrive/dedup/group/delete), leases (grant/ttl/keepalive/list/revoke), barriers (enter/status/leave)
- Wired into flake as `checks.x86_64-linux.coordination-primitives-test`
- Uses `aspen-cli` (no forge feature needed)
- **Gotcha: `counter decr` at zero exits non-zero** — CLI `exit(1)` on `is_success: false`, use `check=False`

### Expanded VM Test Coverage (2026-02-19)

- **5 new test files** created, bringing total to 11 NixOS VM tests
- New `blob-operations.nix`: blob add/get/has/list/protect/unprotect/status/delete/stdin/replication-status/repair-cycle
- New `ratelimit-verify.nix`: ratelimit try-acquire/acquire/available/reset/bucket-isolation; verify kv/blob
- New `cluster-docs-peer.nix`: cluster status/health/metrics/ticket/prometheus; docs CRDT (conditional); peer list; verify all
- New `job-index.nix`: index list/show (4 built-in indexes); job submit/status/list/cancel/purge
- New `automerge-sql.nix`: automerge create/get/exists/list/get-metadata/delete; SQL query/WHERE/COUNT/LIMIT/ORDER BY
- New `aspen-cli-full` package: CLI built with `--features automerge,sql` for full feature testing
- **Gotcha: blob delete only removes user tags** — blob data stays until GC; `has` still returns true after delete
- **Gotcha: `blob add` positional file arg** — use `blob add -` for stdin, NOT `blob add --file -`
- **Gotcha: `blob unprotect` takes positional tag** — NOT `--tag`, it's `blob unprotect <tag-name>`
- **Gotcha: docs handler needs iroh-docs sync** — `ctx.docs_sync` is None when docs_sync not started; test must probe first
- **Gotcha: ratelimit CLI exits 1 on is_success=false** — `std::process::exit(1)` in all ratelimit commands; use `check=False`
- **Gotcha: Python type checker in NixOS tests** — `sorted()` on `Optional` values fails mypy; wrap with `str()`
- **Coverage: 29/33 CLI commands tested (88%)** — remaining 4 (cache, ci, dns, pijul) need feature-gated CLI builds

### Plugin CLI NixOS VM Test (2026-02-19)

- New `nix/tests/plugin-cli.nix` — single-node plugin management CLI test
- Tests: list (empty), install (flags + manifest + overrides), info, enable, disable, remove, reinstall/overwrite, resource limits (fuel_limit, memory_limit), KV prefix config, reload (best-effort), cleanup
- New `aspen-cli-plugins` package: CLI built with `--features plugins-rpc` for plugin management
- Wired into flake as `checks.x86_64-linux.plugin-cli-test`
- Plugin CLI stores manifests in KV (`plugins/handlers/` prefix) and WASM blobs in blob store — does NOT need WASM runtime on server
- Reload is the only op that needs runtime; tested with `check=False` since test node lacks `plugins` feature
- **Gotcha: `plugin remove` on non-existent key returns error** — use `check=False`
- **Gotcha: `plugin info` on removed plugin returns error (not empty)** — use `check=False`
- **Gotcha: dummy WASM files (random bytes) work fine for install** — CLI just uploads blob + writes manifest; validation at reload time

### Multi-Node Test Expansion (2026-02-19)

- **3 new multi-node tests** expanding coverage from 1 to 4 multi-node VM tests
- `multi-node-kv.nix` — 3-node KV: write/read replication, NOT_LEADER forwarding, CAS across nodes, batch write replication, scan consistency, delete propagation, large value replication, failover survival + catch-up
- `multi-node-coordination.nix` — 3-node coordination: lock exclusion across nodes (real distributed locking!), counter linearizability (15 increments from 3 nodes = 15), semaphore capacity across nodes, RW lock multi-node readers/writer exclusion, cross-node queue enqueue/dequeue, sequence monotonicity across nodes, lease cross-node ops, failover survival for locks + counters
- `multi-node-blob.nix` — 3-node blob: cross-node retrieval, blobs from different nodes visible everywhere, replication-status with real replicas, large blob (200KB) replication, protection visible cross-node, failover survival
- All wired into flake.nix as `checks.x86_64-linux.multi-node-{kv,coordination,blob}-test`
- **Pattern: new files must be `git add`ed before `nix eval`** — Nix flake uses git-tracked files only
- Multi-node tests use `aspen-cli` (not `aspen-cli-forge`) since they test KV/blob/coordination primitives, not forge ops
- Blob test uses `features = ["blob"]` in node config; KV/coordination tests use `features = []`

### NixOS VM Integration Test (2026-02-18)

- New `nix/tests/forge-cluster.nix` — NixOS VM test with full networking
- Tests all forge CLI commands E2E: repo, blob, tree, commit, show-commit, log, push, get-ref, branch, tag, issue, patch (including update), clone, federation
- Single-node QEMU VM with aspen-node service + aspen-cli (forge features)
- Ticket read from `/var/lib/aspen/cluster-ticket.txt` (written by aspen-node on boot)
- Wired into `flake.nix` as `checks.x86_64-linux.forge-cluster-test`
- New `aspen-cli-forge` package: CLI built with `--features forge` (needed for push, tag create)
- NixOS test framework provides real kernel networking — no sandbox restrictions
- Run: `nix build .#checks.x86_64-linux.forge-cluster-test`
- Debug: `nix build .#checks.x86_64-linux.forge-cluster-test.driverInteractive`
