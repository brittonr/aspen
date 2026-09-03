# Design: Protocol-aware simulation oracles

## Context

The accepted fabric simulation contract requires extension-owned semantic invariants over canonical state refs, histories, events, and observations. The current reference composition records `semantic_invariants_passed` from the transition and later validates membership in that list. Final state evidence can also hash Rust debug output.

This design adapts the protocol-aware deterministic simulation method described at:

- `https://tigerbeetle.com/blog/2026-08-20-protocol-aware-dst/`

The reference motivates deeper internal validation. It does not transfer TigerBeetle protocol semantics, storage rules, or correctness claims to Molten.

## Success Contract

An accepted protocol-aware run uses canonical bounded projections from the admitted extension path. A separately identified pure oracle recomputes each result from those projections and declared preconditions.

A self-reported invariant name cannot establish success. Missing or conflicting required observations cannot become a pass.

## Decisions

### Decision: Extensions own projections and oracle meaning

**Choice:** Each participating extension owns its projection schema, selected logical positions, safety rules, progress measures, and recovery meaning. The fabric owns envelope admission, bounds, scheduler linkage, cohort assembly, oracle invocation, and evidence composition.

**Rationale:** Protocol meaning belongs with the consumer. The node core must remain workload-neutral.

### Decision: Protocol projections are canonical artifacts

**Choice:** A runtime projection binds protocol, service, participant, generation, source sequence, scheduler choice, transition class, logical position, state, history, durability, progress, and completeness facts. The human-authored profile is typed Nickel. Runtime values use canonical Preserves and domain-separated BLAKE3 refs.

Rust `Debug` text cannot serve as an evidence-bearing protocol identity. Large values remain bounded artifacts behind canonical refs.

**Rationale:** Durable evidence needs stable identities across builds and processes.

### Decision: Oracle evaluation is independent from transition self-report

**Choice:** The pure oracle consumes admitted projection values and explicit preconditions. It does not call the transition under test. It does not treat `semantic_invariants_passed` or another pass list as sufficient evidence.

The oracle and projection contracts have separate identities. They can share canonical domain types and reviewed pure helpers.

**Rationale:** A transition and its self-report can share one defect. An independent evaluation path can detect a false success claim.

### Decision: Safety has explicit levels

**Choice:** Results distinguish local transition safety, pairwise agreement at one logical position, whole-cohort safety, and selected durability properties. A later matching state does not erase an earlier failure.

Physical byte equality applies only when the selected adapter profile promises a canonical physical layout. Other profiles compare extension-owned semantic projections.

**Rationale:** Different safety properties need different facts and claim boundaries.

### Decision: Local guards and global oracles have separate cost profiles

**Choice:** Cheap pure local guards can run in simulation and supported live profiles. The core returns a typed invariant violation. The shell applies the admitted quarantine, fail-stop, or denial policy.

Expensive pairwise and cohort oracles can remain simulation-only. Normal malformed input and policy denial remain typed errors, not invariant crashes.

**Rationale:** Production guards need bounded cost. Simulation can spend more work on global validation.

### Decision: Liveness is participant-scoped and conditional

**Choice:** Each participant result is pass, fail, not-evaluated, or incomplete. Evaluation requires the declared readiness, membership or quorum, disruptive-fault, durability, and virtual-progress facts.

Aggregate cluster progress cannot hide a stalled eligible participant. Missing preconditions cannot become a failure or a pass.

**Rationale:** Liveness has meaning only under explicit stabilization facts.

### Decision: Protocol novelty uses stable canonical identity

**Choice:** A typed profile selects projection fields for novelty. The pure core computes a domain-separated BLAKE3 identity over those canonical fields.

The scheduler and later ChaosControl adapters can use the full identity for guidance. A process-local hash or coverage slot is not durable evidence.

**Rationale:** Search guidance must replay across processes and tool versions.

### Decision: Deterministic work counters are evidence, not hardware benchmarks

**Choice:** Extensions may expose named monotonic counters for messages, transitions, storage operations, copied bytes, repairs, and protocol rounds. Comparisons bind identical world, scheduler, workload, fault, and counter-schema refs.

The result cannot claim wall-clock latency, hardware throughput, or production performance.

**Rationale:** Deterministic counters can expose algorithmic regressions without hardware noise.

### Decision: Evidence fails closed

**Choice:** Receipts bind world, runtime, projection schema, oracle, participant set, scheduler, workload, faults, observation completeness, results, counters, replay, and non-claims. Incomplete, conflicting, unsupported, or stale artifacts remain distinct from pass.

**Rationale:** Evidence must not exceed the observed cohort.

## Functional Core and Imperative Shell

The pure core owns projection admission, canonicalization, cohort assembly, safety evaluation, liveness evaluation, novelty identity, counter comparison, and claim classification.

The shell owns extension execution, port adapters, projection transport, artifact persistence, scheduler control, fault execution, oracle orchestration, and receipt storage.

## Dependencies and Integration

The pure mechanism depends only on accepted Molten simulation types and canonical artifact support. A later ChaosControl integration must use a published immutable protocol-observation contract.

The existing `add-chaoscontrol-consensus-conformance` package remains separate. It owns external KVM evidence for a selected consensus cohort.

## Risks and Trade-offs

- A projection can omit the fact that exposes a defect. Negative observer fixtures and completeness accounting reduce this risk.
- The oracle can repeat an implementation defect. Separate identity, adversarial fixtures, and outside-in history validation remain necessary.
- Detailed projections can increase trace size. Profiles must set finite record, artifact, event, and total-byte bounds.
- Physical equality can overconstrain valid adapters. It remains an explicit selected-profile property.
- Protocol-aware evidence can appear stronger than it is. Receipts retain exact scopes and non-claims.
