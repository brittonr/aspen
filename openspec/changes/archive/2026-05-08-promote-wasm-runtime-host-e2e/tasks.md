## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for promoting the WASM runtime-host matrix row. ✅ 15m (started: 2026-05-08T13:35:04Z → completed: 2026-05-08T13:50:04Z)

## Phase 2: Product-path WASM execution

- [x] [serial] Identify or add the product runtime seam that starts Aspen and executes a declared WASM unit through runtime RPC/CLI/orchestration rather than direct runtime-core calls. ✅ Implemented via `JobManager` + `WorkerPool` + blob-backed `WasmComponentWorker` product orchestration.
- [x] [depends:product-runtime-seam] Add a deterministic WASM fixture with declared ABI, entrypoint, resource limits, and capability handles. ✅ Added inline `execute() -> i32` WASM fixture with bounded fuel/memory limits.
- [x] [depends:wasm-fixture] Add the runnable E2E target that starts Aspen, activates/submits the fixture through the product path, waits for completion, and captures product-visible output or receipt evidence. ✅ Added `wasm_product_path_test` receipt assertions for ABI, entrypoint, exit code, marker, and worker attempts.
- [x] [depends:wasm-e2e-target] Add negative guardrail coverage showing runtime-core-only tests and plugin install/reload plumbing cannot satisfy the runtime-host row. ✅ Added invalid-WASM product-path guard and explicit marker distinction test.

## Phase 3: Harness promotion and operator evidence

- [x] [depends:wasm-e2e-target] Replace `test-harness/suites/vm/runtime-host-wasm-gap.ncl` with a runnable WASM runtime-host row carrying explicit host kind, proof level, support status, target, and proof-marker assertions. ✅ Promoted to `runtime-host-wasm-product-path` cargo-nextest row.
- [x] [depends:runtime-host-wasm-row] Regenerate and check `test-harness/generated/inventory.json`. ✅ `scripts/test-harness.sh export && scripts/test-harness.sh check` passed.
- [x] [depends:runtime-host-wasm-row] Update `docs/runtime-host-readiness.md` with the accepted WASM evidence boundary after the runnable target passes. ✅ Added WASM evidence boundary and doc guard.
- [x] [depends:wasm-readiness-doc] Validate OpenSpec, harness metadata, the new runnable target, and whitespace before archive. ✅ Verified with focused WASM product-path test, runtime-host readiness doc guard, harness metadata check, strict OpenSpec validation, rustfmt check, and `git diff --check`.
