## Why

Two-node VM coverage proves transport and basic cross-node workflows, but it cannot expose majority/minority quorum mistakes, subscriber-versus-voter confusion, or restart/rejoin behavior where one member returns to an existing control-plane group. Existing topology-profile validation covers these concepts in pure tests; VM evidence should include one small three-node topology for platform integration.

## What Changes

- Add a three-node NixOS VM topology profile with explicit voter, restarting member, and subscriber or observer roles.
- Exercise majority/minority partition, restart/rejoin, and duplicate semantic commit suppression in the VM topology.
- Bind three-node topology membership, quorum evidence, reconciliation, and failure diagnostics into canonical VM receipts.
- Add negative fixtures proving subscribers and transport-only peers cannot satisfy voter membership or authority claims.

## Impact

The VM layer gains coverage for role and quorum mistakes that two nodes cannot demonstrate. The evidence remains topology-scoped and does not claim fleet scale, WAN behavior, authority, policy, provenance, resource, source-gate, retention, or production readiness.
