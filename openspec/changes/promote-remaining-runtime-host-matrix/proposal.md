## Why

The runtime-host matrix now has product-path proof for the expensive and high-value rows that were formerly gaps: microVM CI, WASM, OCI lowering, Hyperlight, and Hermit/Uhyve. The remaining risk is not another immediate host implementation; it is drift in the matrix itself. Future rows for native built-ins, trusted external native processes, loader/QEMU unikernel variants, or other host profiles can be added as metadata-only inventory and later cited as readiness by mistake.

A scoped OpenSpec should define the promotion contract for any remaining or newly added metadata-only runtime-host row before implementation work starts. That keeps Aspen from overclaiming readiness while giving the next product slice a deterministic checklist.

## What Changes

- Define a generic row-promotion contract for remaining metadata-only runtime-host matrix rows.
- Require each promotion to name the specific host kind, product orchestration path, proof marker, harness target, and operator receipt boundary.
- Preserve explicit gap labels for rows that only have model, admission, packaging, or metadata evidence.
- Add validation tasks for keeping harness inventory and readiness docs aligned.

## In Scope

- OpenSpec requirements and task rails for future metadata-row promotions.
- Acceptance criteria for product-path proof, negative guardrails, and secret-safe evidence.
- Explicit anti-overclaiming language for rows that are still metadata-only.

## Out of Scope

- Implementing a new runtime host in this slice.
- Re-running already promoted WASM, OCI, Hyperlight, Hermit/Uhyve, or microVM product-path proofs.
- Treating package builds, plugin install/reload, model tests, or inventory rows as runtime-host readiness by themselves.

## Verification

- `openspec validate promote-remaining-runtime-host-matrix --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
