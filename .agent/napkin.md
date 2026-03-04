# Napkin

## Corrections

| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-27 | self | `aspen-sql` had 14 tests that couldn't compile — missing `layer` feature on `aspen-core` dep | When a crate depends on `aspen-core`, check if it uses feature-gated modules (`layer`, `sql`, `global-discovery`). Always try `cargo nextest run -p <crate>` standalone before assuming zero tests. |
| 2026-02-27 | self | `aspen-nix-handler` showed 0 tests but had 5 behind `cache` feature | Feature-gated test modules are invisible to default `cargo nextest run`. Check `#[cfg(test)]` blocks for feature gates, and test with `--features <feat>` when needed. |
| 2026-02-26 | self | Snapshot race: `LogsSinceLast(100)` triggers openraft snapshot during tests, `snapshot.submitted(100) > apply_progress.submitted(99)` → panic → Raft core dead → all operations return NOT_LEADER | Increase snapshot threshold to `LogsSinceLast(10_000)`. Root cause: state machine eagerly applies during `append()` but openraft tracks via `apply()` callback. TOCTOU race between redb `last_applied` and openraft `apply_progress`. |
| 2026-02-26 | self | `aspen-client-api` features (`ci`, `secrets`, `automerge`) defaulted to off → postcard enum discriminants shifted between CLI and server → "Found a bool that wasn't 0 or 1" deserialization crash | Make ALL `aspen-client-api` features default-on: `default = ["auth", "ci", "secrets", "automerge"]`. Wire format enum layout must be identical between all consumers. |
| 2026-02-26 | self | `set -o pipefail` in test script: CLI returns non-zero for expected errors → pipeline exit code is non-zero even though grep matches | Use `{ $CLI cmd 2>&1 \|\| true; } \| grep ...` pattern to suppress CLI exit code when testing error messages. |
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
| 2026-02-26 | self | `verify blob` cross-node check fails in multi-node cluster (blobs are local, not Raft-replicated → 1/3 nodes have blob, below 50% threshold) | Blob cross-node check must be informational only. Only core ops (add/has/get) determine pass/fail. |
| 2026-02-26 | self | `verify all` fails: DocsHandler not registered when `docs_sync` unavailable → dispatch returns "no handler found" → sanitized to "internal error" by `sanitize_error_for_client()` → didn't match "not enabled"/"disabled" | Server sanitization rewrites "no handler found" to generic "internal error" (code=INTERNAL_ERROR). Must match on BOTH code and sanitized message, not just raw error text. |
| 2026-02-26 | self | Rate limiter fail-closed on StorageUnavailable caused cascading failure — ALL RPC requests blocked when KV backend had transient issues | Rate limiter must fail-OPEN on storage errors (like it already does for NotLeader). Rate limiting is best-effort, not safety-critical. |
| 2026-02-26 | self | `openraft/openraft/src/docs/data/` dir untracked due to global `data/` gitignore rule → nix fileset excluded it → compile error E0583 "file not found for module `data`" | Check `git ls-files` for any dir that exists locally but might be gitignored. Global .gitignore patterns (`data/`, `target/`) can hide source files. Use `git add -f` to override. |
| 2026-02-26 | self | Stubbing `iroh-proxy-utils` in fullSrc then enabling `proxy` feature → unresolved imports at compile time | Don't stub git deps whose features are ENABLED in the build. Only stub deps for features that are OFF. For deps with "requires a lock file" vendoring error: fetch source via `builtins.fetchGit`, copy into fullSrc tree, rewrite git dep to path dep, strip `source = "git+..."` line from Cargo.lock. |
| 2026-02-26 | self | `wasmPluginsSrc` used `rawSrc` (no `crates/`) → WASM plugin build couldn't find `aspen-client-api` | `rawSrc` intentionally excludes `./crates` and `./openraft` for lightweight builds. WASM plugin builds need `fullRawSrc` which includes all workspace crates. |
| 2026-02-26 | self | Regression tests for postcard discriminant stability: HealthResponse/FederationStatusResponse/CacheMigrationStartResultResponse struct fields changed since code was written | Always read struct definitions with `rg "pub struct FooResponse"` before constructing test values. Don't guess field names from memory. |
| 2026-02-26 | self | Tests in private modules (`mod storage_init`, `mod sharding_init`) are not discovered by `cargo test --lib` | Put regression tests in publicly reachable modules (e.g., crate root `lib.rs`) or the constants crate. Compile-time assertions (`const _: ()`) work anywhere regardless of module visibility. |
| 2026-02-26 | self | `PluginReloadResult` variant was placed AFTER `#[cfg(feature = "automerge")]` variants in `ClientRpcResponse` — its discriminant shifted when automerge was toggled | All non-gated variants MUST appear BEFORE the feature-gated section. Add golden-file discriminant tests to catch any reordering. |
| 2026-02-26 | self | Snapshot TOCTOU race: `build_snapshot()` reads eagerly-applied `last_applied` from redb, but openraft's `apply_progress` only advances in `apply()` callback | Track `confirmed_last_applied` separately (updated only in `apply()`), use that in `build_snapshot()`. Keep `LogsSinceLast(10_000)` as defense-in-depth. |
| 2026-02-26 | self | Pong is discriminant 12 (not 13) in ClientRpcResponse — miscounted because Pong is a unit variant between ChangeMembershipResult and ClusterState | Always verify discriminant values empirically with `postcard::to_stdvec()` before pinning in golden tests. Don't count by hand. |
| 2026-02-26 | self | Automerge + Docs CLI commands had `other =>` catch-all without `ClientRpcResponse::Error` match → CAPABILITY_UNAVAILABLE showed raw Debug format "unexpected response: Error(ErrorResponse{...})" instead of clean error | Every match on `ClientRpcResponse` must have explicit `ClientRpcResponse::Error(e)` arm before the catch-all `other =>`. The client normalizes CapabilityUnavailable→Error, so all handlers must match Error. |
| 2026-02-26 | self | Counter/Sequence test grep patterns expected `value\|error\|unavailable` but commands output bare numbers (`0`, `101`) on success | Match actual output format: `^[0-9]+$\|value\|error\|unavailable` to accept both numeric success values and error messages. |
| 2026-02-26 | self | `cargo build --bin aspen-node --bin aspen-cli` fails because they're in different packages | Use `cargo build -p aspen -p aspen-cli` for multi-package builds. `aspen-node` requires features: `--features jobs,docs,blob,hooks,automerge`. |

