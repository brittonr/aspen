# Proposal: Add protocol-aware simulation oracles

## Why

Molten whole-system simulation runs the admitted extension core and records bounded invariant results. The current reference path can derive semantic success from invariant names reported by the transition under test. It can also derive final state refs from Rust debug text.

Those mechanisms validate fixture conformance. They do not independently recompute coordinated protocol properties across participants.

Protocol-aware deterministic simulation needs canonical consumer-owned projections, independent pure oracle evaluation, participant-scoped liveness, and stable protocol-state identities. The fabric must transport and evaluate these artifacts without owning consensus, storage, scheduler, or extension semantics.

## What Changes

- Add versioned extension-owned protocol projection contracts and bounded runtime records.
- Replace debug-derived protocol identities with canonical Preserves projections and domain-separated BLAKE3 refs.
- Add independent pure oracle evaluation over admitted projection cohorts.
- Add local, pairwise, cohort, and selected durability safety results.
- Add participant-scoped liveness with pass, fail, not-evaluated, and incomplete results.
- Add stable protocol novelty identities and deterministic work counters.
- Keep local guards available to live profiles while expensive global oracles remain simulation-owned.
- Bind projection, oracle, scheduler, fault, completeness, result, and non-claim facts into evidence.

## Impact

- **Files**: `molten-core` fabric simulation types and pure evaluators, reference services, simulation composition, Nickel profiles, receipts, and documentation.
- **Testing**: canonical projection, false self-report, cross-participant divergence, incomplete observation, conditional liveness, novelty stability, counter overflow, replay, and claim-boundary fixtures.
- **Architecture**: extensions own protocol meaning. The fabric owns bounded envelopes, scheduling facts, evaluation orchestration, and evidence assembly.
- **Dependencies**: the accepted `fabric-simulation` contract. A later ChaosControl adapter can consume the same projections through an immutable published contract.
- **Claims**: passing results cover only the exact projection, oracle, world, scheduler, adapter, fault, and bound cohort.
