## Why

Molten has pure three-node quorum and membership gates plus a typed `vm-three-node-quorum` fixture, but platform evidence is incomplete unless an executable VM shard exercises that topology. Without executable wiring, reviewers cannot distinguish planned quorum coverage from actual VM-observed quorum behavior.

## What Changes

- Add an executable three-node NixOS VM shard or equivalent gated platform check for the `three-node-quorum` fixture.
- Exercise majority quorum, minority denial, restart/rejoin, subscriber or observer non-voter behavior, and duplicate semantic commit suppression through canonical receipts.
- Bind the shard into VM scenario, reconciliation, and aggregate gates without claiming fleet-scale or WAN correctness.
- Add negative fixtures for subscriber-as-voter, transport-only authority, wrong topology, missing quorum, and log-only quorum claims.

## Impact

The harness gains platform-scoped quorum evidence for a bounded topology. It remains separate from pure consensus correctness, authority, policy, provenance, source-gate, resource, retention, deployment, and production claims.
