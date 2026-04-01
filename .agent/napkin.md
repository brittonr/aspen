# Napkin

## Corrections

### Rust / Cargo Patterns

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-24 | unit2nix `mkTestBuiltByPkgs` applied `includeDevDeps=true` to ALL workspace members in one shared fixpoint — dev-dep cycles caused infinite recursion | `mkTestGraphForCrate`: per-crate test graphs where only the target crate gets dev-deps (matches `cargo test -p` behavior) |
| 2026-03-23 | `cargo build -p aspen` doesn't rebuild `aspen-node` without features | Always `cargo build --features jobs,docs,blob,hooks,automerge --bin aspen-node` or `--features full` |
| 2026-02-26 | `aspen-client-api` features defaulted to off → postcard enum discriminants shifted → deserialization crash | All `aspen-client-api` features default-on. Wire format enum layout must be identical between all consumers |
| 2026-02-26 | `PluginReloadResult` placed AFTER `#[cfg(feature)]` variants — discriminant shifted | All non-gated variants BEFORE feature-gated section. Golden-file discriminant tests to catch reordering |
| 2026-02-26 | Snapshot race: `LogsSinceLast(100)` triggers during eager apply → panic | `LogsSinceLast(10_000)` + track `confirmed_last_applied` separately in `apply()` callback |
| 2026-02-24 | WASM plugin host function externs used Rust types (String, Vec) | All host externs MUST use raw C types (`*const c_char`, `*const u8`, `i32`). Vec params need explicit `_len: i32` |
| 2026-02-23 | Added field to widely-used struct → 40 broken callers | Add default trait methods (purely additive). When adding fields: `rg 'StructName\s*\{' --type rust -l` to find ALL sites |

### Struct / API Gotchas

