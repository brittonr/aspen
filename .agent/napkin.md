# Napkin

## Corrections

| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-25 | self | delegate_task workers report success but file changes don't persist (5 incidents) | **NEVER use delegate_task for file edits.** Only use for read-only info gathering. Always verify with `git diff --stat` after any delegation. Do surgical edits directly. |
| 2026-02-25 | self | Postcard enum discriminant mismatch: `ClientRpcResponse` variants shifted by `#[cfg(feature = "ci")]` causing CLI to deserialize wrong variant | Feature flags that add enum variants MUST match between producer (node) and consumer (CLI). Always build CLI with same feature set that affects aspen-client-api enum layout. Affects: secrets CLI needs `ci` feature for CacheMigration variants before Secrets variants. |
| 2026-02-24 | self | WASM plugin AOT precompile version mismatch: wasmtime 36.0.6 vs hyperlight-wasm guest runtime 36.0.3 | Pin wasmtime exactly to match `hyperlight_wasm::get_wasmtime_version()`. Use `config.target("x86_64-unknown-none")` not `linux`. Enable `config.wasm_component_model(true)` to match guest runtime flags. |
| 2026-02-24 | self | Guest SDK extern declarations used Rust high-level types (String, Vec<u8>) producing wrong wasm32 ABI | All host function externs MUST use raw C types (`*const c_char`, `*const u8`, `i32`, `i64`) matching hyperlight primitive ABI. Vec<u8> params require explicit `_len: i32` following the buffer param. |
| 2026-02-24 | self | Host kv_get returned `\x02` (error) for non-existent keys; ALL tag bytes used `\x00` for success causing CString NUL termination issues | KV store `.read()` returns `Err(NotFound)`, NOT `Ok(None)`. Match `NotFound` specifically in host functions. Hyperlight marshals through CString → shift all tag bytes +1: success=`\x01`, not-found=`\x02`, error=`\x03`. |
| 2026-02-24 | self | plugin_init GPF from heap corruption in guest free() | Guest malloc() must prepend 8-byte header with total size. Guest free() reads header to pass correct size to wasm32 dlmalloc (which uses layout.size() for chunk metadata lookup). Wrong size → heap corruption. |
| 2026-02-23 | self | Adding field to widely-used struct breaks ~40 callers across repos | Don't add fields to public structs with many external users. Instead: add default trait method (purely additive, zero caller changes). |
| 2026-02-23 | self | `path:` flake inputs for local repos timeout copying target/ dirs | Use `git+file://` (respects .gitignore) or `builtins.fetchGit`. Ensure repos have `.git` + `.gitignore` with `target/`. |
| 2026-02-23 | self | Symlink causes Cargo package collision (same crate seen as two different paths) | Use SRCDIR variable to rewrite paths consistently instead of symlinks. Cargo sees physical paths, not logical equivalence. |
| 2026-02-23 | self | Subcrates at `crates/{name}/Cargo.toml` used `../../aspen-*/` for cross-workspace paths (wrong depth) | Subcrates are 3 levels deep, need `../../../aspen-*/crates/...` to reach git root. |
| 2026-02-23 | self | `aspen-jobs-guest` (no_std with `#[panic_handler]`) fails `cargo test` — duplicate panic_impl lang item | no_std test binaries link std (→ panic_impl), AND feature unification activates serde/std from siblings. Fix: `test = false` + `doctest = false` in `[lib]`. Add to `default-members` exclusion. |
| 2026-02-19 | self | NixOS VM tests with large data (100KB+ KV, 200KB blobs) fail from log truncation | Keep test data small: 5KB for KV values, 10KB for blobs. Use `logLevel="info"` (not "aspen=debug") for multi-node tests to avoid tracing large payloads. |
| 2026-02-19 | self | Multi-node test hit 50-connection client limit | MAX_CLIENT_CONNECTIONS was too low for tests making 80+ sequential CLI calls. Increased to 200. Batch operations when possible. |
| 2026-02-25 | self | KV reads on followers returned silent "key not found" instead of NOT_LEADER error | KV/lease handlers must check `is_not_leader_error()` and return top-level `ClientRpcResponse::error("NOT_LEADER", ...)`, NOT bury error in domain response fields (WriteResultResponse.error, ReadResultResponse.error). Client `send()` only rotates peers on top-level Error with code="NOT_LEADER". |
| 2026-02-25 | self | `GetClusterTicket` generated single-peer ticket → client loops on same follower after NOT_LEADER | Include all cluster nodes in bootstrap_peers (up to MAX_BOOTSTRAP_PEERS=16). Client rotation requires multi-peer ticket. |

## User Preferences

