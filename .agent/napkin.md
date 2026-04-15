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
| 2026-04-01 | Added `upstream_cache_config` field to `NixBuildWorkerConfig` (behind `#[cfg(feature = "snix-build")]`) but forgot to add it to the struct literal in `client.rs` | When adding cfg-gated fields: `rg 'StructName\s*\{' --type rust` to find ALL construction sites. Also check re-exports in `lib.rs` match the cfg gates of their modules |
| 2026-04-07 | `cargo nextest run -P quick --workspace` failed before Aspen tests because vendored crates `vendor/iroh-h3-axum` and `vendor/iroh-proxy-utils` still ship tests/examples against older iroh APIs | For Aspen audits, exclude those vendor crates (or avoid `--workspace`) unless you're explicitly updating the vendored iroh stack |
| 2026-04-07 | Generic timeout helpers in federation wrapped both iroh transport errors and `anyhow::Result` wire helpers (`write_message`, `read_message`) | Use `E: Into<anyhow::Error>` for mixed error sources. `E: std::error::Error` breaks on futures that already return `anyhow::Error` |
| 2026-04-07 | Pre-commit `rustfmt` runs repo-wide `nix run .#rustfmt`, so a scoped commit can fail if unrelated unstaged Rust files would also be reformatted | Stash or restore unrelated Rust edits before committing a focused Rust change. If needed, run `nix run .#rustfmt` manually before a scoped `--no-verify` commit |
| 2026-04-08 | `openspec list --json` reported a fake change named `active` | This repo has `openspec/changes/active/<date>-...`; the CLI treats the parent `active/` directory as a change. Inspect the nested dated directory directly instead of assuming `active` is a real change |
| 2026-04-09 | I looked only under `openspec/changes/` and assumed `shamir-cluster-secret` was missing; the real change had already been archived under `openspec/changes/archive/2026-04-04-shamir-cluster-secret` | Before saying a change is missing, search both active and archive trees. Archived OpenSpec changes still matter when planning follow-on work |
| 2026-04-08 | Added `iroh = { workspace = true }` to `crates/aspen-dogfood`, but the workspace root does not expose `iroh` through `[workspace.dependencies]` | Before using `workspace = true`, verify the crate is actually listed under root `[workspace.dependencies]`. Aspen’s top-level package has direct `iroh` deps, but that does not make them inheritable by workspace members |
| 2026-04-08 | Checked OpenSpec tasks and requested done review without durable repo-backed evidence for each checked box | Before claiming a change is done, add `verification.md` + `evidence/` under the change dir and run `scripts/openspec-preflight.sh <change>` so checked tasks, changed files, and saved transcripts line up |
| 2026-04-08 | Claimed validation steps (`bash -n`, preflight) in the summary without saving durable artifacts, and the first preflight only checked paths named in `verification.md` | If a validation step is worth mentioning, capture it under `evidence/`. Preflight should also scan git directly for untracked files, not just trust `verification.md` |
| 2026-04-13 | I assumed `openspec new change <name>` would scaffold proposal/design/tasks/spec placeholders in Aspen, but here it only created `.openspec.yaml` | After creating a change, immediately inspect the new directory. Write `proposal.md`, `design.md`, `tasks.md`, and `specs/.../spec.md` explicitly instead of assuming the CLI made them |
| 2026-04-13 | I let `ExpungeNode` / `ExpungeNodeResult` land in the middle of `ClientRpcRequest` / `ClientRpcResponse` and "fixed" the fallout by updating goldens, which broke pinned `Pong`/`Error` postcard discriminants | Client RPC wire enums are append-only for new non-gated variants. Never shift existing postcard discriminants; move new variants to the end and keep fixed-value tests green |
| 2026-04-15 | I used the `rg` tool with alternation like `a\|b` and the harness shell split it as pipelines/commands | For alternation or shell-sensitive regex, use `bash` with a properly quoted `rg 'a\|b' ...` command instead of the bare `rg` tool |
| 2026-04-15 | I used a narrow `edit` replacement on `tasks.md` and duplicated trailing sections instead of cleanly rewriting the small file | When a short generated doc changes structurally, prefer `write` with the full intended content over partial `edit` patches that can leave stale sections behind |
| 2026-04-13 | I added a `forge` slice check for `aspen-rpc-core` and found the crate's `forge` feature exposed `ForgeNode<aspen_blob::IrohBlobStore, ...>` without pulling in `blob` | When a feature-gated type mentions another optional crate in its public fields, make the feature dependency explicit in `[features]` (here `forge-core = ["dep:aspen-forge", "blob"]`) |
| 2026-04-13 | I renamed the node runtime bundle but left `src/node/mod.rs` and `aspen-cluster` bootstrap code compiled unconditionally against `hooks`/`federation`/`docs`, so the supposed minimal `node-runtime` slice still needed the full app bundle | Keep `node-runtime` as the bootstrap core slice (`aspen-cluster/bootstrap`) and put the old appful combination behind an explicit `node-runtime-apps` bundle. In `src/node/mod.rs`, gate federation and ephemeral-hook wiring with their own features so `cargo check --no-default-features --features node-runtime` stays green |

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
| iroh `Endpoint::connect()` | First connection to a relay-disabled peer takes 5-10s (iroh tries relay first, fails, falls back to direct). Don't set ASPEN_RELAY_DISABLED on the client side. Don't create fresh endpoints per retry — iroh caches failed connection state. Wait 3s after node startup before first connect |
| iroh `endpoint.addr()` | With relay disabled, returns no direct IP addresses. Use `endpoint.bound_sockets()` and convert `0.0.0.0` → `127.0.0.1` to populate addresses for tickets/discovery |
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
| general | New files can be invisible to nix eval/build when the flake source filter only includes git-tracked paths | Always `git add` newly created source files before `nix eval`/`nix build`/`nix run`, not just new `.nix` files |
| 2026-04-12 | `lib.fileset.toSource` included `./crates` recursively without filtering nested `target/` dirs, so `direnv`/`nix develop` tried to copy ~422GB of crate-local build artifacts into the store during eval | Wrap recursive source dirs in a filter that drops `.git`/`.direnv`/`target`/`result` directories before passing them to `fileset.toSource` |
| 2026-04-12 | `lib.fileset.fileFilter` on a directory does NOT prune descendants under excluded directory names; `target/foo` still leaks through via child files | For recursive directory pruning, use `lib.fileset.fromSource (lib.sources.cleanSourceWith { ... })` instead of `fileFilter` |
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
| 2026-04-09 | Shell-parity audit said dogfood federation should tolerate an already-federated repo, but the code only relied on the current handler being idempotent | When an audit names idempotency/parity behavior, codify it in the caller with a unit-tested helper instead of assuming server semantics stay unchanged |
| 2026-04-09 | `repo_root=$(cd ... && pwd)` captured the path twice because `CDPATH` made `cd` print to stdout inside a command substitution | In repo scripts, use `CDPATH=` when deriving paths with `cd ... && pwd` so `repo_root` stays single-line |
| 2026-04-09 | A naive regex+brace Tiger Style audit counted trait method declarations as 1,000-line functions because it grabbed the next `{` in the file | When scanning Rust without a real parser, stop at the first top-level `;` after the signature and only count functions whose signatures reach a body `{` before that semicolon |
| 2026-04-09 | `crates/aspen-raft/src/node/tests.rs` is compiled only with `feature = "testing"`, so a new multi-node trust test did not show up when I ran `cargo test -p aspen-raft --features trust ...` | For Raft node tests under `src/node/tests.rs`, include `testing` in the feature set (`--features trust,testing`) when running targeted tests |
| 2026-04-09 | I filtered `cargo test -p aspen-cluster gossip::discovery::test_calculate_backoff_duration` and matched zero tests because the real names are the individual `test_calculate_backoff_duration_*` functions | For targeted lib tests here, use a plain substring like `cargo test -p aspen-cluster calculate_backoff_duration` instead of guessing a module path |
| 2026-04-09 | In coordination tests I assumed `SequenceCurrent` after `reserve(5)` would report the last reserved value, but it returns the next available value (`6`) | Treat sequence `current()` as the next token to hand out, not the end of the previous reservation |
| 2026-04-09 | I guessed `ClientRpcRequest::ForgeCreateRepo.default_branch` was a `String`; the request field is actually `Option<String>` | Before writing forge request tests, check the request struct/enum fields in `aspen-client-api` instead of inferring from handler code |
| 2026-04-09 | Writing `openspec-preflight.txt` during the preflight command made the evidence file itself untracked, so preflight failed on its own output path | Touch and `git add` the target evidence file before running preflight, or capture elsewhere and move it into the repo after the check |
| 2026-04-12 | `scripts/openspec-preflight.sh` validates that every `verification.md` `Changed file:` path is currently modified/staged; listing already-committed implementation files makes retroactive verification fail | For retroactive OpenSpec verification, keep `Implementation Evidence` limited to the new verification/evidence files in git status, and cite committed source paths under `Task Coverage` / saved diff artifacts instead |
| 2026-04-09 | I queued a pueue task into a made-up `tests` group and it failed immediately because this machine only accepts pre-created groups | Before `pueue_run`, check the existing groups from the error output or use `default`/`aspen` instead of inventing a new group |
| 2026-04-09 | I initially treated a missing local old-epoch share as “trust not initialized”, but a newly added voter can become leader and still coordinate rotation using remote old-member shares | In trust rotation, only require `load_share(old_epoch)` when `self.node_id()` is actually in the old membership derived from old-epoch digests |
| 2026-04-09 | I first reused `old_members.len()` as the reconfiguration threshold, but trust threshold is a K-of-N policy and should default from cluster size when durable threshold config is unavailable | For trust rotation, derive K with `Threshold::default_for_cluster_size(old_members.len() as u32)` instead of requiring every old member share |
| 2026-04-09 | `openspec archive` failed to sync a MODIFIED requirement in `cluster-secret-lifecycle`; the existing main spec was still written in delta-style sections, so the automatic header-based merge did not find the target cleanly | If archive sync fails on a spec header, inspect the current main spec, apply the intended change manually to `openspec/specs/.../spec.md`, then archive with `--skip-specs` after saving evidence of the manual sync |
| 2026-04-09 | `openspec status --change <name>` rejects names that start with a digit, so a date-prefixed active change directory was not addressable as a normal change name | For new changes that need normal CLI flow, create them under `openspec/changes/<name>` with a letter-prefixed name. If the repo uses `openspec/changes/active/<date>-...`, operate on the path directly instead of assuming the CLI accepts the dated name |
| 2026-04-09 | Harness metadata freshness was only checked on `export`/`check`, so `list`/`run` and flake eval could consume stale `test-harness/generated/inventory.json` | Generated metadata needs consumer-side freshness gates too: shell entry points should run `scripts/test-harness.sh check`, `flake.nix` should reject stale inventory during eval, and saved evidence should exercise those consumer boundaries instead of only the helper function |
| 2026-04-10 | Fresh trust learners replay the historical `TrustInitialize` log entry even though they were not initial members; treating the missing epoch-1 local share as fatal shut the node down before it could apply the later rotation | `apply_trust_initialize_in_txn` must accept “no local share for this node” and still persist digests/members/threshold so later-joined voters can converge and then pick up their first real share from `TrustReconfiguration` |
| 2026-04-11 | The first fix for replayed `TrustInitialize` was too broad: it tolerated a missing local share for every payload, including malformed init data for an expected initial member | Scope the exception to the historical-replay case by checking whether the local node appears in `payload.members`; if it does, missing local share is still an error, and add a negative regression test |
| 2026-04-11 | Even after that narrowing, a malformed `TrustInitializePayload` could still carry a local share for a node omitted from `payload.members`, and the code would silently ignore it | Treat “share assigned for non-member” as malformed committed state and reject it with a dedicated regression test |
| 2026-04-11 | `aspen-client-api` unit tests compared postcard discriminants with `bytes[0]`, which breaks once enum indices exceed 127 because postcard uses varint encoding | Decode the full postcard varint before asserting discriminant order/value, and regenerate `tests/golden/*_discriminants.txt` plus variant counts whenever enum variants move |
| 2026-04-11 | `DistributedRateLimiter::reset()` reused `read_state()` for CAS expected data; on a missing or stale-read bucket that synthesized a fake full bucket and made CAS impossible until `MaxRetriesExceeded` | For reset/idempotent rewrites, read the raw stored JSON (linearizable if needed) and on CAS failure retry with the returned `actual` value instead of re-synthesizing expected state |
| 2026-04-11 | `test_rate_limiter_burst_capacity` modeled “single instant” but used a positive refill rate on real wall-clock time, so scheduler jitter could make proptest fail nondeterministically | For burst-only rate limiter properties, avoid practical refill during the test and assert strict model equivalence instead of leaving a one-sided mismatch escape hatch |
| 2026-04-11 | Public config structs can bypass constructor-only invariants; `RateLimiterConfig { refill_rate: 0.0, .. }` slipped past `RateLimiterConfig::new()` and produced bogus retry timings | When invariants matter, validate again at the consumer boundary (`DistributedRateLimiter::new`) or make fields private instead of trusting callers to use helper constructors |
| 2026-04-11 | I added a fake `KeyValueStore` test double in `rate_limiter.rs` and hit E0195 because `aspen-traits::KeyValueStore` is annotated with `#[async_trait]` | When implementing Aspen traits in tests, mirror the trait macro on the impl (`#[async_trait]`) instead of assuming native async-trait syntax will match |
| 2026-04-12 | `KeyManager::rotate_epoch()` rebuilt a fresh `SecretsEncryption` and dropped prior epoch keys, so mixed-epoch reads failed during secrets re-encryption | On at-rest key rotation, copy prior epoch keys into the new `SecretsEncryption` until background re-encryption finishes |
| 2026-04-12 | `trust_current_epoch` stays unset after `TrustInitialize`; a watcher that looked only at `load_current_trust_epoch()` treated epoch 1 as epoch 0 and skipped the first automatic secrets re-encryption | For trust-at-rest watchers, infer epoch 1 from persisted trust digests/shares when `trust_current_epoch` is still `None` |
| 2026-04-12 | Reopening a Redb-backed `RaftNode` database path inside same test process failed with `DatabaseAlreadyOpen` because the old Raft runtime still held the storage open | For restart semantics in node tests, exercise startup reconciliation logic directly instead of trying to reopen the same Redb file in-process |
| 2026-04-12 | `NodeBuilder::start()` always wires a trust share client when `trust` feature is compiled, so it cannot represent "trust feature compiled but runtime trust absent" | Verify no-runtime-trust behavior through the secrets-service setup helper, not through a bootstrapped node |
| 2026-04-12 | `InitClusterWithTrust` was missing from `handle_client_request_is_bootstrap()`, so `aspen-cli cluster init --trust` was rejected before cluster initialization and trust-only VM tests failed immediately | Treat `ClientRpcRequest::InitClusterWithTrust { .. }` as both bootstrap and rate-limit-exempt in `crates/aspen-rpc-handlers/src/client.rs`, and add a unit test for the classification |
| 2026-04-12 | Fire-and-forget trust expungement over QUIC was dropping the connection too early; `send.finish()` alone let the peer accept the connection but miss the stream payload | In `crates/aspen-raft/src/trust_share_client.rs`, wait for `send.stopped()` (or an application-level ack) before dropping the connection, and cover it with a real-endpoint test |
| 2026-04-12 | Offline-expunged nodes can restart with stale trust metadata and never issue a trust request on their own, so peer enforcement must actively probe during startup | Run `ensure_startup_trust_peer_probe()` synchronously from `create_raft_node()` before boot continues. A delayed background task leaves a serving window before `ExpungedByPeer` is persisted |
| 2026-04-12 | I first treated validated shares against local digests as enough to clear startup, but that still trusted stale local epoch state | In startup probe, require peer-advertised `ShareResponse.current_epoch` to match the probed epoch before counting a confirmation. Local digests validate share bytes, not peer freshness |
| 2026-04-12 | I claimed "working tree clean" right after a `--no-verify` commit without capturing post-commit `git status` | After any commit used as evidence, run and record `git status --short` (and ideally let hooks pass normally). Don't claim clean/hook-verified state without the output |
| 2026-04-13 | Plain `cargo dylint` under Aspen's nix-managed nightly failed because Dylint wanted a real toolchain triple, `dylint-link` needed `RUSTUP_TOOLCHAIN`, and `dylint_driver` needed a crates-io patch to the fixed git rev | Use `scripts/tigerstyle-check.sh` for Aspen Tiger Style rollout. It wraps `rustup`/`cargo`/`dylint-link`, patches `dylint_driver` through a temporary `CARGO_HOME`, and defaults to a low-noise `--lib --no-deps` pilot scope |
| 2026-04-13 | Re-ran `scripts/tigerstyle-check.sh` inside `nix develop`, which swapped in a toolchain missing `rustc-dev` and broke Dylint with `can't find crate for rustc_driver` | Run Aspen Tiger Style pilot checks directly from the normal shell; let `scripts/tigerstyle-check.sh` manage toolchain wrapping itself |
| 2026-04-15 | `scripts/tigerstyle-check.sh` now fails even from the normal shell on this machine because `cargo`/`rustc` come from Nix nightly `2026-02-06`, `~/.cargo/bin` has `cargo-dylint`/`dylint-link` but no `rustup`, and the wrapper does not provision tigerstyle's `nightly-2026-03-21` `rustc-dev` toolchain | Before more Tiger Style rollout work, repair the script/toolchain bootstrap to verify or install the required rustup nightly/components instead of assuming wrapper env vars are enough |
| 2026-04-15 | First-pass `no_panic` inventory over the pilot lib crates (`aspen-time`, `aspen-hlc`, `aspen-core`, `aspen-coordination`) found raw panic-family matches only in test-only contexts; tigerstyle's latest Aspen owned-package `--all-targets` comparison still shows 3 `no_panic` hits, all in `crates/aspen-coordination/tests/proptest_model_based.rs` | Default `--lib` promotion likely stays clean once the tigerstyle runner works, but widening beyond lib will need either test-context precision or cleanup in that integration test file |
| 2026-04-13 | Buffered-counter regression test waited for spawned flush task with a fixed number of `tokio::task::yield_now()` calls, so the assertion raced the scheduler and flaked | For spawned-task tests, use an explicit completion signal or a bounded timeout/poll loop. Fixed-yield counts are not a correctness boundary |
| 2026-04-13 | I first tried crate attrs and rustflags with `tigerstyle::...` names; source attrs want bare Dylint lint names and command-line tool-lint flags are parsed before the Dylint library loads | In Aspen crates, use bare lint names like `#![warn(ambient_clock)]` plus `#![allow(unknown_lints)]`. Avoid command-line `tigerstyle::...` level flags for rollout control |
| 2026-04-13 | I used `.expect_err(...)` in an `aspen-coordination` test where the `Ok` type (`LockSetGuard<_>`) does not implement `Debug`, so the test crate failed to compile | When the success type is not `Debug`, use an explicit `match` to extract the error instead of `expect_err(...)` |
| 2026-04-13 | The Tiger Style `no_panic` pre-expansion lint still surfaced a `panic!` inside `crates/aspen-coordination/src/lockset.rs`'s `#[cfg(test)]` module during `scripts/tigerstyle-check.sh -p aspen-coordination -- --lib` | For pilot-clean reruns, remember pre-expansion lints can still see test-only macro calls in the same source file; avoid panic-family macros there too |
| 2026-04-13 | Running `scripts/tigerstyle-check.sh` dirtied `Cargo.lock` with a trailing `[[patch.unused]] dylint_driver` entry from the temporary Dylint patch config | After Tiger Style pilot runs, check `git diff -- Cargo.lock`; if only `[[patch.unused]] dylint_driver` changed, restore it with `git checkout -- Cargo.lock` |
| 2026-04-13 | Full-workspace pre-commit `cargo clippy` currently dies in `hyperlight-wasm`'s build script (`could not run cargo build wasm_runtime: Failed to prepare sysroot`), even though the Tiger Style rollout only touches `aspen-time`, `aspen-hlc`, `aspen-core`, and `aspen-coordination` | For Tiger Style rollout validation, run targeted clippy on the touched crates (`cargo clippy -p aspen-time -p aspen-hlc -p aspen-core -p aspen-coordination --lib -- -D warnings`) and do not treat the unrelated Hyperlight sysroot failure as signal about these changes |
| 2026-04-13 | Tried plain `git commit` for an OpenSpec-only archive and local pre-commit failed because `markdownlint` was not on PATH outside the dev shell | For commits that need hooks, run them from `nix develop` so hook tools like `markdownlint` are available |
| 2026-04-13 | `aspen-coordination` had 21 ambient-clock hits spread across barriers, locks, queues, rate limiting, rwlocks, semaphores, and worker routing; fixing them one-by-one would duplicate the same boundary justification everywhere | Isolate coordination timeout/measurement reads in one helper like `crates/aspen-coordination/src/runtime_clock.rs`, then route shell-code `Instant::now()` calls through that boundary and keep the state-transition logic pure |
| 2026-04-13 | `scripts/tigerstyle-audit.py` exited with status 1 on parse errors, but still wrote valid JSON with a populated `parse_errors` list | For Tiger Style audits, always save stdout and inspect the JSON before treating non-zero exit as total failure. The exit code signals parser trouble, not necessarily missing hotspot data |
| 2026-04-13 | I tried repo-wide `openspec validate --specs` while finishing Tiger Style cleanup and hit 182 unrelated spec failures | In Aspen, validate the specific change you are touching; do not assume global `openspec validate --specs` is a clean gate |
| 2026-04-13 | Tiger Style scanner treated Rust lifetimes like char literals and counted doc comments as ambient-time code, so `key_manager.rs:get_if_initialized` looked unterminated and `verified/mod.rs` got a fake `Instant::now()` hit | In lightweight Rust scanners, distinguish `'a` / `'_` lifetimes from `'x'` char literals and strip comments before searching for semantic patterns like ambient time |
| 2026-04-13 | Saved `cargo check -q` transcript for OpenSpec evidence and got an empty file, which looked like missing verification in review | For durable verification artifacts, avoid `-q` or prepend `set -x` so successful commands still leave reviewable output |
| 2026-04-13 | Picked a secrets-at-rest integration test for direct evidence, but local disk-usage guard tripped before the target logic ran | For trust/secrets verification under tight local resource guards, prefer deterministic helper tests plus targeted hotspot scans over heavy Redb integration tests |
| 2026-04-13 | Moved `AppTypeConfig` into leaf crate and hit orphan-rule failures only on impls for `Arc<InMemoryStateMachine>` | Shared openraft type configs can live in a leaf crate, but any trait impl whose `Self` type is external (like `Arc<T>`) still needs a local wrapper such as `InMemoryStateMachineStore` |

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
- Gossip rate limiter tests: use injected-time helpers (`TokenBucket::new_at` / `try_consume_at`, `GossipRateLimiter::new_at` / `check_at`) instead of `sleep` or hidden `Instant::now()` calls
- For Tiger Style `ambient_clock` rollout, keep legitimate wall-clock reads behind explicit boundary helpers (for example `aspen_time::current_time_ms()` or a local helper with `#[allow(ambient_clock, reason = "...")]`) so the rest of the crate stays pure-by-default

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

