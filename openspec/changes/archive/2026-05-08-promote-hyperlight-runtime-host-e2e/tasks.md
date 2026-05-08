## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for promoting the Hyperlight runtime-host matrix row. ✅ Spec package defines product-path evidence, guardrails, and verification boundary.

## Phase 2: Product-path Hyperlight execution

- [x] [serial] Identify or add the product runtime seam that submits a blob-backed Hyperlight guest as a `vm_execute` job through `JobManager` / `WorkerPool` / registered `HyperlightWorker` or equivalent node worker registration. ✅ Added `hyperlight_product_path_test` product path and stable Hyperlight receipt metadata in `HyperlightWorker`.
- [x] [depends:product-runtime-seam] Add or pin a deterministic Hyperlight guest fixture with declared ABI, entrypoint, resource limits, artifact identity, and bounded output. ✅ Reused committed `examples/vm-jobs/echo-worker` Hyperlight guest fixture and default `cargo hyperlight build --release` path.
- [x] [depends:hyperlight-fixture] Add the runnable E2E target that starts the required Aspen runtime path, submits the fixture through product orchestration, waits for terminal state, and captures product-visible receipt evidence. ✅ Gated ignored test passed on a Hyperlight/KVM-capable host with `ASPEN_HYPERLIGHT_RUNTIME_HOST_EXECUTED`.
- [x] [depends:hyperlight-e2e-target] Add negative guardrail coverage showing worker construction, payload serialization, package builds, ignored/manual examples, and direct worker-only calls cannot satisfy the runtime-host row. ✅ Negative invalid-bytes job reaches `HyperlightWorker` through `WorkerPool` before failing and emits the guardrail marker.

## Phase 3: Harness promotion and operator evidence

- [x] [depends:hyperlight-e2e-target] Replace `test-harness/suites/vm/runtime-host-hyperlight-gap.ncl` with a runnable Hyperlight runtime-host row carrying explicit host kind, proof level, support status, target, prerequisites, and proof-marker assertions. ✅ Promoted to `runtime-host-hyperlight-product-path` gated cargo-nextest row.
- [x] [depends:runtime-host-hyperlight-row] Regenerate and check `test-harness/generated/inventory.json`. ✅ `scripts/test-harness.sh export && scripts/test-harness.sh check` passed.
- [x] [depends:runtime-host-hyperlight-row] Update `docs/runtime-host-readiness.md` with the accepted Hyperlight evidence boundary only after the runnable target passes. ✅ Added Hyperlight evidence boundary and doc guard.
- [x] [depends:hyperlight-readiness-doc] Validate OpenSpec, harness metadata, the runnable target, docs guardrails, and whitespace before archive. ✅ Focused cargo tests, cargo-nextest ignored target, docs guardrail, harness check, OpenSpec validation, and whitespace check passed.
