## Context

Existing coverage is uneven: several NixOS tests launch real Cloud Hypervisor guests, `vm-snapshot-e2e.nix` is the closest Aspen-spawned `CloudHypervisorWorker + ci_vm` proof, plugin tests mostly cover WASM blob/manifest/reload plumbing, and OCI/Hyperlight/Hermit coverage is largely runtime-core model/admission evidence.

## Goals / Non-Goals

**Goals:**
- Define a matrix that distinguishes real host execution from Aspen-spawned execution.
- Surface the existing microVM Aspen-spawned path through the test harness.
- Record nested-KVM/impure prerequisites explicitly.
- Keep gap labels honest for WASM, OCI, Hyperlight, and Hermit.

**Non-Goals:**
- Do not run the expensive nested-KVM suite in normal focused verification.
- Do not implement WASM/OCI/Hyperlight/Hermit E2E in this first slice.
- Do not change public runtime receipt schema or production admission semantics.

## Decisions

### 1. First row uses `vm-snapshot-e2e-test`

**Choice:** Register the existing `nix/tests/vm-snapshot-e2e.nix` check as `runtime-host-microvm-ci-vm`.

**Rationale:** It starts `aspen-node` with CI VM environment, waits for `CloudHypervisorWorker` golden snapshot creation, submits `ci_vm` jobs through `aspen-cli`, and verifies job completion. That is the closest existing Aspen-spawned microVM path.

**Alternative:** Create a new microVM test from scratch. Rejected for this slice because it adds churn before the existing proof is discoverable and named.

### 2. Nested KVM is explicit metadata

**Choice:** Extend test-harness prerequisites with `nested-kvm` and tag the suite as expensive/impure.

**Rationale:** Operators need to list and select the suite without accidentally launching a nested virtualization workload. Existing prerequisite values did not distinguish ordinary Nix command support from nested KVM.

### 3. Matrix gaps stay active as metadata-only rows

**Choice:** Add non-runnable `metadata-only` harness rows for WASM, OCI lowering, Hyperlight, and Hermit while retaining only the microVM row as `aspen-spawned-execution` evidence.

**Rationale:** The matrix is more useful when every intended host class is visible, but gap rows must not masquerade as runnable proof. Metadata-only rows carry `runtime_host.kind`, `proof_level`, `support_status`, and `gap_reason`; command listing prints comments and `run` treats them as no-op comments rather than build commands. Future work promotes a gap by replacing the metadata-only target with a real suite target and evidence.

## Risks / Trade-offs

**Expensive VM tests are skipped by default** → Verification for this slice checks evaluation/metadata, not runtime execution. A future promoted acceptance gate must run the VM suite on a nested-KVM host.

**Metadata can overclaim** → The delta spec requires suite descriptions to state host class, spawn path, prerequisites, and observable proof signals.
