# Napkin

## Corrections

| Date | Source | What Went Wrong | What To Do Instead |
|------|--------|----------------|-------------------|
| 2026-02-18 | self | Told user Phase 2 (Capability Advertisement) wasn't done, but it was fully implemented | Check actual code before assessing roadmap status — grep for types/functions mentioned in the plan |
| 2026-02-18 | self | delegate_task worker reported CLI fix success but changes weren't on disk | Do surgical edits directly — delegate_task doesn't persist file writes reliably |

## User Preferences

- User wants to improve plugin system iteratively — lifecycle + hot-reload first

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
