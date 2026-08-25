# Consensus fast-path hazard model

Molten includes a bounded crash-fault model for consensus fast-path composition.
The model is not a live engine and cannot select a production profile.

## Reference cohort

The design reference is `Jetpack: Consensus Made Generally Fast`, OSDI 2026.
The artifact reference is `stonysystems/jetpack` commit `c03e318ec355b11edd42aac56c68d0765f88d1d2` under MIT terms.

Molten independently expresses its profile, transitions, schedules, and invariants.
External TLA+ results and benchmarks do not prove Molten behavior or performance.

## Profile and quorum boundary

`config/consensus-fastpath/profile.ncl` defines three-replica and five-replica profiles.
Both profiles bind one base model, conflict contract, source cohort, finite bounds, invariants, and non-claims.

The model derives a majority and a three-quarter superquorum.
The three-replica superquorum needs every replica.
One failed replica can leave the original majority path available while the fast path is unavailable.

Live and production selections fail contract or Rust admission.
Unknown references, fault models, bounds, ordering guarantees, and claim profiles also fail closed.

## Conflict boundary

The extension owns the versioned conflict contract.
Distinct canonical keys can be independent when neither response uses shared state.

The following inputs conservatively use the original path:

- unknown schemas or operations
- aliases
- ranges and predicates
- preconditions
- response dependencies
- analysis failures

A false positive loses a fast opportunity.
A false negative can break command ordering and is therefore an invariant failure.

## Stable-view boundary

A modeled fast commit needs all of these facts:

- one canonical operation identity on both paths
- matching acceleration and base views
- one same-view superquorum
- a compatible promise from every active original-path proposer
- a conflict-free classification

Mixed-view acknowledgements never combine.
Missing promises, conflicts, identity drift, or insufficient acknowledgements use the original path.
Convergence applies and replies to one session operation at most once.

## Recovery boundary

A base-view change pauses fast admission.
Recovery targets the last normal acceleration view.

The next normal view starts only after these actions:

1. Agree on a recovery set that preserves every accepted command.
2. Commit the recovered commands through the original path.
3. Commit a recovery marker, including an explicit marker for an empty set.
4. Resume with matching base and acceleration views.

Interrupted and cascading recovery cannot drop previously accepted commands.
Recovered commands must precede conflicting new-view work.

## Fault corpus

The bounded corpus names these schedules:

- non-conflicting fast commit
- conflict fallback
- original-only operation
- view-straddled acknowledgement
- missing proposer promise
- leader failure after fast reply
- stale conflicting predecessor
- partition and quorum loss
- interrupted and cascading recovery
- replica restart and rejoin
- duplicate convergence

The model checks recoverability, conflicting predecessors, committed order, execution order, conflict order, applications, and replies.
Counterexample reduction keeps the causal prefix through the first violation.

## Evidence and non-claims

Canonical readback records the profile, source revision, finite coverage, first divergence, invariant results, and unexplored alternatives.
A repro bundle keeps the minimized model schedule and expected violation.

These artifacts are model evidence only.
They do not prove the external artifact, live transport, durability, timing, production linearizability, throughput, latency, Byzantine tolerance, transactions, arbitrary predicates, or release readiness.
