## Why

The runtime-host matrix still lists Hyperlight as a metadata-only gap. Aspen already has `HyperlightWorker` registration, blob-backed `vm_execute` job payloads, admission/profile coverage, and example Hyperlight guest material, but the matrix must not claim readiness until a runnable suite proves real Aspen-spawned Hyperlight execution with product-visible receipt evidence.

## What Changes

- Define the evidence contract for promoting `runtime-host-hyperlight-gap` into a runnable Hyperlight runtime-host row.
- Require product-path job orchestration (`JobManager` / `WorkerPool` / `HyperlightWorker` or equivalent node worker registration) rather than direct worker-only calls.
- Require explicit proof markers and negative guardrails so model tests, worker construction, package builds, and ignored/manual examples cannot satisfy the row.

## Out of Scope

- Promoting the row before a runnable Hyperlight suite exists.
- Claiming Hermit or OCI lowering readiness.
- Treating raw container execution as a production runtime-host proof.

## Verification

Validate this OpenSpec with `openspec validate promote-hyperlight-runtime-host-e2e --strict`. Implementation follow-up must add the runnable target, regenerate/check harness inventory, update readiness docs only after the target passes, and run the focused product-path test plus metadata/docs guards.
