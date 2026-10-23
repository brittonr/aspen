# Protocol-aware simulation oracles: verification

## Baseline (task 1, recorded with the change)

- `evaluate_invariants` decided extension-semantic invariants by checking
  that each observation's `semantic_invariants_passed` list contained the
  invariant id. That list was copied from the transition under test, so a
  transition's self-report established success.
- Reference transitions carried `state_material = format!("{next:?}")`.
  Executor state refs, the callback state ref, and final state refs were BLAKE3
  over Rust debug text.

## After

**Pure core: `molten_core::protocol_oracle`.**

- Canonical Preserves value encoding: integers, strings, byte strings,
  symbols, records, and sequences. Domain-separated BLAKE3 refs. A shell test
  shows the core encoder's bytes equal the `preserves` crate's canonical
  encoding of the same projection.
- Typed profile admission (`validate_oracle_profile`).
- Projection admission. It refuses a debug-derived state ref, unknown
  participants, malformed refs, and unbounded state.
- Cohort assembly:
  - incomplete on a lossless sequence gap, a missing required participant, or
    a partial observation;
  - malformed on a duplicate sequence, generation drift, or a record or byte
    overflow.
- Safety at four levels:
  - local rules, through the extension's `ProtocolSemantics`;
  - pairwise and cohort agreement at shared logical positions, with physical
    refs compared only under a canonical-physical profile;
  - durability: a recovered state must be one the participant held at that
    position.

  The earliest failure is retained with its scheduler choice and projection
  refs, and an incomplete cohort never passes.
- Participant-scoped liveness: pass, fail, not-evaluated, or incomplete.
- Novelty identities over profile-selected fields, and `NoveltyGuidance`.
- Monotonic work counters, with exact comparison within one cohort only.
- Evidence values and refs, the evidence verdict, and claim admission. Only a
  deterministic-simulation claim is admitted; KVM, live, production, release,
  and hardware-performance claims are refused.

**Extension-owned semantics: `molten_core::fabric_simulation`.**

- `reference_state_value` (the canonical projector),
  `decode_reference_state`, and `reference_local_violations`, the single
  local-rule function.
- `ReferenceProtocolSemantics` for the oracle, and
  `guard_reference_transition` for the executor.

**Cutover.**

- `SimulationObservation.projection_ref` replaces the pass list.
- `evaluate_invariants` takes the oracle's safety results. An unsupported or
  incomplete rule fails.
- `ReferenceServiceTransition.state_material` is removed.
- Executor, callback, and final state refs use the canonical projector.
- The executor applies the shared guard with a fail-stop policy.

**Shell.**

- `ProtocolTrace` records initial, workload, and recovery projections and
  cumulative counters.
- `evaluate_protocol_trace` binds `ProtocolRunEvidence` into
  `ReferenceFixtureRun`.
- The canonical run record gains a bounded protocol status. Its readback field
  count is 17.
- `fabric-simulation inspect` prints the status.
- `explore_reference_seeds` exposes novelty scores to schedule exploration.
- `counter_cohort` binds counter comparisons.

**Configuration.**

- `config/protocol-oracle/profile.ncl` is the typed profile. It has one
  positive fixture and nine negative fixtures, each failing for its stated
  reason against a passing control:
  - unknown-field, stale-schema, malformed-ref;
  - duplicate-participant, undeclared-participant-service;
  - overflow-bounds, unbounded-payload;
  - missing-non-claim, unknown-novelty-field.
- `config/protocol-oracle/consumer-contract.ncl` is the immutable observation
  contract for a later ChaosControl adapter: record label, field order,
  encoding, identity domains, and the non-evidential report field.
- The flake check `protocol-oracle-profile` exports both and `cmp`s them
  against the generated JSON.

## Results

- `cargo test -p molten-core --lib -- protocol_oracle fabric_simulation`: 59
  passed. Coverage:
  - encoding vectors and the field list;
  - debug-identity, unknown-participant, malformed-ref, and unbounded-state
    refusals;
  - gap, missing-participant, and partial-observation incompleteness;
  - duplicate-sequence, generation-drift, and overflow;
  - divergence followed by later convergence keeping the first failure;
  - semantic agreement versus a physical mismatch;
  - durability pass, fail, incomplete, and not-evaluated;
  - stalled, partitioned, faulted, lost, and short-horizon liveness;
  - novelty stability and change;
  - counter regression, allowed reset, and cohort mismatch;
  - evidence verdicts and claim promotion;
  - a false self-report failing the oracle;
  - the guard and the oracle sharing one rule function;
  - a malformed command staying a typed admission error.
- `cargo test -p molten --lib -- fabric_simulation`: 24 passed. Coverage:
  - the reference run binds its evidence, and the readback status equals the
    evaluated status;
  - all six local results pass;
  - repeated runs repeat projection, novelty, evidence, and run refs;
  - final state refs are canonical;
  - crash recovery in both causal worlds passes durability, with liveness not
    evaluated under the crash;
  - profile drift and unknown fields are refused;
  - novelty guidance scores a repeated seed as nothing new;
  - counters compare exactly within one cohort and refuse another schedule;
  - the consumer contract matches the core field list and domains.
- CLI smoke (`cargo run -- fabric-simulation run` then `inspect`) printed:
  `fabric-simulation protocol verdict=incomplete ... projections=9 gaps=0
  safety-pass=6 safety-fail=0 safety-incomplete=0 safety-not-evaluated=3
  liveness-pass=0 liveness-fail=0 liveness-open=3 novelty=9 first-failure=none`.
  - The reference run covers 6 virtual ticks against the profile's 64-tick
    horizon, so liveness stays incomplete instead of passing.
  - Durability is not evaluated, because no recovery happens in that world.

## Scope notes

- The reference world runs one participant per service. Pairwise and cohort
  agreement are therefore not evaluated there, and are exercised by core
  fixtures with two replicas.
- The local guard runs in `ServiceExecutor`, the only executor of the
  reference extension. It is the same function the oracle applies.

## Non-claims

- Simulation results do not prove KVM, live, production, or release behavior.
- Oracle results hold only for the recorded world, scheduler, workload,
  faults, and projections.
- Work counters are deterministic algorithmic observations, not hardware
  performance.
- An independent oracle can share a defect with the implementation it checks.

## Gates run

- `cargo test --workspace --lib`: 1733 (`molten`) and 619 (`molten-core`)
  passed. `cargo test --test cliharness fabric_simulation`: 2 passed.
  `cargo test --test fabric_simulation_boundary`: 3 passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean. `cargo octet
  check`: clean.
- `nix build .#checks.x86_64-linux.protocol-oracle-profile`: built.
- Cairn sync created the accepted `fabric-simulation` requirements. Cairn
  strict validation reported no spec or substance findings. The tracey guard
  passed with 2953 requirements, 1019 referenced, and 0 dangling.
