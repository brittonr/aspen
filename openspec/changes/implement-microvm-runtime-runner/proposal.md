## Why

Aspen needs a real isolated host runner before OCI lowering or tenant/CI workloads can execute without drifting toward plain host containers. The host-loading spec names MicroVm as a boundary, but no active change owns the node-local runner contract.

## What Changes

- Define the node-local microVM runner lifecycle for Firecracker, Cloud Hypervisor, QEMU microvm, and equivalent engines.
- Specify runner capability detection, artifact preparation, start/stop, leases, heartbeats, logs, outputs, and receipts.
- Keep production admission fail-closed when virtualization support or runner capability is missing.

## In Scope

- Active OpenSpec package for the microVM runtime runner implementation seam.
- Requirements, design constraints, implementation tasks, and verification plan.
- Integration with the existing runtime-host-loading and runtime-service-core direction.

## Out of Scope

- Full OCI lowering implementation.
- Hermit-specific unikernel profile details beyond generic guest-artifact support.
- Live VM migration.

## Verification

- `openspec validate implement-microvm-runtime-runner --strict`
- Focused runtime-core or runner tests added by the implementation task.
- Docs/source-anchor tests where the change affects runtime architecture documentation.
- `git diff --check`