| 2026-02-27 | self | `SignedObject::new()` wraps with HLC timestamp → importing same git object twice produces different BLAKE3 hashes | Idempotency guarantee is at SHA-1 mapping level, not BLAKE3 level. Import always creates a new SignedObject, but the SHA-1↔BLAKE3 mapping gets overwritten to point to the latest. Tests should verify SHA-1 stability, not BLAKE3 stability. |
| 2026-02-27 | self | Bridge export adds trailing `\n` to commit/tag messages; import strips trailing newlines via `lines().collect().join("\n")` | Test data must include trailing `\n` on commit/tag messages to match what the bridge export produces. The bridge normalizes messages to end with `\n`. |
| 2026-02-27 | self | Wire-level test: `Command::output()` blocks tokio runtime — server can't process requests while client waits | Use `tokio::task::spawn_blocking()` for subprocess calls inside async tests. `timeout()` wrapping a blocking call doesn't actually timeout — it needs to be async-aware. |
| 2026-02-27 | self | Wire-level test: `iroh::Endpoint::bound_sockets()` returns `0.0.0.0` — client can't connect | Convert `0.0.0.0` → `127.0.0.1` in `EndpointAddr.addrs` for loopback tests. `git-remote-aspen` detects local addrs and disables discovery. |
| 2026-02-27 | self | Wire-level test: global push session state stomped by parallel tests | Use per-session-id `HashMap<String, PushSession>` instead of single `Option<PushSession>`. Random session IDs provide isolation. |

