## Why

The local multiprocess model validates process plans and run receipts, but developers still need a cheap executable layer that actually starts isolated `molten node` processes before paying the NixOS VM cost. Without that runner, process lifecycle, state-root isolation, local transport handles, signal handling, and cleanup failures can remain invisible until the VM shard runs.

## What Changes

- Add an executable local multiprocess runner that consumes the existing pure plan and spawns isolated node processes from explicit state roots.
- Run a small cross-process control workflow and collect startup, workflow, shutdown, cleanup, and reconciliation receipts.
- Deny pass evidence for stale tickets, state-root collisions, transport collisions, missing receipts, orphaned processes, and cleanup failures.
- Keep local multiprocess evidence scoped below VM/platform evidence and mark logs diagnostic-only.

## Impact

Developers get a fast integration check between deterministic simulation and NixOS VM. The runner can catch lifecycle and cleanup regressions early without claiming systemd, QEMU, live WAN, authority, policy, provenance, resource, source-gate, retention, deployment, or production-readiness evidence.
