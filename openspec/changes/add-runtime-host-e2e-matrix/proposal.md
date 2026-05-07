## Why

Aspen has real Cloud Hypervisor microVM tests and runtime-core admission models for WASM, OCI, Hyperlight, Hermit, and microVM host loading, but the current test surface does not clearly prove which host classes are actually spawned by Aspen. That makes it too easy to overclaim model/plumbing coverage as full runtime-host execution.

## What Changes

- Add a runtime-host E2E matrix requirement that separates model tests, real host execution, and Aspen-spawned execution.
- Register the existing `CloudHypervisorWorker + ci_vm` path as the first Aspen-spawned microVM proof in the test-harness inventory.
- Make nested KVM an explicit suite prerequisite so expensive/impure host tests are discoverable without running by default.
- Leave WASM runner, OCI lowering execution, Hyperlight, and Hermit E2E implementation as follow-up matrix rows rather than pretending they exist.

## Capabilities

### Modified Capabilities
- `runtime-host-loading`: Adds verification requirements for host-runner E2E matrix coverage and clear support-level labeling.
- `test-suite-metadata`: Uses suite metadata to expose the first runtime-host E2E row and its prerequisites.

## Impact

- **Files**: `openspec/changes/add-runtime-host-e2e-matrix/**`, `test-harness/**`, `crates/aspen-testing/src/suite_inventory.rs`, generated inventory.
- **APIs**: No public runtime API changes.
- **Dependencies**: None.
- **Testing**: Validate OpenSpec, regenerate/check harness inventory, run focused aspen-testing unit tests, evaluate the flake check attr, and avoid launching the nested-KVM VM test unless explicitly requested.
