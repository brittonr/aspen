## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for promoting the OCI lowering runtime-host matrix row. ✅ Spec package defines source OCI identity, isolated lowering target, receipt, and anti-overclaiming requirements.
- [x] [depends:spec-foundation] Validate the new OpenSpec package and all specs strictly. ✅ `openspec validate promote-oci-lowering-runtime-host-e2e --strict`; `openspec validate --all --strict --json`; `git diff --check`.

## Phase 2: Product-path proof

- [x] [parallel:proof] Choose the smallest deterministic OCI fixture and isolated target host path; prefer a proven local product path if it can retain OCI source identity. ✅ Selected an OCI-packaged WASM fixture with immutable `sha256:` OCI identity and derived `aspen:runtime-host/wasm-v1` target artifact.
- [x] [parallel:proof] Implement a runnable product-path suite that ingests immutable OCI identity, lowers to a derived isolated artifact, submits execution through Aspen orchestration, and observes terminal state/receipt evidence. ✅ `crates/aspen-jobs/tests/oci_lowering_product_path_test.rs` submits the derived artifact through `JobManager` + `WorkerPool` and checks terminal success.
- [x] [parallel:guard] Add negative guardrail coverage for model-only lowering plans, raw host-container/dev-only execution, missing derived artifacts, and mutable tags without content digest identity. ✅ Guard tests cover model-only receipts, raw `OciContainer`, mutable tag inputs, missing derived artifacts, and invalid derived WASM bytes reaching the product worker path before failure.
- [x] [parallel:receipt] Ensure receipts/logs include `ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED`, source OCI digest, selected target host, derived artifact hash, runner identity, lifecycle state, and bounded output without secrets. ✅ Product receipt assertions include the marker, OCI source digest, target host, derived hash, runner identity, lifecycle state, bounded WASM output, and secret-safety checks.

## Phase 3: Matrix/docs/archive

- [x] [depends:proof,guard,receipt] Replace `runtime-host-oci-lowering-gap` with the runnable harness row and regenerate/check `test-harness/generated/inventory.json`. ✅ `runtime-host-oci-lowering-product-path` row added; `scripts/test-harness.sh export && scripts/test-harness.sh check` passed.
- [x] [depends:proof,guard,receipt] Update runtime-host readiness docs and doc guards only after runnable evidence passes. ✅ `docs/runtime-host-readiness.md` and `tests/runtime_host_readiness_docs.rs` updated; doc guard test passed.
- [x] [depends:matrix-docs] Run focused tests, harness check, `openspec validate --all --strict --json`, and `git diff --check`; archive only when all tasks are complete. ✅ Focused OCI product-path tests, runtime readiness doc guard, harness check, strict OpenSpec validation, and `git diff --check` passed before archive.