| 2026-02-27 | self | `aspen-rpc-handlers` `git-bridge` feature only propagated to `aspen-forge/git-bridge`, NOT `aspen-forge-handler/git-bridge` — server returned GIT_BRIDGE_UNAVAILABLE | Feature chains must propagate to ALL downstream crates that have `#[cfg(feature = ...)]` guards. Check handler/executor crates too, not just core crates. |
| 2026-02-27 | self | `bins.git-remote-aspen` uses `nodeCommonArgs` which has stub source (no `crates/`) — fails to compile with `aspen-auth` not found | Use `fullBin` (from `fullCommonArgs`) for binaries needed in VM tests. Stub-based builds only work for the main node/CLI that don't need real crate implementations. |
| 2026-02-27 | self | `full-aspen-node-plugins` was missing `forge,git-bridge,blob` compile features — native git bridge handler never compiled in | VM test node builds need ALL features exercised by the test. Check what compile-time `#[cfg(feature)]` guards exist in handler code. |
| 2026-02-27 | self | `git push` in VM test: chaining `git remote add && git push` in one `succeed()` call — if push fails, can't distinguish which command failed | Split multi-command shell operations into separate `succeed()` calls. Use `execute()` to capture exit codes for debugging. |

| 2026-02-27 | self | `cargo nextest run` (no flags) only runs root package (813 tests). `cargo nextest run --workspace` runs all 5,722 tests across 72 crates. Federation crate's 88 tests were never being run in default test runs. | Use `--workspace` or `-p <crate>` to test non-root workspace crates. Check `cargo nextest list -p <crate>` to verify test discovery. |
| 2026-02-27 | self | `ForgeNodeRef` type alias = `Arc<ForgeNode<IrohBlobStore, dyn KeyValueStore>>` — can't substitute `InMemoryBlobStore` in tests | Test ForgeNode KV operations directly in `aspen-forge` (accepts generic blob store). Test handler functions that don't need ForgeNode (trust/untrust) separately in `aspen-forge-handler`. |
| 2026-02-27 | self | iroh 0.95 `Endpoint::connect(PublicKey, alpn)` fails with `clear_discovery()` — no addressing info available | Use `Endpoint::connect(EndpointAddr, alpn)` with explicit socket addresses. Convert `0.0.0.0` → `127.0.0.1` from `endpoint.bound_sockets()`. Pattern: `EndpointAddr::new(endpoint.id())` + insert `TransportAddr::Ip(fixed_addr)`. |
| 2026-02-27 | self | iroh 0.95 protocol handlers registered via `Router::builder(endpoint).accept(ALPN, handler).spawn()` — NOT `endpoint.add_protocol()` | Use `Router::builder(endpoint.clone()).accept(alpn_bytes, handler).spawn()`. Keep `Router` alive (it drives the accept loop). |
| 2026-02-27 | self | `sync::wire` module was private — integration tests couldn't access `read_message`/`write_message` for manual handshake | Made `pub mod wire` in `sync/mod.rs`. Federation wire tests need it for direct protocol interaction without going through `connect_to_cluster()`. |