- Improve plugin system iteratively
- For multi-crate changes: do edits directly (delegate_task unreliable for file writes)

## Patterns That Work

**Multi-Repo Architecture (1 main package, 48 sibling repos):**

- Main repo has `[patch."https://github.com/brittonr/aspen.git"]` section mapping git deps to local `path = "../aspen-{name}/crates/..."` to prevent type duplication
- Sibling repos use `git = "https://github.com/..."` in workspace deps; main repo patches override with local paths
- Cross-workspace subcrate paths are 3 levels deep: `../../../aspen-{repo}/crates/{crate}`
- **Without [patch]**: types from git aspen-core ≠ types from workspace aspen-core → trait implementation failures (E0277)
- **workspace = true in extracted crates pulls entire git repo** → type duplication; use explicit `path = "..."` deps instead
- **Nix fullSrc layout mirrors real filesystem**: `$out/aspen/` + `$out/aspen-*/` as peers → NO path rewriting. Use `postUnpack = 'sourceRoot="$sourceRoot/aspen"'` for crane to enter the subdirectory. Siblings at `../aspen-*/` resolve naturally.
- **Cannot nest sibling repos inside main workspace dir**: Cargo resolves `workspace = true` against the outermost `[workspace]` root, not the inner one. `exclude` doesn't help. Keep siblings as peers (same parent dir), never children.

**WASM Plugin System (hyperlight-wasm):**

- Three-tier dispatch: native `RequestHandler` → `ServiceExecutor` → WASM `AspenPlugin`
- Plugin KV namespace isolation: `allowed_kv_prefixes` + `validate_key_prefix()` enforcement
- Empty `kv_prefixes` in manifest → auto-scoped to `__plugin:{name}:`
- Target spec filename becomes target name: use `x86_64-hyperlight-none.json` for correct sysroot lookup
- Pre-build wasm_runtime in separate derivation, patch vendored build.rs to use `HYPERLIGHT_WASM_RUNTIME` env var
- cargo-hyperlight is `[patch.crates-io]` → replace entire build.rs to eliminate dependency in vendored builds
- wasm32 malloc/free must track size: prepend 8-byte header, read in free() for dlmalloc chunk metadata
- hyperlight host function string returns NOT auto-freed: guest must free host function returns itself
- Permissions: `PluginPermissions` with per-capability bools (kv_read, kv_write, blob_read, blob_write, hooks, sql_query, etc.)
- Plugin registry requires linearizable KV scan (ReadIndex) → **only works on Raft leader**; followers fail with "not leader"
- **WASM plugin hot-reload after failover**: Pre-stage blobs on followers via `blob add`, trigger `plugin reload` on the **new leader** only

**NixOS VM Tests:**

- `skipLint = true` for complex Python scripts (type checker chokes on certain patterns)
- Two CLIs in VM: `aspen-cli` (test features) + `aspen-plugin-cli` (plugins-rpc) avoids binary name conflict
- CLI temp file pattern: `>/tmp/_cli_out.json 2>/dev/null` then `cat` (serial console mixes stdout/stderr)
- Delete cluster-ticket.txt before systemd restart to avoid stale ticket with wrong ports
- Restart nodes one at a time with health check between (simultaneous 2/3 restart breaks quorum)
- `memorySize = 4096` (hyperlight needs more RAM than default 1024)
- `logLevel = "info"` for multi-node tests (debug tracing of large payloads fills logs)

**Nix Flake:**

- `fullPluginsCargoVendorDir` needed for `cargo clippy --workspace` (compiles hyperlight-wasm even without `--features plugins-rpc`)
- `fullNodeCargoArtifacts` uses patched vendor dir + `HYPERLIGHT_WASM_RUNTIME` for plugin builds
- `pkgs.nixosTest` → `pkgs.testers.nixosTest` (renamed in newer nixpkgs)

## Patterns That Don't Work

- delegate_task for file creation/edits (5 confirmed failures across sessions)
- Adding fields to public structs with many external consumers (use trait methods instead)
- `workspace = true` in extracted crate workspace deps with git URLs (pulls entire repo, causes type duplication)
- Bare EndpointId in `add-learner` (requires JSON with `addrs` array)
- Plugin reload on Raft follower (KV scan needs ReadIndex leadership)

## Domain Notes

**Architecture:**

- Aspen node binary (main repo) + 47 sibling library repos
- Plugin system: 3-tier dispatch (RequestHandler → ServiceExecutor → WASM)
- Native handlers: blob, cluster, core-essentials, forge-federation+git-bridge only
- WASM plugins: coordination, automerge, secrets, service-registry, hooks, kv, sql, dns, forge (30 ops), docs, jobs
- FoundationDB-inspired unbundled database: stateless layers over KV/blob primitives

