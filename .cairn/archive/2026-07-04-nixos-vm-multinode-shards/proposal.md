## Why

The current `nixos-vm-multinode` check exercises many valuable paths in one large NixOS test-driver script. When it fails, reviewers must inspect a broad artifact set to determine whether the breakage came from node startup, live control transport, remote service/job coordination, restart recovery, VM fault evidence, or final export. That slows feedback and makes unrelated changes pay the full VM cost.

## What Changes

- Split the monolithic VM check into named shard checks for smoke, live control, service/job coordination, restart recovery, VM faults, and full aggregation.
- Add a pure shard-plan model that declares each shard's scenario fixture, required receipts, expected artifact kinds, unavailable policy, and evidence-only caveats.
- Emit canonical shard receipts and an aggregate manifest so reviewers can inspect the smallest failing layer before running the full topology.
- Preserve the existing full VM evidence output as an aggregation profile rather than deleting coverage.

## Impact

Developers get faster, more local VM failures while release review still has a complete aggregated evidence surface. Shard pass evidence remains scoped to the declared scenario and cannot replace authority, policy, provenance, resource, source-gate, retention, production-readiness, or broader live-network claims.