| 2026-02-28 | self | cdylib WASM guest crates (12 in aspen-plugins) SIGSEGV when nextest runs native test binary — FFI exports crash without hyperlight sandbox | Add `test = false` + `doctest = false` to `[lib]` in all cdylib plugin Cargo.toml files. Add `tests/smoke.rs` integration test so nextest still finds ≥1 test binary per crate. |
| 2026-02-28 | self | `cargo nextest run` with 0 test binaries exits with error code — auto-test harness treats as failure | cdylib crates need at least one integration test file (`tests/*.rs`) even if lib tests are disabled. nextest `--no-tests pass` is CLI-only, not configurable via `.config/nextest.toml`. |
| 2026-02-28 | self | Thought `aspen-wasm-plugin` had 0 integration tests — actually has 8 behind `#[ignore]` + `#[cfg(feature = "testing")]` | `cargo nextest run` shows "8 skipped" but doesn't explain why. Use `--run-ignored all` with `--features testing` to run them. All 8 pass with KVM (~8.7s each for AOT compile + hyperlight sandbox boot). |
| 2026-02-28 | self | `vm_executor_test.rs` used manual `Job { ... }` struct literal — broke when `execution_token` field was added | Use `Job::from_spec(job_spec)` instead of manual construction. It handles all fields and stays compatible. |
| 2026-02-28 | self | `vm_integration_test.rs` used `"test_blob_hash"` as BLAKE3 hash — `iroh_blobs::Hash::parse()` panics on non-64-hex-char strings | Always use valid 64-char hex strings for BLAKE3 hashes in tests (e.g., `"0000...0000"`), even when the blob won't exist. |
| 2026-02-28 | self | `load_wasm_handler` test helper didn't AOT precompile or size input buffer — real 1.5MB plugins couldn't load (buffer too small) | Test helpers must match production code: AOT precompile via `precompile_wasm()`, set `with_guest_input_buffer_size(aot_bytes.len() + 128KB)`. Made `precompile_wasm` pub for this. |
| 2026-02-28 | self | WASM plugin integration tests called `handler.handle()` before `call_init()` — handler rejected with "state: Loading" | Always call `handler.call_init().await` after `load_wasm_handler()` before dispatching requests. Plugin lifecycle: Loading → Initializing → Ready. |
| 2026-02-28 | self | `PluginHostContext::new()` defaults to `PluginPermissions::default()` (all denied) — host `kv_get` silently returned error tag `\x03` | Test host contexts need `.with_permissions(PluginPermissions::all())` or explicit per-capability grants. Default is least-privilege (all denied). |
| 2026-02-28 | self | Echo plugin didn't exist — integration tests referenced `aspen_echo_plugin.wasm` that was never built | Created `aspen-echo-plugin` crate in aspen-plugins repo. Handles Ping→Pong, ReadKey→kv_get, else→UNHANDLED error. |

| 2026-03-03 | self | `host.succeed("socat ... &")` hangs — background `&` keeps shell alive in NixOS test `succeed()` | Use `systemd-run --unit=name command` for background processes in NixOS VM tests. Never use `&` in `succeed()`. |
| 2026-03-03 | self | `cluster status` endpoint_id field is full `EndpointAddr { id: PublicKey(hex), addrs: {...} }` debug string — can't pass to CLI as `--endpoint-id` (shell chokes on `{` and `(`) | Extract hex public key with `re.search(r'PublicKey\(([0-9a-f]+)\)', raw_eid)` before passing to CLI. |
| 2026-03-03 | self | `aspen-node-vm-test` is built with `ci,docs,hooks,shell-worker,automerge,secrets` but NOT `net` — NetHandler not compiled in, `net publish` silently fails | Use `full-aspen-node-plugins` for tests that need `net` feature. Always check which features are compiled into the node package. |
| 2026-03-03 | self | WASM KV plugin install fails in nested KVM (hyperlight sandbox inside QEMU) — `plugin list` returns 0 plugins after reload | Native handlers (net, blob, cluster) work without plugins. Only use WASM plugins in tests when the feature actually requires them. `inmemory` storage backend doesn't need KV plugin. |
| 2026-03-03 | self | `fullBin { name = "aspen-net"; }` uses root Cargo.toml pname and `--bin aspen-net` — fails with "no bin target named aspen-net in default-run packages" | For bins in subcrates, use explicit `craneLib.buildPackage` with `--package aspen-net --bin aspen-net` instead of `fullBin`. |
| 2026-03-03 | self | `echo ''` in nix test string triggers alejandra parse error ('' is multiline string delimiter) | Avoid `''` in nix `''...''` strings. Use `echo empty` or `echo ""` (double-quoted) instead. |
| 2026-03-03 | self | Git worktree for aspen has path collision: relative path deps in sibling crates (e.g., `aspen-dns = { path = "../../../aspen-dns/crates/..." }`) resolve differently, causing lockfile package collisions | Don't use git worktrees for development in aspen — the workspace has external sibling-repo path deps that break in worktrees. Work directly in the main repo. |
| 2026-03-03 | self | Rust 2024 edition: `tokio::fs::write(path, &string)` fails type inference — needs explicit `string.as_bytes()` | In closures and async chains, use `.as_bytes()` for `tokio::fs::write` with String data. Also: `map_err(\|e: toml_edit::TomlError\|` needs explicit error type in closures. |
| 2026-03-03 | self | `age::Encryptor::with_recipients()` takes `impl Iterator<Item = &dyn Recipient>`, not `Vec<Box<Recipient>>` | Create vec of `Box<dyn Recipient>`, then pass `recipients.iter().map(\|r\| r.as_ref() as &dyn age::Recipient)` |

