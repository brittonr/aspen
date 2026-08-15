## Why

Cloudflare's Meerkat write-up is a useful external signal for Molten's own control-plane roadmap: strong consistency remains valuable for small, global coordination state, but leader-dependent Raft can be fragile under wide-area latency, slow leaders, and leader failure. Molten already scopes consensus narrowly to control-plane state, so the right adoption path is to preserve that boundary while making consensus behavior explicit enough to evaluate future leaderless quorum profiles without weakening existing Raft evidence.

## What Changes

- Add explicit consensus algorithm profiles to control-plane manifests so `raft` remains the default and any Meerkat/QuePaxa-inspired leaderless profile is opt-in, experimental, and evidence-gated.
- Add explicit read consistency modes for control-plane and coordination reads: linearizable reads require quorum/read evidence, while local-stale reads are allowed only as visibly stale, non-authoritative observations.
- Add placement and fault-model evidence for consensus groups so operators can review member placement, majority reachability assumptions, and the expected latency/failure trade-offs before relying on the group.
- Add deterministic simulation requirements for slow or failed leaders, concurrent proposals, majority/minority partitions, stale read attempts, and placement misconfiguration.
- Keep non-claims explicit: no Byzantine tolerance, no general database promise, no ordinary actor traffic through consensus, and no lease reads without future timing assumptions plus policy evidence.

## Impact

- **Files**: Adds a native Cairn change package under `cairn/changes/meerkat-control-plane-adoption/` with deltas for `consensus`, `coordination`, and `testing-harness`.
- **Testing**: Lifecycle validation should pass for the new proposal/design/tasks/spec deltas. Future implementation will need focused consensus, coordination, and deterministic simulation tests before archive.