**Dogfood federation parity:**

- `crates/aspen-dogfood` federation mode currently starts alice+bob and establishes trust, but `cmd_build()` still just waits for CI on bob. The deprecated shell flow also did `FederateRepository` on alice, `FederationSyncPeer` on bob, created a bob-side mirror repo, then pushed federated clone content into bob before waiting for CI.
- The active follow-up is `openspec/changes/active/2026-04-08-port-dogfood-federation-orchestration`; use that as the source of truth instead of assuming `--federation build` already matches `scripts/deprecated/dogfood-federation.sh`.

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

**Testing harness:**

- `aspen-testing` is the intended shared harness layer, but root integration tests still carry a parallel `tests/support/real_cluster.rs` cluster bootstrapper. When improving test infrastructure, audit both paths instead of assuming all real-cluster helpers live under `crates/aspen-testing*`.

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

### 2026-04-01: Dogfood binary local connectivity — iroh QUIC fails without relay

The `aspen-dogfood` binary spawns alice/bob on localhost but the client can't connect. Root cause: iroh `Endpoint::connect()` with `RelayMode::Disabled` and no mDNS fails the QUIC handshake even when direct socket addresses are in the ticket. The `N0` preset may require relay for the initial connection establishment (hole-punching bootstrapping).

Attempted fixes:

1. `--relay-mode disabled` on spawned nodes ✅ (no more IPv6 relay errors)
2. `ASPEN_RELAY_DISABLED=1` env var → `RelayMode::Disabled` in `AspenClient` ✅
3. Still fails: `I/O error: connection closed` on every connect attempt

The NixOS VM tests work because VMs have their own network stack with proper routing. The dogfood binary needs either:

- iroh mDNS re-enabled for localhost discovery (simplest)
- A custom discovery mechanism reading from the filesystem discovery files
- Or the node's `EmptyPreset` approach instead of `presets::N0`

Openspec change created: `fix-dogfood-local-networking`

### 2026-04-01: Dogfood local connectivity fixed

Three issues blocked `aspen-dogfood start` from connecting to locally spawned nodes:

1. **ASPEN_RELAY_DISABLED global env** — dogfood main() set this globally via `unsafe { set_var(...) }`, which propagated to `AspenClient::create_client_endpoint()`. Client endpoint with relay disabled can't complete QUIC signaling to relay-disabled nodes. Fix: only set on child processes via `cmd.env()`.

2. **Health-check timing** — ticket file is written right after router spawns, but iroh's QUIC endpoint needs ~3s to fully stabilize for incoming connections. First connection attempt during this window gets stuck permanently (iroh caches failed connection state). Fix: 3s sleep after ticket appears.

3. **"healthy" vs "reachable"** — `wait_for_healthy` expected `status=healthy` but the node reports `unhealthy` until `InitCluster` is called (which happens AFTER health-check). Fix: accept any successful health response as "reachable".

### 2026-04-01: Federation clone validated — 127-file large repo passes VM test

Expanded `federation-git-clone.nix` with 127-file nested repo (src/{a..z}/{1..4}.txt, docs/{a..j}.md, lib/{a..f}/{x,y}.rs). All files synced correctly through federation. Also found and fixed a mirror name collision bug: `get_or_create_mirror` truncated `fed_id_str` to 24 chars, capturing only the origin key prefix. Two repos from the same origin got identical mirror names. Fixed by hashing the full `fed_id_str` with blake3.
