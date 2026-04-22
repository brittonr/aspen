# Napkin - Mistakes and Learnings

## No-std aspen-core baseline (2026-04-21)

**Discovery**: `cargo check -p aspen-cluster` is currently blocked by pre-existing workspace breakage outside the no-std scaffolding slice.

### What To Do

- Treat `cargo check -p aspen-core`, `cargo check -p aspen-core-no-std-smoke`, and the saved compile-slice evidence under `openspec/changes/no-std-aspen-core/evidence/compile-*.txt` as the reliable slice-local rails until the wider workspace breakage is fixed.
- `cargo check -p aspen-cluster` may still die before cluster code because vendored `openraft` currently panics inside `#[since(...)]` proc-macros on this toolchain; do not misattribute that failure to `aspen-core/std` gating.
- `cargo tree -p aspen-core -e features` can keep showing `iroh`/`iroh-base` even after `aspen-core` disables defaults on `aspen-cluster-types` if `crates/aspen-traits` still depends on `aspen-cluster-types` with defaults. Boundary cleanup must fix both the leaf crate feature gate and every transitive re-export path.
- After the cluster-types cleanup, if `scripts/check-aspen-core-no-std-boundary.py` reports only `rand`, `rand_core`, and `getrandom` as unexpected plus `rand` as denylisted, remaining work is concentrated in `aspen-hlc -> uhlc`, not the rest of the alloc-only graph.
- Fix for that leak lives in `vendor/uhlc/`: make `rand` optional, keep it in upstream-like defaults, and let `aspen-hlc` depend on `uhlc` with `default-features = false`. Re-check both `cargo tree -p aspen-core --no-default-features -e normal` and the boundary checker after any future `uhlc` update.
- The vendored `openraft-macros` `since` attribute can panic on this toolchain before Aspen code even builds. If `cargo check -p aspen-cluster` or `cargo check -p aspen-cli` fails inside `#[since(...)]`, inspect `openraft/macros/src/utils.rs::is_doc()` first: the debug assertions on non-doc tokens are the trigger.
- After the macro panic is gone, the next cluster/cli blockers may still be unrelated parser breakage elsewhere. This session found duplicated/half-merged code in `crates/aspen-coordination-protocol/src/lib.rs`, `crates/aspen-jobs-protocol/src/lib.rs`, and `crates/aspen-dag/src/sync.rs`; fix those before blaming `aspen-core` feature gating.
- If you split std helpers into `aspen-core-shell`, alias that package under dependency key `aspen-core` in shell consumers (`aspen-core = { package = "aspen-core-shell", ... }`). That preserves existing `aspen_core::*` import paths while keeping real `aspen-core` alloc-only.
- Highest-ROI follow-up seam after `aspen-core-shell` / `aspen-auth-core` / `aspen-hooks-ticket`: split `crates/aspen-storage-types` so alloc-only `KvEntry` leaves the foundational graph while `redb::TableDefinition` (`SM_KV_TABLE`) moves to a shell/storage crate. Today `aspen-core` still pulls `redb`/`libc` only because it re-exports that table definition.
- Next low-tier seam after storage: make `crates/aspen-traits` genuinely alloc-safe (`alloc::sync::Arc` or shell-only blanket impls) so `aspen-core` no longer relies on a std-bound trait leaf.
- Best next wire-layer seam after those: convert `crates/aspen-client-api` to alloc-only (`alloc::collections::BTreeMap`, tests off `postcard::to_allocvec`) and demote `serde_json` in `aspen-{forge,jobs,coordination}-protocol` to dev-dependencies. That unlocks portable client/wire crates without touching runtime behavior.
- Representative rails for the split are now `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core-no-std-smoke`, `cargo check -p aspen-core-shell --features layer,global-discovery,sql`, `cargo test -p aspen-core --test ui`, `cargo check -p aspen-cluster`, `cargo check -p aspen-cli`, `cargo check -p aspen-rpc-handlers`, and `cargo check -p aspen --no-default-features --features node-runtime`.
- Do not summarize old verification rails as "green" unless this transcript ran them or you cite the saved archive evidence path explicitly. For this change, durable proof lives under `openspec/changes/archive/2026-04-21-no-std-aspen-core/`, while the leftover active dir `openspec/changes/no-std-aspen-core/` only carries copied UI evidence.
- OpenSpec typed tasks reject suffixed IDs like `I1a` / `V2b`. For decomposed work, either use additional plain numeric tasks (`I6`, `V7`, etc.) or keep one typed task plus indented bullet substeps.
- If you forget to capture a baseline before editing an uncommitted OpenSpec seam, recover it from `HEAD` with a temporary git worktree instead of pretending the modified tree is the baseline.
- I accidentally ran a 33s `cargo test -p aspen-client-api` through `bash`; for Aspen compile/test rails that might stretch past ~30s, queue them through `pueue_run` even when they look incremental.
- Low free space can make `edit` truncate large staged OpenSpec files to zero bytes. If that happens, recover the staged content with `git show :path/to/file` before rewriting, and free space quickly with targeted `cargo clean -p ...` instead of nuking the whole workspace cache.
- Baseline-capture clones for Aspen must preserve sibling-repo layout. A `/tmp` git worktree or clone without adjacent `aspen-dns` / `aspen-wasm-plugin` siblings can break workspace path deps, and if those siblings resolve `../aspen` back to the live repo Cargo can hit package-collision lockfile errors. Prefer a standalone baseline clone with explicit sibling symlinks that point back to the baseline snapshot.
- pueue task environments can inherit `CARGO_INCREMENTAL=1`, which makes Aspen's `sccache` wrapper abort in detached baseline clones/worktrees (`sccache: incremental compilation is prohibited`). Unset `CARGO_INCREMENTAL` before long cargo rails in those captures.
- In this shell, `env -u CARGO_INCREMENTAL cargo ...` was still not enough for compile/test rails that hit the `sccache` wrapper. Reliable escape hatch was `env -u CARGO_INCREMENTAL RUSTC_WRAPPER= bash -lc 'cargo ...'`.

