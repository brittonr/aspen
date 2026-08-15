## Phase 1: Composite profile and contracts

- [ ] [serial] [depends:model-consensus-fast-path-hazards] Define typed Nickel acceleration descriptors binding exact base algorithm/implementation identities, acceleration identity, compatibility cohort, conflict contract, quorum/recovery policy, topology, environment, resources, evidence refs, enablement posture, and non-claims. r[molten.consensus.fast_path_acceleration.profile]
- [ ] [depends:fast-path-acceleration-profile] Implement pure descriptor compatibility and admission that checks base receive/propose/execute ordering prerequisites and denies unknown, disabled, stale, evidence-incomplete, wrong-base, wrong-membership, wrong-command-domain, live-incompatible, or silently defaulted profiles. r[molten.consensus.fast_path_acceleration.profile] r[molten.consensus.fast_path_acceleration.compatibility] r[molten.consensus.fast_path_acceleration.base_prerequisites]
- [ ] [parallel] Add positive exact-composition fixtures and negative receive/propose reorder, proposal/execution reorder, wrong-base, stale-profile, missing-model, missing-live-engine, unsupported-capability, fallback-substitution, and claim-overreach fixtures. r[molten.consensus.fast_path_acceleration.compatibility] r[molten.consensus.fast_path_acceleration.base_prerequisites] r[molten.consensus.fast_path_acceleration.nonclaims]

## Phase 2: Extension conflict and normalized port boundaries

- [ ] [serial] Add a versioned extension-owned pure conflict-classifier contract bound to application and command schemas, with conservative unknown handling and no engine/runtime handles. r[molten.consensus.fast_path_acceleration.conflict_binding]
- [ ] [parallel] Keep extension outcomes normalized and engine-private while binding both paths to one canonical command/session/group/generation/epoch and policy/authority/resource cohort. r[molten.consensus.fast_path_acceleration.dual_path]
- [ ] [parallel] Add positive independent-command and fallback fixtures plus negative false-non-conflict, schema drift, unresolved alias/range, identity mismatch, duplicate application, and engine-internal leakage fixtures. r[molten.consensus.fast_path_acceleration.conflict_binding] r[molten.consensus.fast_path_acceleration.dual_path]

## Phase 3: Live acceleration shell and recovery

- [ ] [serial] [depends:fabric-consistency-service-runtime] Implement the supervised fast-path shell over admitted transport, durable state, time, membership, placement, fencing, resource, and base-engine ports without changing extension state semantics. r[molten.consensus.fast_path_acceleration.dual_path]
- [ ] [depends:fast-path-acceleration-shell] Implement same-view superquorums, all-active-proposer promises, original-path fallback, independent acceleration views, recovery-set agreement, recovery/no-op markers, accepted-set carry-forward, and recovery-before-new-view admission. r[molten.consensus.fast_path_acceleration.recovery]
- [ ] [parallel] Add static-membership three-replica and five-replica service profiles and explicit denial for dynamic membership, leadership transfer, Byzantine faults, interactive transactions, cross-group atomicity, and unsupported read/command classes. r[molten.consensus.fast_path_acceleration.compatibility] r[molten.consensus.fast_path_acceleration.nonclaims]

## Phase 4: Adaptive routing and evidence

- [ ] [serial] Define typed Nickel routing profiles with named observation windows, attempt/probe bounds, resource thresholds, topology inputs, conflict observations, and backoff behavior; implement a pure decision core and thin telemetry shell. r[molten.consensus.fast_path_acceleration.adaptive_policy]
- [ ] [parallel] Emit bounded profile/group admission, fast-commit range, fallback, recovery, view/epoch, aggregate health/resource, and benchmark evidence without per-message authority receipts. r[molten.consensus.fast_path_acceleration.evidence]
- [ ] [parallel] Add operator status and dry-run enable/disable workflows that distinguish base-path health, fast-path health, three-node availability limits, current routing decision, recovery state, resource use, evidence refs, and non-claims. r[molten.consensus.fast_path_acceleration.evidence] r[molten.consensus.fast_path_acceleration.nonclaims]

## Phase 5: Simulation, live failure, and performance admission

- [ ] [serial] [depends:fabric-whole-system-simulation] Run the same acceleration core through deterministic simulation for conflict, fallback, mixed-view, leader-failure-after-reply, stale-conflict, partition, quorum loss, crash/restart, interrupted recovery, cascading recovery, resource exhaustion, cancellation, and drain schedules. r[molten.consensus.fast_path_acceleration.production_admission]
- [ ] [depends:fast-path-acceleration-recovery] Add distinct-process live tests for fast/original convergence, original-only equivalence, partitions, proposer crash, durable restart, recovery markers, stale-epoch fencing, and three-node/five-node availability. r[molten.consensus.fast_path_acceleration.production_admission] r[molten.consensus.fast_path_acceleration.validation]
- [ ] [depends:fast-path-acceleration-live-tests] Add environment-scoped benchmarks comparing base, acceleration-disabled, fixed-attempt diagnostic, and adaptive profiles across latency, throughput, tail latency, CPU, memory, network, contention, locality, and failure recovery; record accepted thresholds in typed policy rather than code. r[molten.consensus.fast_path_acceleration.production_admission]
- [ ] [depends:fast-path-acceleration-benchmarks] Add fail-closed production admission requiring the exact live base profile, model/simulation/live evidence, conflict-domain coverage, original-path non-regression, bounded recovery/resource impact, measured workload benefit, operator approval, and rollback posture. r[molten.consensus.fast_path_acceleration.production_admission]

## Phase 6: Validation

- [ ] [depends:fast-path-acceleration-production-admission] Run positive and negative descriptor, base-ordering prerequisite, conflict, port, identity, recovery, adaptive-policy, simulation, live-failure, benchmark, evidence, non-claim, rollback, and production-admission suites. r[molten.consensus.fast_path_acceleration.validation]
- [ ] [serial] [depends:fast-path-acceleration-validation] Run focused formatting/tests, Octet checks, Cairn validation and proposal/design/tasks gates, traceability coverage, live cluster evidence, and the smallest relevant Nix checks before sync and archive. r[molten.consensus.fast_path_acceleration.validation]

## Blocker

This package is blocked on completion of `model-consensus-fast-path-hazards`,
`fabric-consistency-service-runtime`, and the required
`fabric-whole-system-simulation` consistency profile. The base live Raft engine
must be production-admitted before acceleration implementation begins. Do not
substitute the in-process model, ambient sockets, fabricated quorum receipts,
external TLA+ results, or external benchmarks for those prerequisites.
