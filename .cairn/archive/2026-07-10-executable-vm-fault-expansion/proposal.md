## Why

The distributed harness models many fault types, but executable VM evidence is only as strong as the faults actually supported and recorded by the platform check. Simulated or unavailable fault evidence must not be mistaken for real executable fault coverage.

## What Changes

- Expand executable VM fault coverage where host support permits: delay, drop, partition, rejoin, asymmetric latency, restart during dispatch, stale ticket, wrong authority, duplicate operation, conflicting operation id, corrupted receipt, and permission-denied state root.
- Add a support matrix that records required capability, preflight result, injection refs, child refs, post-fault refs, decision, replay status, and caveats per fault.
- Deny pass evidence when network-control support is unavailable, cleanup cannot restore the VM state, or only logs support the claim.
- Keep simulated fault cases classified as diagnostic unless separately promoted by an explicit review gate.

## Impact

Fault coverage becomes more honest and reviewable. Executable fault evidence remains bounded to the VM topology and does not grant authority, policy, provenance, source-gate, resource, retention, deployment, production, or WAN transport trust.
