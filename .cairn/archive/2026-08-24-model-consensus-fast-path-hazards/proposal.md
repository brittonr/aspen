## Why

Molten is developing a live, extension-facing consistency service, but its current Raft path is intentionally model-only and the live service is blocked on admitted cross-process transport. Adding a latency fast path before the base engine is live would compound that blocker and blur production claims. The Jetpack OSDI 2026 paper and artifact nevertheless expose reusable safety hazards that the existing consensus model and fault corpus do not make explicit: acknowledgements spanning views, loss of proposer promises after election, stale conflicting entries preceding recovered fast commits, and duplicate execution when the fast and original paths converge.

A bounded model-only change can capture those hazards now without selecting Jetpack, importing its implementation, or claiming live performance. It gives the later live-acceleration work a checked prerequisite and gives deterministic simulation and ChaosControl concrete failure schedules to exercise.

## What Changes

- Add a model-only crash-fault consensus acceleration profile pinned to the Jetpack paper and MIT-licensed artifact identity as design references.
- Define pure, schema-bound conflict classification and conservative fallback semantics.
- Model concurrent fast and original paths over the same canonical command identity, same-view superquorums, proposer promises, fallback, convergence, and duplicate suppression.
- Model independent acceleration views and base-engine views, recovery-set agreement, recovery markers, cascading failures, and recovery-before-new-view ordering.
- Add positive and negative three-replica and five-replica fault scenarios, invariant evidence, counterexample export, and explicit model-only non-claims.
- Keep the work independent of the blocked live transport shell and make it a prerequisite for any production fast-path acceleration profile.

## Impact

- **Files**: typed model profile and fixtures, pure consensus transition model, conflict-contract model, bounded exploration/fault scenarios, canonical model evidence, operator readback, and `cairn/specs/consensus/spec.md`.
- **Testing**: happy-path fast commit and fallback; conflict conservatism; view-straddled acknowledgements; leader failure after fast reply; stale conflicting entries; recovery interruption; duplicate suppression; three-node availability limits; five-node quorum intersection; deterministic replay and counterexample minimization.
- **Safety**: all results remain pure-model or deterministic-simulation evidence. They do not prove the Jetpack artifact, Molten live transport, a live Raft engine, production linearizability, throughput, latency improvement, Byzantine tolerance, transaction semantics, or release readiness.