## Tigerstyle scope (2026-04-21)

**Discovery**: `cargo tigerstyle check` on Aspen workspace reports vendored `openraft` / `openraft_macros` findings too. Aspen-local cleanup can reach zero Aspen findings while vendored macro lints still remain.

### What To Do

- Filter SARIF/JSON by `crates/aspen-`, `src/`, etc. when user asks for Aspen-only fixes.
- Do not assume workspace summary means Aspen crates still have violations.
- After fixes, rerun and verify remaining findings are only under `openraft/`.
- `cargo tigerstyle check` still exits non-zero when warnings remain, even if the log reaches `Finished checking`; inspect the summary/error lines to distinguish deny-level blockers from warning-only debt.
- Check both `git diff` and `git diff --cached` before claiming nothing changed; Aspen sessions often leave staged-only edits that `git diff` alone hides.

## CI with Forge Web (2026-04-19)

**Discovery**: CI with Aspen Forge Web is **already fully implemented and working**.

### What's Already Working

1. **CI Orchestrator** (`PipelineOrchestrator`) - Created at node startup when `config.ci.is_enabled = true`
2. **Trigger Service** (`TriggerService`) - Auto-triggers CI on ref updates when `config.ci.auto_trigger = true`
3. **Forge Gossip Handler** (`CiTriggerHandler`) - Registered to receive `RefUpdate` announcements from forge
4. **Forge-Web UI** - Has complete CI routes:
   - `/ci` - List all pipelines
   - `/{repo_id}/ci` - List pipelines for repo
   - `/{repo_id}/ci/{run_id}` - Pipeline detail
   - `/{repo_id}/ci/{run_id}/{job_id}` - Job logs
5. **RPC Handlers** - All CI operations available:
   - `CiTriggerPipeline`, `CiGetStatus`, `CiListRuns`, `CiCancelRun`
   - `CiWatchRepo`, `CiUnwatchRepo`
   - `CiListArtifacts`, `CiGetArtifact`
   - `CiGetJobLogs`, `CiSubscribeLogs`, `CiGetJobOutput`

### Integration Points

