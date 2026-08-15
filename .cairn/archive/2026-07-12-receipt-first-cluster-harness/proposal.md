## Why

Cluster harness behavior is already modeled in canonical receipts, fixture metadata, drift summaries, and failure bundles, but the everyday executable path still lets operators and CI reason from scattered per-node files, rendered stdout, or handwritten command sequences. That makes review harder and leaves room for cluster pass claims that are not tied to one durable lifecycle artifact directory.

We need a receipt-first cluster harness surface where a checked scenario fixture drives execution, the shell writes one reviewable run directory, and offline verification can distinguish simulation, local multiprocess, VM, unavailable, and diagnostic-only evidence without replaying ambient state.

## What Changes

- Add a receipt-first executable cluster harness command surface that writes `cluster-lifecycle-run-v1` evidence for fixture-backed cluster workflows.
- Standardize cluster run artifact directories and offline verification so canonical refs, not stdout, become the review surface.
- Connect fixture-derived plans to the local multiprocess tier as an executable middle rung between deterministic simulation and NixOS VM evidence.
- Add first-divergence failure triage and sealed diagnostic failure-bundle export for denied cluster runs.

## Impact

- **Files**: `src/cluster.rs`, `src/cli/ops/cluster.rs`, `src/testing/multinode/**`, `src/nixos/**` as needed, `tests/parts/cliharness/**`, `docs/distributed-testing.md`, `README.md`, `cairn/specs/testing-harness/spec.md`.
- **Testing**: focused cluster lifecycle unit tests, CLI tests for receipt output and offline verification, local multiprocess positive/negative tests, fixture-driven metadata tests, failure-bundle and first-divergence denial tests, plus Cairn validation/gates.
- **Safety**: new receipts and directories remain evidence surfaces only; they do not grant authority, policy, provenance, resource, source-gate, retention, transport, deployment, production-readiness, or release trust by themselves.
