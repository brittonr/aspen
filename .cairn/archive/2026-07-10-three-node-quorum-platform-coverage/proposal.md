## Why

Molten has pure three-node quorum and membership models, but the executable VM topology currently covers two nodes. That leaves voter majority/minority behavior, restart/rejoin, subscriber or observer negatives, and duplicate semantic commit suppression without platform-level evidence.

## What Changes

- Add a bounded three-node VM topology shard with explicit voter roles and isolated node state roots.
- Exercise majority quorum, minority denial, restart/rejoin, and duplicate operation suppression with canonical evidence.
- Add negative fixtures that reject subscriber, observer, transport-only, partitioned-minority, or missing-quorum evidence as voter membership or authority.
- Bind three-node shard evidence into reconciliation and aggregate outputs without claiming fleet-scale correctness.

## Impact

Reviewers gain platform evidence for quorum-shaped behavior while keeping the claim scoped to a small VM topology. Pure consensus logic, authority, policy, provenance, and production claims remain separately gated.