**Plugin Architecture:**

- Priority range 900-999 (WASM), 500-899 (native services), 100-499 (core infrastructure)
- KV prefix namespacing: `__plugin:{name}:` auto-scope or explicit `kv_prefixes` in manifest
- Host functions: 23 total (kv ops, blob ops, timers, hooks, sql_query, service_execute, random, signing, cluster info, capabilities)
- API versioning: `PLUGIN_API_VERSION` (currently 0.3.0), `query_host_api_version()`, `host_capabilities()` probe
- Plugin metrics: per-plugin counters (request_count, success/error, duration, active_requests) via AtomicU64
- Hot-reload: graceful drain (wait for active_requests=0, bounded 30s timeout), cancel timers, unsubscribe hooks

**Key Types:**

- `ClientRpcRequest` enum: 100+ variants across all services
- `ClientRpcResponse` enum: feature-flag-sensitive (postcard discriminant mismatch risk)
- `PluginManifest`: name, version, priority, app_id, permissions, kv_prefixes, dependencies, min_api_version
- `HandlerRegistry`: uses `ArcSwap` for hot-reload (`.load()` not field access)
- `PluginHostContext`: permissions, kv_prefixes, timers, subscriptions, service_executors, hook_service

**Raft + NOT_LEADER Flow:**

- KV scan linearizable read requires leadership (ReadIndex)
- Write operations on follower → Raft ForwardToLeader → map_raft_write_error() → KeyValueStoreError::NotLeader
- Handlers check `is_not_leader_error()` → return top-level `ClientRpcResponse::error("NOT_LEADER", ...)`
- Client detects `e.code == "NOT_LEADER"` → rotates to next bootstrap peer → retries
- Multi-peer tickets required for automatic failover (up to MAX_BOOTSTRAP_PEERS=16)

**Git Bridge (git-remote-aspen):**

- Incremental push: three-phase protocol (enumerate SHA-1s → probe server → send missing only)
- `GitBridgeProbeObjects` RPC: read-only (no Raft write), checks `has_sha1()` per hash, bounded 100K max
- Probe graceful degradation: if server doesn't support it, falls back to full push
- Fast path: when all objects already exist on server, uses `GitBridgePush` with empty objects + ref update only
- Adding new RPC variants: add to BOTH `ClientRpcRequest` AND `ClientRpcResponse` enums, update variant_name(), domain(), to_operation(), executor dispatch, HANDLES list, and tests (handles_count + git_bridge_ops)
- Four repos touched for new RPC: aspen-forge-protocol (response type), aspen-client-api (req/resp variants + auth ops), aspen-rpc (handler + executor + client rate-limit), aspen (git-remote-aspen client)

**Pre-Existing Issues (not blockers):**

- aspen-nix-cache-gateway: h3-iroh 0.96 vs iroh 0.95.1 mismatch (excluded from default builds via default-members)
- shellcheck warnings on scripts/ (not from our changes)

**Testing:**

- 1,781+ unit tests across workspace + sibling repos
- 18 NixOS VM integration tests (10 non-plugin + 8 WASM plugin tests)
- Coverage: ~48% workspace average (aspen-client 57%, aspen-blob 53%, aspen-transport 65%)

**CI Worker Cache Integration:**

- `RpcCacheIndex` in aspen-client implements `CacheIndex` trait via RPC (CacheQuery/CacheStats)
- Feature-gated: `aspen-client/cache-index` (pulls aspen-cache + async-trait)
- `ci-basic` feature activates `aspen-client/cache-index` automatically
- Worker fetches cache public key via `SecretsNixCacheGetPublicKey` RPC at startup
- Gateway selection: Ping probe → first responder (fallback: first bootstrap peer)
- Cache substituter auto-enabled when public key available, gracefully disabled otherwise
- Env vars: `ASPEN_CACHE_NAME` (default: "aspen-cache"), `ASPEN_TRANSIT_MOUNT` (default: "transit")

**Cross-Repo Dependency Patterns:**

- aspen-rpc → aspen-ci, aspen-nix, aspen-coordination, aspen-forge, aspen-secrets, aspen-docs, aspen-jobs, aspen-hooks (ServiceExecutor impls)
- aspen-cluster → aspen-cluster-bridges, aspen-sharding, aspen-federation (optional features)
- aspen-client → aspen-core, aspen-client-api, aspen-auth, aspen-blob, aspen-transport (all via workspace deps)
- All plugin crates → aspen-plugin-api, aspen-wasm-guest-sdk (git deps)
