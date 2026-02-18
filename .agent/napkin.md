# Napkin

## Corrections

| Date   | Source | What Went Wrong    | What To Do Instead |
|--------|--------|--------------------|--------------------|

## User Preferences

- (accumulate here as you learn them)

## Patterns That Work

- Plugin system spans multiple crates: `aspen-plugin-api`, `aspen-wasm-plugin`, `aspen-wasm-guest-sdk`, plus individual plugin crates (`forge`, `hooks`, `service-registry`)
- `docs/planning/` contains architectural planning docs (e.g., `plugin-system.md`)
- Three sandbox backends: Hyperlight micro-VM (`plugins-vm`), hyperlight-wasm (`plugins-wasm`), Cloud Hypervisor full VM (`ci-vm`)
- Job execution also includes local shell and nix build modes
- `crates/aspen-jobs/src/vm_executor/` has the core worker implementations
- `crates/aspen-ci-executor-vm/` has CloudHypervisorWorker
- Feature flags control which backends are compiled in

## Patterns That Don't Work

- (approaches that failed and why)

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
