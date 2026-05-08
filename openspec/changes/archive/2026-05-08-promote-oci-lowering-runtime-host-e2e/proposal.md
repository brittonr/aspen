## Why

The runtime-host matrix still lists OCI lowering as a metadata-only gap. Aspen has portable `OciLoweringPlan` and `OciLoweringReceipt` admission/receipt coverage, plus proven target host rows for microVM, WASM, and gated Hyperlight, but no runnable suite currently ingests an immutable OCI artifact, lowers it into an isolated Aspen runtime host, executes it, and records secret-safe product-visible receipt evidence.

OCI is a packaging and lowering input, not a production host boundary. The matrix must keep that distinction explicit so model/admission coverage and raw container smokes cannot be mistaken for Aspen-spawned execution.

## What Changes

- Define the evidence contract for promoting `runtime-host-oci-lowering-gap` into a runnable OCI-lowering runtime-host row.
- Require an immutable OCI source identity plus a derived isolated execution artifact that is submitted through a real Aspen target host path such as microVM, WASM, Hyperlight, or a VM-backed unikernel.
- Require explicit OCI lowering proof markers and negative guardrails so admission/model tests, image metadata parsing, package builds, and raw container execution cannot satisfy the row.

## Out of Scope

- Promoting the row before a runnable OCI-lowering suite exists.
- Treating Podman/Docker-style host containers as a production runtime-host proof.
- Claiming Hermit readiness or broadening any target-host row beyond its own verified evidence.

## Verification

Validate this OpenSpec with `openspec validate promote-oci-lowering-runtime-host-e2e --strict`. Implementation follow-up must add the runnable target, regenerate/check harness inventory, update readiness docs only after the target passes, and run the focused product-path test plus metadata/docs guards.