## User Preferences

- Improve plugin system iteratively
- For multi-crate changes: do edits directly (delegate_task unreliable for file writes)
- delegate_task for test writing: CLI output tests worked, secrets tests didn't persist. Pattern: delegate works for single-file appends but fails for multi-file edits.
- CLI parse tests: always check actual clap subcommand names (e.g., `status` not `state`, `enqueue` not `push`, `--repo` flag not positional). Use `grep -A10 "pub enum.*Command"` on the command file first.
- `crates/aspen-client/src/rpc_types.rs` is orphaned (156 structs, not compiled). Comment says "included in rpc.rs" but nothing references it.

| 2026-02-27 | self | `kv_store.write()` takes `WriteRequest`, not `(&str, Vec<u8>)` — and `kv_store.delete()` takes `DeleteRequest`, returns `DeleteResult` with `is_deleted` | Always check trait signatures: `KeyValueStore::write(WriteRequest)`, `delete(DeleteRequest)`. Use `WriteRequest::set(key, value_string)`. IndexScanResult `primary_keys` are `Vec<Vec<u8>>`, not `Vec<String>` — hex-encode for wire format. |
| 2026-02-27 | self | `ScanRequest` field is `limit_results` not `limit`, and it doesn't impl `Default` | Always check struct field names and trait impls with rg before using `..Default::default()`. `ScanRequest` has 3 fields: `prefix`, `limit_results`, `continuation_token`. |
| 2026-02-27 | self | KV `KeyValueWithRevision.value` is `String`, not `Vec<u8>` — no need for `from_utf8()` | KV values are stored as String. When storing JSON, use `serde_json::to_string()` + store directly. When reading, `serde_json::from_str(&entry.value)` works. |
| 2026-02-27 | self | `handle_index_drop` used `_sys:index:` prefix but `handle_index_create` stores under `/_sys/index/` (INDEX_METADATA_PREFIX) — keys didn't match | Always use `INDEX_METADATA_PREFIX` from `aspen_core::layer` for index system keys. The canonical format is `/_sys/index/{name}`, not `_sys:index:{name}`. |
| 2026-02-27 | self | `kv_store.read()` takes `ReadRequest` (not `&str`), returns `ReadResult { kv: Option<KeyValueWithRevision> }` — not a direct value | Create a helper like `kv_read_value(ctx, key) -> Option<String>` that wraps the ReadRequest/ReadResult boilerplate. Use `.and_then()` for chained deserialization. |
| 2026-02-27 | self | `super::*` in test modules doesn't re-export `use` items from parent — test module couldn't see `AlertSeverity` etc. | Always add explicit `use aspen_client_api::TypeName` imports in test modules for types used in test code, even if `super::*` is present. |

| 2026-02-27 | self | `QuorumCheckResult` used in `assert_eq!` but missing `PartialEq` derive — test compilation fails | Always derive `PartialEq` on result types used in test assertions. Check all types in `Result<T, E>` — both T and E need `PartialEq`. |
| 2026-02-27 | self | Rust 2024: explicit `ref` in pattern bindings not allowed when implicitly borrowing | Don't use `Some(ref x)` in Rust 2024 — use `Some(x)` instead. The borrow is implicit. |
| 2026-02-27 | self | Adding field to `FederationSettings` breaks 3 constructor methods (`disabled()`, `public()`, `allowlist()`) | When adding fields to structs with constructor methods, update ALL constructors immediately. Use `#[serde(default)]` for backwards-compatible deserialization. |

