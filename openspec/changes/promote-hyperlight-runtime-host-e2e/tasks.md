## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for promoting the Hyperlight runtime-host matrix row. ✅ Spec package defines product-path evidence, guardrails, and verification boundary.

## Phase 2: Product-path Hyperlight execution

- [ ] [serial] Identify or add the product runtime seam that submits a blob-backed Hyperlight guest as a `vm_execute` job through `JobManager` / `WorkerPool` / registered `HyperlightWorker` or equivalent node worker registration.
- [ ] [depends:product-runtime-seam] Add or pin a deterministic Hyperlight guest fixture with declared ABI, entrypoint, resource limits, artifact identity, and bounded output.
- [ ] [depends:hyperlight-fixture] Add the runnable E2E target that starts the required Aspen runtime path, submits the fixture through product orchestration, waits for terminal state, and captures product-visible receipt evidence.
- [ ] [depends:hyperlight-e2e-target] Add negative guardrail coverage showing worker construction, payload serialization, package builds, ignored/manual examples, and direct worker-only calls cannot satisfy the runtime-host row.

## Phase 3: Harness promotion and operator evidence

- [ ] [depends:hyperlight-e2e-target] Replace `test-harness/suites/vm/runtime-host-hyperlight-gap.ncl` with a runnable Hyperlight runtime-host row carrying explicit host kind, proof level, support status, target, prerequisites, and proof-marker assertions.
- [ ] [depends:runtime-host-hyperlight-row] Regenerate and check `test-harness/generated/inventory.json`.
- [ ] [depends:runtime-host-hyperlight-row] Update `docs/runtime-host-readiness.md` with the accepted Hyperlight evidence boundary only after the runnable target passes.
- [ ] [depends:hyperlight-readiness-doc] Validate OpenSpec, harness metadata, the runnable target, docs guardrails, and whitespace before archive.
