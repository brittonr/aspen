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

### Feature Gate Rules

| Rule | Why |
|------|-----|
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
| 2026-03-25 | `--timeout` on DeployArgs conflicted with global `--timeout` (RPC timeout) | Use `--deploy-timeout` (long = "deploy-timeout") for deploy-specific timeout. Global `--timeout` is `timeout_ms` |
| 2026-03-25 | Dogfood script `do_deploy` reimplemented rolling deploy in ~200 lines of bash | Use `cli cluster deploy --wait --deploy-timeout N` — let `DeploymentCoordinator` handle node ordering, quorum safety, drain, health |

### Iroh / Networking

| Date | Issue | Resolution |
|------|-------|------------|
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

## Investigation Items

All resolved as of 2026-03-24.
