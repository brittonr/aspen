# Tasks: Protocol-aware simulation oracles

## Foundation and contracts

- [x] [serial] Record the current pass-list evaluation, debug-derived final-state identity, accepted simulation contract, related ChaosControl changes, and lifecycle baseline. r[molten.fabric_simulation.protocol_projection] r[molten.fabric_simulation.oracle_independence]
- [ ] [serial] Define a typed Nickel protocol-oracle profile for projection schemas, participants, logical positions, invariants, liveness preconditions, novelty fields, counters, bounds, and non-claims. r[molten.fabric_simulation.protocol_projection] r[molten.fabric_simulation.protocol_oracle_evidence]
- [ ] [serial] Add pure domain types for protocol projections, cohorts, oracle identities, safety levels, participant liveness, completeness, novelty, and work counters. r[molten.fabric_simulation.protocol_projection] r[molten.fabric_simulation.protocol_safety] r[molten.fabric_simulation.participant_liveness]
- [ ] [parallel] Add positive profile and projection fixtures plus negative unknown-field, stale-schema, malformed-ref, duplicate-sequence, generation-drift, overflow, and unbounded-payload fixtures. r[molten.fabric_simulation.protocol_projection] r[molten.fabric_simulation.protocol_oracle_validation]

## Pure projection and oracle core

- [ ] [serial] Implement canonical Preserves projection encoding and domain-separated BLAKE3 refs without Rust debug formatting. r[molten.fabric_simulation.protocol_projection]
- [ ] [serial] Implement bounded cohort assembly by admitted participant and extension-owned logical position. r[molten.fabric_simulation.protocol_projection] r[molten.fabric_simulation.protocol_safety]
- [ ] [serial] Replace pass-list-only semantic evaluation with a separately identified pure oracle over admitted projection values. r[molten.fabric_simulation.oracle_independence]
- [ ] [serial] Implement local, pairwise, cohort, and selected durability safety results with earliest-failure retention. r[molten.fabric_simulation.protocol_safety]
- [ ] [serial] Implement participant liveness with pass, fail, not-evaluated, and incomplete results over explicit stabilization facts. r[molten.fabric_simulation.participant_liveness]
- [ ] [parallel] Add false-self-report, later-convergence, physical-layout-mismatch, stalled-participant, active-fault, missing-precondition, and incomplete-observation tests. r[molten.fabric_simulation.oracle_independence] r[molten.fabric_simulation.protocol_safety] r[molten.fabric_simulation.participant_liveness] r[molten.fabric_simulation.protocol_oracle_validation]

## Same-core integration and guidance

- [ ] [serial] Instrument the reference extension transitions to emit bounded consumer-owned projections from the admitted state-transition path. r[molten.fabric_simulation.protocol_projection]
- [ ] [parallel] Keep cheap local guards available to supported live profiles while routing expensive cohort evaluation through the simulation shell. r[molten.fabric_simulation.local_protocol_guards]
- [ ] [serial] Add stable protocol novelty identities from profile-selected canonical fields and expose them to scheduler guidance. r[molten.fabric_simulation.protocol_novelty]
- [ ] [serial] Add named monotonic deterministic work counters and exact-cohort comparison decisions. r[molten.fabric_simulation.protocol_cost]
- [ ] [parallel] Add positive novelty and cost comparisons plus negative unstable-field, counter-regression, overflow, changed-schedule, and hardware-performance-promotion fixtures. r[molten.fabric_simulation.protocol_novelty] r[molten.fabric_simulation.protocol_cost] r[molten.fabric_simulation.protocol_oracle_validation]

## Evidence and workflows

- [ ] [serial] Bind projection, oracle, participant, scheduler, workload, fault, completeness, result, counter, replay, and non-claim refs into simulation evidence. r[molten.fabric_simulation.protocol_oracle_evidence]
- [ ] [parallel] Add bounded operator status for protocol cohorts, safety levels, participant liveness, incomplete observations, novelty, counters, and first failure. r[molten.fabric_simulation.protocol_oracle_evidence]
- [ ] [serial] Export an immutable consumer adapter contract for later ChaosControl protocol-observation integration without workspace-relative product dependencies. r[molten.fabric_simulation.protocol_oracle_evidence]

## Validation and closeout

- [ ] [serial] Run focused pure, reference-service, replay, evidence, positive, and negative tests. r[molten.fabric_simulation.protocol_oracle_validation]
- [ ] [serial] Run formatting, Clippy, Octet, Cairn validation, proposal, design, tasks gates, and the smallest relevant Nix checks. r[molten.fabric_simulation.protocol_oracle_validation]
- [ ] [serial] Retain simulation, KVM, live, production, hardware-performance, and universal-correctness non-claims before sync or archive. r[molten.fabric_simulation.protocol_oracle_evidence] r[molten.fabric_simulation.protocol_oracle_validation]
