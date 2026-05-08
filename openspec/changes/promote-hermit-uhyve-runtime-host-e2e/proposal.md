# Promote Hermit/Uhyve runtime-host E2E

## Why

Hermit is the final runtime-host matrix row that still remains metadata-only after the microVM, WASM, Hyperlight, and OCI-lowered WASM promotions. The existing Hermit work proves portable profile/admission/receipt modeling, but it does not spawn a Hermit unikernel through Aspen. The next promotion needs a real Uhyve-backed product-path run, because Uhyve is the Hermit-specific hypervisor and is the likely lowest-friction Hermit host path.

## What Changes

- Define the evidence contract for replacing `runtime-host-hermit-gap` with a runnable Hermit/Uhyve product-path row.
- Require Aspen job orchestration or node worker registration to launch the Hermit unikernel; direct `uhyve <image>` smokes remain non-proof.
- Require a stable proof marker (`ASPEN_HERMIT_UHYVE_RUNTIME_HOST_EXECUTED`) and guard marker (`ASPEN_HERMIT_UHYVE_RUNTIME_HOST_PRODUCT_PATH_GUARD`).
- Keep the row gated/opt-in unless the host has Uhyve plus the required virtualization backend.

## In Scope

- A Uhyve runner/worker seam that accepts a declared Hermit artifact from Aspen-owned blob/artifact storage.
- Product-path tests that submit the run through `JobManager`/`WorkerPool` or equivalent node worker registration.
- Secret-safe lifecycle/output receipts that identify image hash, runner engine, lifecycle state, exit status, and bounded serial output.
- Harness/docs/doc-guard updates only after runnable product-path evidence exists.

## Out of Scope

- Promoting Hermit from admission/model/schema tests, package builds, direct `uhyve` shell commands, or skipped tests that were not executed in a capable environment.
- Treating Hermit as an OCI/Linux container or native process.
- Broad networking/TAP support, multi-node Hermit orchestration, or loader/QEMU promotion unless Uhyve proves blocked.

## Verification

- `openspec validate promote-hermit-uhyve-runtime-host-e2e --strict`
- Focused Hermit runner/product-path tests once implemented.
- Gated real Uhyve proof with `/dev/kvm` or equivalent host support, executed through Aspen orchestration.
- `scripts/test-harness.sh export && scripts/test-harness.sh check`
- `cargo test --test runtime_host_readiness_docs -- --nocapture`
- `openspec validate --all --strict --json`
- `git diff --check`
