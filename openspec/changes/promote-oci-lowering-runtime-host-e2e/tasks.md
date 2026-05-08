## Phase 1: Spec foundation

- [x] [serial] Create the focused OpenSpec package for promoting the OCI lowering runtime-host matrix row. ✅ Spec package defines source OCI identity, isolated lowering target, receipt, and anti-overclaiming requirements.
- [x] [depends:spec-foundation] Validate the new OpenSpec package and all specs strictly. ✅ `openspec validate promote-oci-lowering-runtime-host-e2e --strict`; `openspec validate --all --strict --json`; `git diff --check`.

## Phase 2: Product-path proof

- [ ] [parallel:proof] Choose the smallest deterministic OCI fixture and isolated target host path; prefer a proven local product path if it can retain OCI source identity.
- [ ] [parallel:proof] Implement a runnable product-path suite that ingests immutable OCI identity, lowers to a derived isolated artifact, submits execution through Aspen orchestration, and observes terminal state/receipt evidence.
- [ ] [parallel:guard] Add negative guardrail coverage for model-only lowering plans, raw host-container/dev-only execution, missing derived artifacts, and mutable tags without content digest identity.
- [ ] [parallel:receipt] Ensure receipts/logs include `ASPEN_OCI_LOWERING_RUNTIME_HOST_EXECUTED`, source OCI digest, selected target host, derived artifact hash, runner identity, lifecycle state, and bounded output without secrets.

## Phase 3: Matrix/docs/archive

- [ ] [depends:proof,guard,receipt] Replace `runtime-host-oci-lowering-gap` with the runnable harness row and regenerate/check `test-harness/generated/inventory.json`.
- [ ] [depends:proof,guard,receipt] Update runtime-host readiness docs and doc guards only after runnable evidence passes.
- [ ] [depends:matrix-docs] Run focused tests, harness check, `openspec validate --all --strict --json`, and `git diff --check`; archive only when all tasks are complete.