| 2026-03-03 | self | Background flush timer needs `AspenFs` for KV access but `AspenFs` is moved into `Server<AspenFs>` — can't hold `&AspenFs` in the timer thread | Use `clone_for_kv_access()` to create a lightweight KV-only clone that shares the `Arc`-wrapped backend. Share the `WriteBuffer` via `Arc<WriteBuffer>`. Timer thread holds its own `AspenFs` clone for writes. |

## Patterns That Work

**Workspace Architecture (consolidated — formerly 48 sibling repos):**

- All 70+ crates live under `crates/` in the main workspace
- Only 3 external repos needed: `aspen-wasm-plugin`, `aspen-plugins`, `aspen-wasm-guest-sdk`
- `fullSrc` derivation: `$out/aspen/` (workspace) + `$out/aspen-wasm-plugin/` + `$out/iroh-proxy-utils/` as peers
- Use `postUnpack = 'sourceRoot="$sourceRoot/aspen"'` for crane to enter the subdirectory
- `rawSrc` = lightweight (no crates/, no openraft/) for quick builds; `fullRawSrc` = everything for VM tests
- Git deps that are feature-enabled: fetch source, copy into tree, rewrite to path dep
- Git deps that are feature-disabled: stub with empty crate to avoid vendoring failures
- Strip `source = "git+..."` from Cargo.lock for any dep converted from git to path

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
- Chunked push: PushStart → PushChunk × N → PushComplete. Session state keyed by random session ID. PushStart stores repo_id + ref_updates; PushChunk imports objects; PushComplete applies refs.
- Wire-level test pattern: `MinimalForgeServer` — lightweight iroh QUIC server handling only git bridge RPCs with `ForgeNode<InMemoryBlobStore>`. No Raft, no full handler registry. Tests are `#[ignore]` for CI sandboxes.

**Observability Pipeline:**

- Traces: complete (ingest → KV at `_sys:traces:{trace_id}:{span_id}` → query → CLI)
- Metrics: complete (ingest → KV at `_sys:metrics:{name}:{ts:020}` + metadata at `_sys:metrics_meta:{name}` → query with aggregation → CLI)
- Alerts: complete (rules at `_sys:alerts:rule:{name}`, state at `_sys:alerts:state:{name}`, history at `_sys:alerts:history:{name}:{ts:020}`)
- Alert state machine: Ok → Pending (breached, waiting for_duration) → Firing (breached long enough) → Ok (resolved)
- `AlertEvaluate` takes explicit `now_us` parameter for FCIS/deterministic testing
- `MetricQuery` supports aggregation (avg/sum/min/max/count/last) + time-bucketed downsampling via `step_us`
- Metric TTL: default 24h (`METRIC_DEFAULT_TTL_SECONDS`), max 7d (`METRIC_MAX_TTL_SECONDS`)
- No periodic alert evaluation yet — on-demand only via `AlertEvaluate` RPC

**Pre-Existing Issues (not blockers):**

- aspen-nix-cache-gateway: h3-iroh 0.96 vs iroh 0.95.1 mismatch (excluded from default builds via default-members)
- shellcheck warnings on scripts/ (not from our changes)

**Testing:**

- 1,781+ unit tests across workspace + sibling repos
- 18 NixOS VM integration tests (10 non-plugin + 8 WASM plugin tests)
- 42 aspen-wasm-plugin tests (34 unit + 8 KVM integration via `--run-ignored all --features testing`)
- 28 aspen-plugins tests (16 signing/tooling + 12 cdylib smoke tests)
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