**Node startup** (`src/bin/aspen_node/setup/client.rs`):
- Lines 195-246: CI orchestrator creation
- Lines 248-326: Trigger service creation with leader check
- Lines 331-346: Register `CiTriggerHandler` with forge gossip

**Forge handler** (`crates/aspen-forge-handler/src/handler/handlers/`):
- `git_bridge.rs`: Emits `RefUpdate` gossip on git push → triggers CI
- `federation.rs`: Emits `RefUpdate` on mirror ref updates → triggers CI (if `federation_ci_enabled`)

**Forge-web** (`crates/aspen-forge-web/src/`):
- `routes.rs`: CI routes at lines 615-767
- `state.rs`: CI client methods at lines 835-995
- `templates.rs`: CI page templates

### To Use CI with Forge Web

```bash
# 1. Start node with CI enabled
cargo run --features "node-runtime-apps,blob,automerge,forge,ci,jobs,shell-worker" \
  --bin aspen-node -- \
  --node-id 1 --cookie my-cluster

# Set env vars for CI config
export ASPEN_CI_ENABLED=true
export ASPEN_CI_AUTO_TRIGGER=true

# 2. Create repo with .aspen/ci.ncl:
# {
#   name = "my-pipeline",
#   stages = [{ name = "build", jobs = [{ name = "test", type = 'shell', command = "echo", args = ["hello"] }] }],
# }

# 3. Watch repo for triggers
aspen-cli ci watch <repo_id>

# 4. Start forge-web
cargo run --bin aspen-forge-web -- --ticket <ticket> --tcp-port 8080

# 5. Browse to http://localhost:8080/ci
```

### Tests

- `cargo test -p aspen-ci --features nickel --lib trigger::service::tests::test_watch_and_unwatch_repo`
- `cargo test -p aspen-forge-web --lib ci`
- `cargo test -p aspen --test ci_integration_test test_ci_trigger_pipeline_rpc`

### Key Insight

The CI system was **already fully integrated** - no work needed. The architecture is:

```
git push → forge git_bridge → RefUpdate gossip → CiTriggerHandler → TriggerService
                                                                       ↓
                                                                ForgeConfigFetcher (reads .aspen/ci.ncl)
                                                                       ↓
                                                                OrchestratorPipelineStarter
                                                                       ↓
                                                                PipelineOrchestrator → aspen-jobs
```

Forge-web connects via `aspen-client` RPC calls to the same CI handlers.

## Hook/auth seam split follow-ups (2026-04-22)

**Discovery**: `aspen-auth` and `aspen-hooks-types` had runtime baggage hidden behind seemingly lightweight public types.

### What To Do

- Treat `crates/aspen-auth-core/` as the portable auth surface. `crates/aspen-auth/` is now shell-only for builder/verifier/HMAC/revocation.
- `aspen-client-api` auth feature should point at `aspen-auth-core`, not shell `aspen-auth`, or it drags `aspen-core-shell` back into wire crates.
- `AspenHookTicket` now lives in `crates/aspen-hooks-ticket/`. If a consumer only needs hook config/event schema, depend on `aspen-hooks-types`; if it needs ticket URLs, use `aspen-hooks-ticket` or `aspen-hooks` re-exports.
- `aspen-client-api` test code uses `postcard::to_stdvec`, so if you tighten postcard features for that crate you must keep `use-std` enabled or rewrite the tests to use alloc-only serializers.

## OpenSpec archive preflight gotcha (2026-04-21)

**Discovery**: Archiving a change can break `scripts/openspec-preflight.sh` if archived docs still point at `openspec/changes/<name>/...`.

### What To Do

- After moving `openspec/changes/<name>` to `openspec/changes/archive/<date>-<name>`, rewrite repo-relative paths in archived `verification.md`, `tasks.md`, and any spec/evidence docs that mention the old change path.
- At minimum, fix `verification.md` `Changed file:` entries; preflight fails immediately if they still point at the old active path.
- If `verification.md` task coverage lines use archive paths, `tasks.md` must use the same text verbatim or preflight reports missing checked task coverage entries.
- Stage the archived tree before rerunning preflight, otherwise it fails on untracked archive files.
