## Why

Pure distributed simulation gives fast deterministic feedback, and the NixOS VM topology gives platform evidence, but there is no cheap middle layer that starts real node processes with isolated state roots. Bugs in sockets, process lifecycle, local transport setup, receipt writing, and shutdown cleanup can therefore escape until the expensive VM check.

## What Changes

- Add a local multiprocess multinode harness profile that spawns separate `molten node` processes with explicit state roots and admitted local transport.
- Keep process planning, expected receipt classification, and reconciliation logic in pure testable builders.
- Add positive coverage for a cross-process control workflow and negative coverage for stale tickets, port or state-root collisions, missing receipts, crash cleanup, and orphaned state.
- Emit canonical harness receipts that remain local integration evidence and do not claim VM, live WAN, authority, policy, provenance, resource, source-gate, retention, or production-readiness trust.

## Impact

Developers get faster feedback for real node lifecycle behavior without paying VM startup cost. VM checks remain required for systemd, QEMU networking, filesystem permission, and platform-fault claims.