| 2026-03-03 | self | systemd-run `--property=StandardError=file:/tmp/foo.log` buffers and doesn't capture tracing output from Rust binaries | Use `bash -c 'export PATH=...; exec binary 2>/tmp/foo.log'` for log capture, or use `journalctl -u <unit>` to read systemd journal |
| 2026-03-03 | self | Multiple `--property=Environment=VAR=val` in systemd-run — later values overwrite earlier ones | Use `bash -c 'export VAR1=val1 VAR2=val2; exec cmd'` pattern instead of multiple `--setenv` or `--property=Environment` |
| 2026-03-03 | self | aspen-node starts but KV operations return NOT_INITIALIZED — cluster not auto-initialized | Must call `InitCluster` RPC explicitly before KV ops work. `FuseSyncClient::init_cluster()` added for this. The `cluster ticket generated` log is NOT proof of initialization — it just means the ticket was printed. |
| 2026-03-03 | self | CH guest VirtioFS mount blocks NixOS boot when backend is slow (Raft cluster) | Every VirtioFS op is a network roundtrip through iroh QUIC to Raft leader. Guest boot with VirtioFS mount can take 30-60s. Increase curl timeouts to 180s for Raft-backed VirtioFS tests. |
| 2026-03-03 | self | subagent created chunking.rs that called private methods on AspenFs | When designing a module that interacts with a struct's internals, make the required methods `pub(crate)` upfront, or design the API so the module only uses public methods. |
| 2026-03-03 | self | VirtioFS+net test: `aspen-cluster-virtiofs-server` exits 1 because test called `cluster init` before the server, and server's `init_cluster()` returns `Ok(false)` on already-initialized clusters | Don't call `cluster init` via CLI before starting `aspen-cluster-virtiofs-server` — the server does its own `init_cluster()` and exits on `Ok(false)`. Let the server initialize the cluster. |
| 2026-03-03 | self | `pureBin` build failed: new workspace member `aspen-contacts` not in `Cargo.lock` → vendoring fails with "snafu not found in workspace.dependencies" | Always run `cargo generate-lockfile` after adding new workspace members. Check `grep <crate-name> Cargo.lock` before committing. |
| 2026-03-03 | self | `required_app()` match non-exhaustive when `ci`/`automerge` features are OFF — new variants (Contacts, Calendar) always present but feature-gated match arms at the end get removed | Add `#[allow(unreachable_patterns)] _ => None` catch-all at end of `required_app()` to handle any combination of feature flags. |
| 2026-03-03 | self | FlushTimer needs AspenFs for KV writes but AspenFs owns the WriteBuffer — circular reference | Use `clone_for_kv_access()` to create a lightweight AspenFs clone that shares the Arc<KvBackend> but has fresh empty cache/buffer/prefetcher. Timer holds the clone, shares Arc<WriteBuffer> with the primary AspenFs. |
| 2026-03-03 | self | Planned complex KvOps trait refactor across chunking/writeback when a simpler clone_for_kv_access approach works | Before designing a trait abstraction, check if a simpler structural pattern (clone with shared backend) solves the problem. Traits are better when you have genuinely different implementations; clones work when you just need to break a reference cycle. |
| 2026-03-03 | self | `generate_id("contact", &contact.uid)` produced same ID for all contacts without UID — uid defaults to "" | When generating deterministic IDs from optional/empty fields, always include a disambiguator (parent_id + display_name + now_ms) to avoid collisions. Never hash only the empty string. |
| 2026-03-03 | self | iCal test events used raw Unix-ms numbers as DTSTART (e.g. `DTSTART:1700001000000`) — parser expects `YYYYMMDDTHHMMSS` format and returned 0 | Use proper iCal datetime format: `20231114T090000Z` not raw milliseconds. Create `ical_dt(offset_hours)` helper for tests that computes valid iCal datetimes. |
| 2026-03-04 | self | `AspenClient` doesn't implement `Clone` — can't share it across gRPC request handlers | For keyservice: connect a fresh `TransitClient` per-request via `TransitClient::connect()`. For other patterns, use `TransitClient::from_client()` only when you own the `AspenClient`. |
| 2026-03-04 | self | Clippy `enum_variant_names` error on tonic-generated proto code (all variants end in `Key`) | Add `#[allow(clippy::enum_variant_names)]` on the proto include module. Generated code is upstream SOPS proto — can't rename variants. |
| 2026-03-04 | self | SOPS format modules had crypto functions (encrypt_sops_value, decrypt_sops_value) duplicated in toml.rs | Extract format-agnostic crypto to `format/common.rs`, re-export from format modules for backwards compat. |