| What | Correct Usage |
|------|---------------|
| `WriteRequest` | Has `command: WriteCommand` field, not `key`/`value`. Use `WriteRequest::set(key, value_string)` |
| `ReadRequest` | Has `consistency` field. Use `ReadRequest::new(key)` constructor |
| `ScanRequest` | `limit_results: Option<u32>`, `continuation_token`. No `Default` impl |
| `KV values` | `KeyValueWithRevision.value` is `String`, not `Vec<u8>` |
| `DeterministicKeyValueStore` | `.new()` returns `Arc<Self>`. `.read()` returns `Err(NotFound)` for missing keys |
| `RequestHandler::handle()` | Signature is `handle(request, ctx)` — ctx is second parameter |
| `JobId` | Newtype, no `From<String>`. Use `serde_json::from_value(json!("id"))` |
| `DummyBuildService` | Unit-like struct — must use `DummyBuildService {}` (Rust 2024 strict) |
| `Membership::get_node()` | Not `get()`. Check openraft API with rg before writing |
| `EndpointAddr.addrs` | `BTreeSet<TransportAddr>` not `Vec<DirectAddr>` (iroh 0.97) |
| `aspen-cli kv set` | Not `kv put` |
| `async-trait` | NOT a workspace dep — each crate specifies `async-trait = "0.1"` directly |
| `tokio::sync::Notify` | Edge-triggered: create `notified()` future BEFORE checking the condition, then `.await`. If you check-then-await, notification between check and await is lost |
| Worker `mark_started` | If it fails, MUST release the queue item back (nack/release_unchanged). Otherwise job is orphaned: dequeued from queue but never started. Fixed in `run_worker_execute_with_handler` |
| 3-node dogfood | QUIC stream contention during heavy git push (33K objects) causes 21-140ms node unreachability blips. ReadIndex quorum confirmations fail → forwarded reads fail → `get_job` returns None → `JobNotFound`. Single-node dogfood doesn't hit this |
| Bash `set -u` + EXIT traps | Local vars go out of scope after function returns, but EXIT trap persists. Use `${var:-}` in cleanup functions called from traps |
| Queue item disposition | On `mark_started` failure: ACK if job is Running or NotFound (another worker has it or it's gone). RELEASE only for transient errors. Releasing Running/NotFound items causes infinite dequeue-release hot loops |
| MAX_SNAPSHOT_SIZE | 100MB too small for CI workloads (33K git objects + build logs). Raised to 512MB. Snapshot size exceeded breaks ReadIndex → cascading read failures |
| Deploy self-connect | iroh QUIC `Endpoint::connect()` rejects self-connections. 1-node deploy failed with "Connecting to ourself is not supported". Fixed: `IrohNodeRpcClient` detects `node_id == source_node_id` and validates artifact locally |
| 3-node CI trigger dedup | Leader receives same RefUpdate twice: once from `notify_local_handler()` (stripped `refs/` prefix) and once from gossip broadcast. Fixed: `recent_triggers` dedup cache keyed on (repo_id, commit_hash) + leader-only processing via `is_leader` callback |
| Pipeline false Success | `check_active_job_statuses` treated `Ok(None)` (job not found) as completed. Under forwarding failures, all jobs appear "not found" → pipeline marked Success. Fixed: treat as incomplete |
| Forwarded RPC truncation | Under QUIC snapshot load, `recv.read_to_end()` returns 0 bytes → postcard "Hit the end of buffer". Fixed: detect empty/truncated responses, retry with fresh connection. 3-node dogfood now completes check+build stages (previously stuck at check) |

### Feature Gate Rules

| Rule | Why |
|------|-----|
| After touching feature-gated code, test WITHOUT the feature too | CI clippy runs workspace clippy without features — dead code, unused imports, missing Ok() wrappers surface only in the no-feature path |
| `#[allow(dead_code)]` for structs used in both cfg paths, `#[cfg]` for functions only used in one | Struct behind `#[cfg(feature)]` breaks the `#[cfg(not(feature))]` fallback that uses `::default()` |
| `#[cfg(feature = "ci")]` checks ROOT crate feature, not transitive | Use `--features ci` not `ci-basic` for binary checks |
| Feature chains must propagate to ALL downstream handler crates | `ci = ["dep:aspen-ci-handler", "aspen-ci-handler/forge", "aspen-ci-handler/blob"]` |
| When adding `#[cfg(feature)]` guards, verify feature is defined | Check `[features]` table AND propagation from parent crates |
| After touching feature-gated code, `cargo check --features <each-combo>` | Feature combinations create different type signatures |
| `cargo nextest run` (no flags) only runs root package (813 tests) | Use `--workspace` or `-p <crate>` to test non-root crates |

### Nix Build System

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-24 | nix-compat stub chain: `aspen-rpc-core → aspen-ci → aspen-cache → nix-compat` pulled stub into 12 handler crates | Move unused deps to `[dev-dependencies]`. Make deps optional behind same feature they're used in |
| 2026-03-14 | Nix IFD caching: changing derivation script doesn't invalidate store output | Use completely separate derivation path (different name/inputs) instead of modifying existing one |
| 2026-03-12 | unit2nix `buildRustCrate` ignores `required-features` on `[[test]]`; `nativeBuildInputs` only affects build, not test runner; `CARGO_BIN_EXE_*` not available | Exclude affected crates from per-crate tests; they still run under crane nextest |
| 2026-03-08 | crane `buildDepsOnly` content-addresses from Cargo.toml/Cargo.lock — ciSrc script changes don't change deps drv | Add explicit `cargoLock = ciSrc + "/aspen/Cargo.lock"` |
| 2026-03-06 | Nix sandbox tmpfs disk full kills redb tests | `ci-nix` nextest profile excludes `test(/test_redb/)`. `writableStoreUseTmpfs = false` + large `diskSize` for builds |
| 2026-03-06 | `build-plan.json` stale after adding workspace members | Always `nix run .#generate-build-plan` after changes. Now using auto mode (IFD) |
| general | New .nix files not visible to nix eval | Always `git add` new files before `nix eval`/`nix build` |
| general | `nix-collect-garbage -d` freed only 3.8GB — `.direnv/` GC roots | `find ~ -name '.direnv' -type d -exec rm -rf {} +` first, then GC |
| general | Nix SQLite cache corrupts when disk fills | `rm -f ~/.cache/nix/fetcher-cache-v4*` to recover |

### NixOS VM Test Rules

| Rule | Why |
|------|-----|
| `systemd-run` transient units don't inherit PATH | Use absolute nix store paths: `${pkg}/bin/name` |
| `succeed("cmd &")` hangs | Use `systemd-run --unit=name command` for background processes |
| `echo ''` in `''...''` nix string = parse error | Use `\|\| true` or `echo ""` (double-quoted) |
| Use `pkgs.writeText` for multi-line configs | Never use bash heredocs inside Python `succeed()` calls |
| `wait_until_succeeds` with 30s CLI timeout | Use `--timeout 5000` for fast-failing health checks |
| `PrivateTmp=true` isolates `/tmp` per service | Use `/root` or `/var/lib` for shared paths |
| Use VM clock (`date +%s`) not Python `time.time()` | Clocks differ between host and guest |
| WASM plugins need bare-metal KVM | Make plugin count assertions non-fatal; test by actually using plugins |
| VM `nix build` needs `writableStoreUseTmpfs = false` + 20GB+ disk | Default tmpfs limits to ~50% RAM |
| `nix.settings.experimental-features = ["nix-command" "flakes"]` | Required for `nix build` in VMs |

### Federation Dogfood

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-27 | `federation sync` CLI takes `--peer` (named flag), not positional | Check clap args: `rg "struct SyncArgs" -A 10` before scripting |
| 2026-03-27 | `kv scan` takes prefix as positional, not `--prefix` | CLI prefix is positional: `kv scan "prefix:"` |
| 2026-03-27 | `ci-aspen-node-snix-build` missing `federation` feature → ALPN handshake fail | Add `federation` to features when using federation sync protocol |
| 2026-03-27 | Federation cluster key ≠ iroh secret key → `FederatedId` mismatch → FEDERATION_DISABLED | Federation cluster key MUST match iroh secret key (NixOS tests already do this) |
| 2026-03-27 | `git-remote-aspen` federated clone path (`fed:` URL) silently returns empty repo | Fixed: `federation_import_objects` now uses `import_objects()` (plural) with topological sort instead of sequential `import_object()`. Objects arriving in non-dependency order (commit before tree) no longer fail silently. |

### Deploy / Dogfood

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-28 | Leader ReadIndex quorum timeout during heavy git push → self-forwarding attempt → iroh QUIC self-connect crash | Guard all 6 forwarding sites with `leader_id != self.node_id()`. Add ReadIndex retry (3 attempts, 50ms backoff) when leader hint is self. Belt-and-suspenders endpoint ID check in `IrohWriteForwarder::get_connection()` |
| 2026-03-28 | `block_on()` inside tokio runtime in `with_peer_cache_dir` — panics on startup | Use `Arc::get_mut()` + `RwLock::get_mut()` for sync access during builder phase when Arc isn't shared yet |
| 2026-03-28 | NixOS `aspen-init-profile` only set profile on first boot (`if [ ! -L ]`) — deploys kept running stale binary | Compare `readlink -f` of profile target vs `cfg.package` and update when they differ |
| 2026-03-28 | `pureSrc`/`pureCargoVendorDir` missing subwayrat handling — Nix sandbox can't fetch git dep | Add `/subwayrat/b` to sed keep-list AND `isSubwayrat` override in `overrideVendorGitCheckout` (match other vendor dirs) |
| 2026-03-28 | 1-node dogfood deploy fails: "Connecting to ourself is not supported" | DeploymentCoordinator uses iroh QUIC to push updates — can't connect to self. Use 3-node cluster (`ASPEN_NODE_COUNT=3`) or skip deploy stage |
| 2026-03-25 | `--timeout` on DeployArgs conflicted with global `--timeout` (RPC timeout) | Use `--deploy-timeout` (long = "deploy-timeout") for deploy-specific timeout. Global `--timeout` is `timeout_ms` |
| 2026-03-25 | Dogfood script `do_deploy` reimplemented rolling deploy in ~200 lines of bash | Use `cli cluster deploy --wait --deploy-timeout N` — let `DeploymentCoordinator` handle node ordering, quorum safety, drain, health |

### Iroh / Networking

| Date | Issue | Resolution |
|------|-------|------------|
| 2026-03-28 | `--bind-port 7777` accepted but nodes bind to random ports | `merge_iroh_config` was missing `bind_port` — CLI override silently dropped. Fixed: merge `bind_port` when non-zero |
| 2026-03-28 | CLI on follower gets "not leader" for reads (git list, kv scan) | Two fixes: (1) bind_port merge for firewall, (2) follower read forwarding — `read()` and `scan()` now catch `NotLeader` from `read_ensure_consistency`/`scan_ensure_linearizable` and forward to leader via QUIC (same connection cache as write forwarding). Works from any node's own ticket. |
| 2026-03-22 | Connection pool served stale connections after peer restart — election storms | `add_peer()` detects address changes, calls `connection_pool.evict(node_id)` |
| 2026-03-21 | After `systemctl restart`, iroh gets new port, Raft has stale address | Three-layer defense: gossip cache fallback, persistent peer cache, authoritative membership update via `add_learner` |
| 2026-03-21 | openraft randomizes election timeout once → persistent split-votes | Per-node election timeout jitter via `node_id % (range/3)` |
| 2026-03-23 | Worker on follower: `RaftNode::write()` returns ForwardToLeader → stuck | `WriteForwarder` trait + `IrohWriteForwarder` forwards via CLIENT_ALPN QUIC |
| 2026-03-23 | Write batcher bypassed forwarding entirely | flush_batch + write_direct now detect ForwardToLeader and forward. Followers skip batcher |
| 2026-03-23 | `IrohWriteForwarder` only handled Set/Delete | Extended to CAS, SetMulti, SetMultiWithTTL, DeleteMulti, Batch |
| 2026-02-25 | KV reads on followers returned silent "key not found" | Handlers check `is_not_leader_error()` → top-level `Error("NOT_LEADER")`. Client rotates peers |
| 2026-02-26 | Rate limiter fail-closed on StorageUnavailable | Rate limiter fails-OPEN on storage errors (best-effort, not safety-critical) |

### CI / Dogfood

| Date | Issue | Resolution |
|------|-------|------------|
| 2026-03-24 | Rolling-restart test: CLI returned exit 0 on failed add-learner/change-membership; IPv6 regex captured fragments | CLI checks `is_success`, returns non-zero. Test uses IPv4-only regex, retries, verifies voter state |
| 2026-03-21 | `nix flake archive` hangs indefinitely in VMs (no internet) | 120s `tokio::time::timeout`, `kill_on_drop(true)`. Pipeline continues without prefetch |
| 2026-03-14 | CI failure cache returned `Err(InvalidConfig)` for cached flake refs | Changed from blocking to advisory: log warning + continue |
| 2026-03-10 | Visibility timeout 300s < nix build duration 8+ min → ack failed → pipeline stuck | Increased to 3600s. `ack_job()` proceeds to `mark_completed()` regardless of queue ack result |
| 2026-03-09 | Stage-level status always "pending" | Added `compute_stage_status()` that derives from job statuses |
| 2026-03-08 | `VmPool::acquire()` used `mem::forget(permit)` → leaked on drop | Store `OwnedSemaphorePermit` in VM struct. Auto-released on drop |
| 2026-03-07 | `ci logs` always "not found" — empty `run_id` in PipelineContext | Pass `run_id` from created run into `start_pipeline_build_updated_context` |
| 2026-03-12 | Deploy executor took `output_paths[0]` = man pages | `select_primary_output()` prefers paths without `-man`/`-doc`/`-dev` suffixes |

## User Preferences

- delegate_task works for file edits (pi bug fixed 2026-03-10). Use for larger autonomous tasks, direct edits for surgical changes.
- CLI parse tests: check actual clap subcommand names first. Use `grep -A10 "pub enum.*Command"`.
- Backwards compatibility is not a concern; prioritize clean solutions.

## Patterns That Work

**General Rust:**

- `rg "pub struct TypeName" crates/ -A 20` before constructing ANY struct
- `rg 'StructName\s*\{' --type rust -l` before adding fields to structs
- `rg "enum TypeName" src/` before writing match arms
- Check trait definitions for `#[async_trait]` annotation before implementing
- `Path::starts_with()` matches whole components — use `.to_string_lossy().starts_with()` for prefix matching
- `age::secrecy::ExposeSecret` + `.expose_secret()` for age identity strings

**Nix Build:**

- `ciSrc` (pure eval, stubs optional plugin) for CI checks; `ciPluginsSrc` (impure) for plugin tests
- `ciVmTestBin { features = [...]; }` / `ciVmTestCliBin ["ci" "forge"]` for VM test binaries
- unit2nix auto mode (IFD) — 68/80 crates with per-crate `test.check.<name>` in flake checks
- `overrideVendorGitCheckout` + `snix-src` flake input for real snix deps
- `noLocked = true` when stripping external deps from manifest
- `skipStalenessCheck = true` for all `buildFromUnitGraph` calls

**NixOS VM Tests:**

- `skipLint = true; skipTypeCheck = true;` for complex Python test scripts
- Delete cluster-ticket.txt before systemd restart (stale ports)
- Restart nodes one at a time with health check between
- `memorySize = 4096` for hyperlight; per-node resource config for multi-VM
- `RUST_LOG=info,aspen_raft_network=warn,aspen_rpc_core=warn` to suppress heartbeat spam
- `NO_COLOR=1` when log output will be parsed by scripts

**WASM Plugin System:**

- Three-tier dispatch: native `RequestHandler` → `ServiceExecutor` → WASM `AspenPlugin`
- `PluginPermissions::all()` in test host contexts (default is deny-all)
- Always `call_init().await` after `load_wasm_handler()` before dispatching
- AOT precompile + `with_guest_input_buffer_size(aot_bytes.len() + 128KB)` + output buffer 256KB
- `test = false` + `doctest = false` in cdylib Cargo.toml; add `tests/smoke.rs` for nextest

**Self-Hosting Pipeline:**

- Full loop: `nix run .#dogfood-local -- full-loop` (~11 min pipeline + 2 min verify)
- 3-node: `nix build .#dogfood-serial-multinode-vm`
- Feature chain: root `ci` → `ci-basic` → `nix-executor` + `aspen-ci/nix-executor`
- `.aspen/ci.ncl` is the real CI config; `type = 'nix` for builds, `type = 'shell` for checks
- `json.JSONDecoder().raw_decode()` for CLI output parsing (tolerates trailing stderr)

**VM Serial Testing:**

- `vm_boot` + `vm_serial` for iterative dogfood without NixOS test framework rebuild
- Boot milestones: `"Welcome to NixOS"` → `"login:"` → `"root@dogfood"` (auto-login)
- Disk image from nix store is read-only — `cp` + `chmod +w` before `vm_boot`
- vm_boot defaults to UEFI — use systemd-boot, not BIOS GRUB
- Use `"root@dogfood"` as prompt, not `"[#$] "` (ANSI escapes break regex)

## Patterns That Don't Work

- `path:` flake inputs for local repos (copies target/ dirs — use `git+file://`)
- Git worktrees in aspen (external sibling-repo path deps break)
- `workspace = true` for git URL deps (type duplication)
- Plugin reload on Raft follower (KV scan needs ReadIndex leadership)
- `nix flake archive` in VMs without internet (hangs forever without timeout)
- Pre-populating build deps for inner `nix build` (nixpkgs version mismatch)
- `--out-link` in daemon/systemd contexts (requires writable cwd — use `--no-link` + `--print-out-paths`)

## Domain Notes

**Key Architecture:**

- Iroh-only networking (no HTTP). ALPN-based protocol routing.
- Raft consensus for all cluster-wide state. redb unified log + state machine.
- KV prefix namespacing: `__plugin:{name}:` for WASM, `_sys:` for system, `_ci:` for CI
- `ClientRpcResponse::Error` must be matched explicitly before catch-all in every match

**Raft + NOT_LEADER Flow:**

- Write on follower → ForwardToLeader → IrohWriteForwarder → leader via CLIENT_ALPN QUIC
- Batcher skipped on followers (falls through to forwarding path)
- Handlers check `is_not_leader_error()` → top-level `Error("NOT_LEADER")`
- Client detects NOT_LEADER → rotates to next bootstrap peer → retries
- Multi-peer tickets required for automatic failover (up to 16 peers)

**Git Bridge:**

- Incremental push: enumerate SHA-1s → probe server → send missing only
- Chunked push: PushStart → PushChunk × N → PushComplete (session ID keyed)
- Hash mappings batched via `store_batch` + `SetMulti` (chunked to MAX_SETMULTI_KEYS)
- Wire-level tests: `MinimalForgeServer` with `#[ignore]` for CI sandboxes

**Nix Cache Round-Trip:**

- CI build → NAR upload to blob → narinfo in KV → gateway serves → nix substituter fetches
- NAR hash in nix32 encoding (not hex). Signing uses nix32 NarHash in fingerprint.
- `--extra-substituters` + `--extra-trusted-public-keys` (extra- prefix appends, doesn't replace)

**Observability:**

- Metrics: `_sys:metrics:{name}:{ts:020}`. Alert state: Ok → Pending → Firing → Ok
- `AlertEvaluate` takes explicit `now_us` (FCIS). Periodic evaluator on leader only.
- Metric TTL: default 24h, max 7d

**unit2nix Coverage:**

- 68/80 crates. 12 excluded (stubs, sandbox-incompatible, CARGO_BIN_EXE, vendored)
- All excluded crates still tested via crane nextest

### Federation CI Integration

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-27 | Edits to `trigger/service.rs` silently reverted — dogfood-local script does `git stash`/checkout | Always commit changes before running dogfood. Or avoid dogfood during active editing. |
| 2026-03-27 | `cached_execution` field missing in 3 test structs (pre-existing) | When adding fields to `JobConfig`/`LocalExecutorPayload`, rg all test constructors. These are never behind `..Default::default()` |
| 2026-03-27 | `aspen_core::test_support` is `pub(crate)` — not usable from other crates' tests | Use `aspen_testing_core::DeterministicKeyValueStore` instead |
| 2026-03-27 | `tracing::info!` macro holds `&dyn Value` (non-Send) across `.await` inside `task_tracker.spawn` | Compute values before the macro: `let x = foo.read().await.len(); info!(x = x, ...)` |

### 3-Node Dogfood

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-29 | `dependency_graph.mark_running()` on followers returns JobNotFound — dependency tracker is in-memory local state, not replicated through Raft | Return Ok(()) when job not in local graph — it was submitted on another node |
| 2026-03-29 | `nix-env --profile /nix/var/nix/profiles/aspen-node --set` fails with permission denied in dogfood-local | Set `ASPEN_PROFILE_PATH=$CLUSTER_DIR/node$i/nix-profile` and `ASPEN_RESTART_METHOD=execve` |
| 2026-03-29 | execve restart reads `/proc/self/exe` → old nix store path after profile switch | Pass profile-resolved binary path to execve: `profile_path/bin/aspen-node` |
| 2026-03-29 | `ClientRpcResponse::Error` (discriminant 14) not handled in `interpret_write_response` → "unexpected response type" spam + write forwarding failures | Add explicit match arm for `ClientRpcResponse::Error` in write forwarding |
| 2026-03-29 | 656MB snapshots exceed 512MB limit → snapshot transfer rejection between nodes | Raise MAX_SNAPSHOT_SIZE to 1GB |
| 2026-03-29 | Worker error logging at debug level — failures invisible in default log config | Use info level for dequeue errors and mark_started failures |

## Investigation Items

### Federation clone completeness (2026-04-01) — VALIDATED

**Status**: Unit tests confirm the full federation roundtrip works. Three new tests added:

1. `test_federation_roundtrip_different_keys_14_object_dag` — 14-object DAG with worst-case ordering, different origin/mirror secret keys, convergent import loop
2. `test_federation_roundtrip_gpgsig_commit` — GPG-signed commit survives federation path
3. `test_federation_roundtrip_gitlink_submodule` — Gitlink entries (mode 160000) survive federation path

The import → export path is correct at the unit level. `import_objects` sorts into waves (blobs → trees → commits), the convergent loop retries failed objects with newly-available mappings, and `export_commit_dag` BFS walks the mirror's own BLAKE3 DAG correctly.

**End-to-end validated** (2026-04-01): `dogfood-federation -- full` succeeded through federation clone. All 34,645 objects (18 chunks) cloned from Alice → Bob via federation, then pushed to Bob's Forge with 0 skipped. CI trigger failed separately (unrelated to clone completeness).

### Federated clone SHA-1 drift in export (2026-03-30) — FIXED

**Symptom**: `dogfood-federation -- full` fails at federated clone. `git-remote-aspen` receives all 34,051 objects (18 chunks) but git reports `Could not read 9970f375...` (a commit).

**Root cause**: 15 GitHub-signed commits have `gpgsig` multi-line headers between the committer line and the blank line separator. The `parse_git_commit` import parser didn't handle these — it jumped straight from committer to message. The gpgsig was either lost or mixed into the message, so re-exported commits had different content → different SHA-1. The 164 drifted objects = 15 signed commits + trees/commits that transitively reference them.

**Fix**: Added `extra_headers: Vec<(String, String)>` to `CommitObject`. Import parser captures all headers between committer and blank line (gpgsig, mergetag, encoding, etc.) including multi-line continuation (leading space). Export emits them verbatim. Test: `test_gpgsig_commit_roundtrip`.

**Not the cause** (investigated and ruled out): tree entry mode format (`40000` vs `040000` — git stores without leading zero), tree entry sort order (our `git_tree_entry_cmp` matches git exactly across 20K+ trees), author line spacing (zero drift across 2,909 commits).

### Federation / Git Bridge

| Date | What Went Wrong | What To Do Instead |
|------|----------------|-------------------|
| 2026-03-30 | Federation sync have-set dedup only resolved 2% of entries — server re-sent 97% of objects each round | **FIXED** (d0b5a231): envelope BLAKE3 in SyncObject. Verified 2026-03-31: `test_cross_batch_dedup_via_envelope_blake3` confirms zero duplicates across multi-round sync. The 2% figure was from before the fix was applied. |
| 2026-03-30 | "message too large: 21MB > 16MB" killed federation sync at round 7 — batch of 1000 objects with large source files exceeded MAX_MESSAGE_SIZE | Increase MAX_MESSAGE_SIZE 16MB→64MB, reduce batch 5000→2000 objects. Error manifests as "failed to read message length" on client (server rejects before writing response) |
| 2026-03-30 | Federation sync completes (66K objects) but `handle_git_bridge_fetch` returns 0 for HEAD commit | Root cause: 2,902 objects fail to import across all batches ("hash mapping not found" — trees reference blobs from other batches). HEAD commit is among the failed objects → no SHA-1 mapping → git lookup fails. Origin SHA-1 == import SHA-1 for all successfully imported objects (re-serialization doesn't change SHA-1 after all). Fix needed: ensure complete import — either single-pass with all objects, or incremental retry until zero failures |
| 2026-03-29 | Large git objects (>1MB base64) silently dropped from KV during push — only stored in iroh-blobs, which fails on read | Chunked KV storage: split across `forge:obj:{repo}:{hash}:c:{N}` entries with `chunks:N` manifest |
| 2026-03-29 | Federation sync batch size 1000 with max 10 rounds = 10K objects max, repo has 33K+ | Increased to 5000/round, 100 rounds. Still not enough for federated git clone (DAG walk truncation) |
| 2026-03-29 | `do_verify` in dogfood-federation.sh picked clippy job (first successful) instead of build-node | Filter by job name: prefer `build-node` → `nix-build` → `build-*` → first |
| 2026-03-29 | Federated git clone empty for large repos — `federation_import_objects` fails with "hash mapping not found" | Root cause: truncated DAG walk sends trees referencing blobs not in batch. Fixed: two-pass import (blobs first), list-refs split from fetch, c2e index for DAG dedup |
| 2026-03-29 | c2e index (content→envelope hash) only matches 4/33K entries | **Fixed**: Three bugs: (1) have_set contained envelope BLAKE3 hashes but exporter treated as content hashes (hash domain confusion), (2) content hashes differed due to TreeObject sort divergence, (3) TreeObject used Rust str::cmp instead of git's mode-aware sort. Fix: re-key c2e by SHA-1 hex, fix TreeObject sort to match git, preserve commit/tag message bytes through round-trip, change have_set to SHA-1 domain |
| 2026-03-29 | Federation resolver created without git exporter — `sync_objects` used generic KV path instead of DAG walk | Blob store not available at federation init time. c2e index writes moved to import path instead |

### Metrics Crate Patterns

| What | How |
|------|-----|
| `metrics` crate macros | Use full path `metrics::counter!()`, `metrics::histogram!()` — no `use` import needed |
| PrometheusBuilder | Call `install_recorder()` once early at startup. Returns `PrometheusHandle` for `render()` |
| Label values | Must be `&'static str` or `String` — owned strings need `.clone()` for static labels |
| Multiple exporters | Install only one recorder globally. For fan-out (Prometheus + OTLP), use `metrics-util::layers::FanoutBuilder` |
| `variant_name()` | `ClientRpcRequest::variant_name()` returns `&'static str` — safe for metric labels |
| Context construction sites | `TestContextBuilder::build()` in `aspen-rpc-core`, production in `src/bin/aspen_node/setup/client.rs`, testing in `src/node/mod.rs`, forge-web tests |

## Session Log

### 2026-03-28: Dogfood self-forwarding fix

Fixed two bugs blocking dogfood git push on 3-node cluster:

1. **Self-forwarding crash**: 6 forwarding sites (read/write/delete/scan in kv_store, flush/direct_write in write_batcher) lacked `leader_id != self.node_id()` guard. When leader's ReadIndex timed out under load, it tried to forward to itself via iroh QUIC, which hard-rejects self-connections. Added guards to all sites + belt-and-suspenders endpoint ID check in `IrohWriteForwarder::get_connection()`.

2. **Transient ReadIndex failure on leader**: During heavy write load (33k git objects), leader's quorum confirmation times out transiently. Added retry loop (3 attempts, 50/100/150ms backoff) that only triggers when leader hint points to self. Followers still forward immediately.

Results:

- Git push to Forge succeeded (33,446 objects imported)
- Full CI pipeline passed: format-check ✅, clippy ✅, build-node ✅, build-cli ✅, nextest-quick ✅
- Deploy stage: node 2 upgraded healthy, node 3 stuck draining (pre-existing deploy drain timeout)
- Earlier format-check failure was just `flake.nix` needing alejandra formatting (feature-matrix nix check wasn't formatted)

### 2026-03-28: Testing improvements (continued)

Round 2 additions:

- Error injection tests for coordination (rate limiter fail-open, counter/sequence/lock error propagation) — 10 tests
- Feature matrix nix CI check (12 feature combos in flake checks)
- Property tests for raft-network encoding (shard prefix, NTP, EWMA, backoff, health transitions) — 11 tests
- Found and fixed: `calculate_connection_retry_backoff` shift overflow on attempt >= 64, no 60s cap
- Round-trip fuzz target for ClientRpcRequest/ClientRpcResponse serialization idempotency

### 2026-03-28: Wire format golden tests for aspen-client-api

Added comprehensive postcard discriminant stability tests:

- Golden files: `crates/aspen-client-api/tests/golden/{request,response}_discriminants.txt`
- Test file: `crates/aspen-client-api/tests/wire_format_golden.rs`
- 10 new tests: variant counts (334 req, 267 resp), ~130 pinned discriminants across all sections, structural invariants, critical discriminant pins (Error=14, Pong=12, GetHealth=0)
- Gotcha: many response struct fields differ from what you'd guess (e.g., `was_found` not `is_found`, `was_deleted` not `is_success`). Always `rg "pub struct FooResponse"` before constructing.

### 2026-03-29: Fix deploy drain timeout (node 3 stuck draining)

Two bugs causing "node 3 stuck draining" during 3-node dogfood deploy:

1. **Notify race in `execute_drain`**: Classic TOCTOU — `finish_op()` calls `notify_waiters()` between the `in_flight_count()` check and `notified().await`, losing the notification. Drain then waits for the full 30s timeout instead of completing instantly. Fix: create the `Notified` future BEFORE checking the count (standard tokio::sync::Notify pattern).

2. **Missing timeout on QUIC stream ops in `IrohNodeRpcClient::send_rpc`**: Only `connect()` and `read_to_end()` had 30s timeouts. `open_bi()`, `write_all()`, and `finish()` had no timeout — if the target node was under load (full QUIC flow-control window from 33k git objects), these could block indefinitely. The coordinator's status stayed at "Draining" because `send_upgrade` never returned. Fix: single outer timeout around the entire I/O sequence (connect + open_bi + write + read).

Also fixed: pre-existing compilation error in `deploy_rpc_integration.rs` (`handle_node_upgrade` missing `expected_binary` arg).

### 2026-03-30: Federation dogfood — stream-too-long fixed, DAG walk incomplete

Fixed `stream too long` error in federated clone: git-remote-aspen used `MAX_CLIENT_MESSAGE_SIZE` (16MB) for `read_to_end` but a 33K-object federated clone response is much larger. Added `MAX_GIT_FETCH_RESPONSE_SIZE` (256MB).

After fix: federation sync pulls all 33,847 objects (post-sync retry recovers cross-batch failures). But `handle_git_bridge_fetch` on the mirror only exports 7,514 objects — the `export_commit_dag` BFS doesn't walk the complete DAG. Git sees "Could not read 387a1814..." (missing parent commit).

Root cause hypothesis: during federation import retry, many objects get imported but their SHA-1→BLAKE3 mappings may not be fully written. The exporter's BFS relies on `read_object_bytes` which reads from the blob store by BLAKE3 hash, but the cross-references from commits→trees→blobs are via BLAKE3 hashes stored in the signed objects. Some BLAKE3 hashes in the DAG may reference objects not stored in the mirror's blob namespace (wrong repo prefix), or the mapping store has gaps.

Key evidence from bob's logs:

- First batch: 1000 objects → 547 imported, 453 stuck (unresolvable deps)
- Post-sync retry: recovered 33,847 objects (1990 commits + 15675 trees + 16182 blobs)
- But mirror fetch returns only 7,514 objects — BFS terminates early without error
- Git reports missing parent commit → incomplete object graph

### 2026-03-29: Federation dogfood — sync works, federated clone blocked on large repos

Ran `nix run .#dogfood-federation -- full` end-to-end:

**Working**: Two independent clusters (alice + bob) start, alice hosts Forge repo (33,543 objects pushed), federation sync discovers 1 resource (forge:repo, 1 ref), bob trusts alice, CI pipeline runs on bob (clippy ✅, build-node ✅, build-cli ✅, nextest-quick ✅), verify finds binary.

**Fixed (3 bugs)**:

1. Large git objects (15 tree objects >1MB) silently dropped from KV during push. Federation exporter couldn't read them. Fix: chunked KV storage across multiple keys.
2. Federation sync capped at 10×1000=10K objects. Fix: 100×5000=500K.
3. Verify script picked clippy output instead of build-node. Fix: name-based job selection.

**Blocked**: Federated git clone (`fed:` URL) returns empty repo for large repos. The DAG walk exports objects but truncates at batch boundary, producing trees that reference blobs not in the batch. Bob's import fails: "hash mapping not found: {sha1}". Needs either full dependency closure per batch or an import path that tolerates forward references.

### 2026-03-29: 3-node dogfood full-loop — CI pipeline passes, deploy works

Fixed 4 bugs blocking 3-node dogfood:

1. **Deploy permission denied**: dogfood-local.sh didn't set `ASPEN_PROFILE_PATH` or `ASPEN_RESTART_METHOD`. Fix: set per-node profile path and execve restart.

2. **execve restart used old binary**: `/proc/self/exe` points to old nix store path after profile switch. Fix: resolve binary through profile symlink.

3. **Follower workers couldn't execute jobs** (ROOT CAUSE): `dependency_graph.mark_running()` returned JobNotFound — dependency tracker is local in-memory state, not replicated through Raft. Fix: return Ok when job not in local graph.

4. **Write forwarding Error handling + snapshot size**: `ClientRpcResponse::Error` not handled in `interpret_write_response`. MAX_SNAPSHOT_SIZE raised 512MB→1GB.

Results: CI pipeline all green ✅, all 3 nodes upgraded via rolling deploy ✅. Deploy status polling times out (QUIC strained after heavy pipeline) — cosmetic only.

### 2026-03-30: Fix federation clone — gitlink entries dropped during import

Two bugs caused "bad tree object" during federated clone:

1. **Gitlink entries (mode 160000) skipped during tree import**: `parse_git_tree_content` had `continue` for mode 160000 entries (submodules like `cloud-hypervisor`, `iroh-proxy-utils`). The original tree SHA-1 was computed from ALL entries, but the imported TreeObject was missing gitlinks → re-exported tree had different content → different SHA-1. Parent commits referenced the original SHA-1 → git said "bad tree object".

2. **Sort order**: Initially tried treating gitlinks as directories (appending `/` for sort). Wrong — git's `base_name_compare` uses `S_ISDIR()` which is false for mode 160000. Gitlinks get `\0` like regular files. The wrong sort produced different byte ordering → different SHA-1.

Fix: preserve gitlink entries with raw SHA-1 zero-padded to 32 bytes in the `[u8; 32]` hash field. Export uses stored SHA-1 directly. DAG walk skips gitlink entries (external commits).

Results: federated clone of 34,072 objects (18 chunks) succeeded ✅. Push to bob's forge succeeded ✅. CI trigger failed (separate issue — root cause found: `handle_git_bridge_push` never called `announce_ref_update`, so auto-trigger was dead code for all git pushes).

### 2026-03-31: Fix git push CI auto-trigger — missing announce_ref_update

Root cause: `handle_git_bridge_push` in `git_bridge.rs` updated refs but never called `forge_node.announce_ref_update()`. The federation path (`federation.rs:1717`) calls it correctly, but the regular git push path via `git-remote-aspen` never wired it up. Both `dogfood-local.sh` and `dogfood-federation.sh` always fell back to manual `ci run` because the gossip `RefUpdate` announcement was never emitted.

Fix: Added `announce_ref_update` call after each successful ref update in `handle_git_bridge_push`. Also resolves `old_sha1` → blake3 hash for the announcement's `old_hash` field (used by path-based trigger filters). Also set `ASPEN_CI_FEDERATION_CI_ENABLED=true` for bob in `dogfood-federation.sh` so mirror repos can auto-trigger CI.

### 2026-03-31: Fix federation clone completeness — IncompleteDag error + verify_dag_integrity

`export_commit_dag_collect` silently `continue`d past unresolvable BLAKE3 hashes in the BFS walk, orphaning entire subtrees. For the Aspen repo (~34K objects), this truncated federation clones to ~7.5K objects. The silent skip masked the root cause — any import gap was invisible.

Fixes:

1. **IncompleteDag error**: BFS now collects missing hashes and returns `BridgeError::IncompleteDag` instead of silently continuing. The error includes the missing hash list and count of successfully collected objects.
2. **verify_dag_integrity**: New reusable function in `aspen-forge::git::bridge::integrity` that walks from ref heads, verifying all objects are reachable in KV. Replaces 130-line inline BFS in `federation_git.rs`.
3. **Tests**: gpgsig round-trip, mergetag round-trip, 14-object DAG completeness, IncompleteDag error behavior, verify_dag_integrity.
4. **Updated existing test**: `test_remap_missing_entry_skips_gracefully` now expects IncompleteDag instead of silent empty result.

Note: The 7.5K/34K truncation was observed *before* the gitlink/gpgsig fixes (also 3/30). After those fixes, SHA-1 drift is eliminated and the convergent import should succeed completely. The IncompleteDag error makes any remaining gaps visible rather than silent.

### 2026-03-31: Fix deploy status polling timeout — remove premature error bail

`deploy_wait` in the CLI had `MAX_CONSECUTIVE_ERRORS = 12` (60s at 5s interval) which bailed with "lost connection to cluster" even when the deploy actually succeeded. During a rolling deploy, nodes restart — connection failures are expected. After a heavy CI pipeline (33K git objects), QUIC reconnection under load took >60s, triggering the premature bail. The dogfood script treated this as a deploy failure.

Fix: removed the hard consecutive error bail. The `--deploy-timeout` deadline already provides the real timeout. Errors during deploy are logged periodically (every 5th consecutive error) but don't terminate polling. Added backoff: poll interval doubles from 5s to 10s when errors persist, reducing QUIC pressure during node restarts.
