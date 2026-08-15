## Why

The current NixOS VM multi-node evidence is sufficient for an internal pilot claim inside a controlled VM topology, but the README explicitly leaves real WAN transport, sustained SLOs, adversarial security, authority delegation, retention policy, destructive operations, source-gate trust, and fleet-scale pressure out of scope. The next roadmap slice should target a constrained external/live pilot proof without overstating production readiness.

## What Changes

- Define an operator-managed multi-host pilot soak that runs outside the single NixOS VM topology.
- Bind node-control live workflow, peer tickets, authority grants, remote dataspace/service exchange, blob-ref job execution, coordination apply, retention readback/clearance review, replay verification, network diagnostics, resource envelope, and rollback/stop-the-line evidence.
- Emit a pilot decision receipt that can pass only for the named constrained workload and denies broad production claims.
- Preserve subsystem boundaries: soak evidence is review evidence, not authority, policy, provenance, source-gate, retention, transport, or destructive-operation trust.

## Impact

- **Files**: production soak/operator workflow code, node runtime diagnostics, runbooks, release/pilot decision docs, and tests/fixtures for positive and negative pilot evidence.
- **Testing**: local deterministic fixtures, NixOS VM baseline remains green, external/live pilot runbook evidence, negative stale/missing/over-broad pilot denial tests, and release readback caveats.
