## Why

The runtime-host matrix now has proven Cloud Hypervisor microVM execution, but the WASM row remains metadata-only. WASM is the next lowest-cost runtime-host class to promote because Aspen already has runtime-core WASM admission, lifecycle, and receipt model coverage, and the row does not require nested KVM.

Without a focused promotion OpenSpec, future work could accidentally count plugin install/reload plumbing or runtime-core model tests as full runtime-host execution. The next row needs a runnable suite that starts Aspen, drives a WASM unit through the product runtime path, and captures product-visible output or receipt evidence.

## What Changes

- Define the acceptance contract for promoting `runtime-host-wasm-gap` into a runnable WASM runtime-host E2E row.
- Require the promoted suite to execute through the real Aspen runtime/RPC/host-call path rather than only direct unit tests.
- Specify proof markers, secret-safe receipt expectations, harness metadata updates, and negative guardrails that prevent overclaiming model coverage.

## In Scope

- A focused OpenSpec package for the WASM runtime-host E2E promotion.
- Requirements, design constraints, task rail, and validation plan for the future implementation slice.
- Test-harness row shape for replacing the current metadata-only WASM gap row.

## Out of Scope

- Implementing the WASM runner/E2E suite in this spec-only slice.
- Promoting OCI lowering, Hyperlight, or Hermit rows.
- Treating CLI plugin installation, reload, or runtime-core-only model tests as sufficient E2E proof.
- Broadening the default local check set with expensive or environment-sensitive tests.

## Verification

- `openspec validate promote-wasm-runtime-host-e2e --strict`
- `openspec validate --all --strict --json`
- `git diff --check`
