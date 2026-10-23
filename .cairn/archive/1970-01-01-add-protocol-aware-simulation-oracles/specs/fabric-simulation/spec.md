# Fabric Simulation Delta

## ADDED Requirements

### Requirement: Protocol projections are canonical and consumer-owned
r[molten.fabric_simulation.protocol_projection] A protocol-aware extension MUST declare a versioned projection contract. Each record MUST bind the protocol, service, participant, generation, source sequence, scheduler choice, transition class, logical position, canonical state, history, durability, progress, and observation-completeness facts required by that contract. Runtime projections MUST use canonical Preserves values and domain-separated BLAKE3 refs. Evidence-bearing identity MUST NOT depend on Rust debug formatting or a process-local hash.

#### Scenario: Canonical projection repeats
- GIVEN identical admitted extension state, projection schema, scheduler position, and transition facts
- WHEN the pure projector runs in separate processes
- THEN the canonical projection and BLAKE3 ref match.

#### Scenario: Debug text is the only state identity
- GIVEN a protocol record whose state identity comes only from Rust debug output
- WHEN protocol-aware admission runs
- THEN admission rejects the record as non-canonical.

#### Scenario: Required participant sequence has a gap
- GIVEN a lossless profile and a missing source sequence from one required participant
- WHEN cohort assembly runs
- THEN the cohort is incomplete and cannot support a passing oracle result.

### Requirement: Protocol oracle evaluation is independent from self-report
r[molten.fabric_simulation.oracle_independence] A protocol oracle MUST derive its result from admitted projection values and explicit preconditions. It MUST have an identity separate from the transition artifact under test. A reported invariant name, boolean pass field, callback receipt, or transition-owned pass list MUST NOT establish success without independent evaluation.

#### Scenario: Transition falsely reports success
- GIVEN a transition reports an invariant as passed but two canonical projections disagree at one shared logical position
- WHEN the independent oracle evaluates the cohort
- THEN it reports the matching safety failure.

#### Scenario: Oracle input is missing
- GIVEN an oracle requires one participant projection that was not observed
- WHEN evaluation runs
- THEN it returns incomplete instead of pass.

### Requirement: Protocol safety results preserve their level and first failure
r[molten.fabric_simulation.protocol_safety] Protocol-aware evaluation MUST distinguish local transition safety, pairwise agreement, whole-cohort safety, and selected durability properties. It MUST retain the earliest observed failure even when later projections agree. Physical byte equality MUST apply only when the selected adapter contract declares canonical physical layout.

#### Scenario: Replicas diverge and later converge
- GIVEN two participants disagree at one shared logical position and agree at a later position
- WHEN safety evaluation completes
- THEN the earlier pairwise failure remains failed with its first scheduler and projection refs.

#### Scenario: Semantic state matches across different physical adapters
- GIVEN two admitted adapters produce the same semantic projection but different physical layouts
- WHEN the profile does not require canonical physical layout
- THEN semantic agreement can pass and physical equality is not evaluated.

### Requirement: Local protocol guards keep bounded runtime behavior
r[molten.fabric_simulation.local_protocol_guards] Cheap local protocol guards MUST be pure and MUST use the same admitted semantic function in simulation and each supported live profile. A guard violation MUST return a typed invariant result to the shell. Normal malformed input, policy denial, resource denial, and unsupported behavior MUST remain typed non-invariant results.

#### Scenario: Impossible local transition occurs
- GIVEN an admitted transition violates a declared local safety invariant
- WHEN the local guard evaluates the new state
- THEN it returns the stable invariant violation for the shell's admitted quarantine, denial, or fail-stop policy.

#### Scenario: Malformed command is denied
- GIVEN a command fails normal schema admission
- WHEN the extension processes it
- THEN it returns the typed admission error and does not relabel it as an internal invariant failure.

### Requirement: Participant liveness is conditional and explicit
r[molten.fabric_simulation.participant_liveness] A participant liveness oracle MUST return pass, fail, not-evaluated, or incomplete. It MUST bind participant identity, readiness, required membership or quorum, disruptive-fault state, durability facts, progress measure, start position, and finite virtual-progress horizon. Aggregate cohort progress MUST NOT hide a failed eligible participant.

#### Scenario: Eligible participant does not progress
- GIVEN one participant remains ready, connected to its required peers, free of disruptive faults, and observed for the full virtual-progress horizon
- WHEN its declared progress measure does not advance
- THEN that participant's liveness result fails.

#### Scenario: Partition still blocks required peers
- GIVEN an active admitted partition denies a participant's required communication
- WHEN the liveness oracle evaluates that interval
- THEN it returns not-evaluated for the affected progress property.

#### Scenario: Required progress observation is lost
- GIVEN stabilization facts are present but a required participant observation is missing
- WHEN liveness evaluation runs
- THEN it returns incomplete instead of pass or fail.

### Requirement: Protocol novelty identity is stable and bounded
r[molten.fabric_simulation.protocol_novelty] A protocol-aware profile MUST select the canonical projection fields that define novelty. The pure core MUST compute a domain-separated BLAKE3 identity over those fields and MUST retain the full identity in evidence. Coverage slots or process-local hashes MAY guide one run but MUST NOT serve as durable novelty identity.

#### Scenario: Equivalent protocol state repeats
- GIVEN two runs reach the same selected canonical protocol fields under the same schema
- WHEN novelty identity is computed
- THEN both runs produce the same full identity.

#### Scenario: Selected logical state changes
- GIVEN one selected canonical protocol field changes
- WHEN novelty identity is computed
- THEN the new identity differs from the prior recorded identity.

### Requirement: Deterministic protocol costs use declared work counters
r[molten.fabric_simulation.protocol_cost] A protocol-aware profile MAY declare named monotonic work counters. Comparisons MUST bind identical world, runtime, scheduler, workload, fault, counter-schema, and bound refs. Counter results MUST NOT claim wall-clock latency, hardware throughput, production capacity, or environment-independent performance.

#### Scenario: Algorithmic work decreases under one cohort
- GIVEN two implementations run the same admitted deterministic cohort and expose compatible counter schemas
- WHEN comparison evaluates their message, transition, storage, byte, repair, or round counters
- THEN it reports the exact bounded counter differences without a hardware-performance claim.

#### Scenario: Counter decreases within one run
- GIVEN a declared monotonic counter has a lower later value without a generation reset allowed by its schema
- WHEN counter admission runs
- THEN it rejects the observation as malformed or incomplete.

### Requirement: Protocol oracle evidence fails closed
r[molten.fabric_simulation.protocol_oracle_evidence] Protocol-aware evidence MUST bind the world, runtime, projection schema, oracle, participants, scheduler, workload, faults, completeness, safety, liveness, novelty, work counters, replay, and non-claims. Incomplete, conflicting, unsupported, stale, or unbounded inputs MUST remain distinct from pass. Simulation evidence MUST NOT satisfy KVM, live, production, release, or hardware-performance claims.

#### Scenario: Complete protocol-aware run exports
- GIVEN an admitted bounded run has complete observations and stable oracle results
- WHEN evidence exports
- THEN the bundle binds every required identity, result, first failure, and non-claim.

#### Scenario: Simulation result is promoted to live proof
- GIVEN a passing deterministic simulation bundle without the required live profile evidence
- WHEN claim admission evaluates a live or production label
- THEN it rejects the promotion.

### Requirement: Protocol oracle validation is adversarial
r[molten.fabric_simulation.protocol_oracle_validation] Validation MUST include positive and negative fixtures for projection admission, oracle independence, safety levels, local guards, participant liveness, novelty, counters, replay, evidence, and claim boundaries.

#### Scenario: False success fixture runs
- GIVEN a transition self-reports success while its canonical projections violate a declared invariant
- WHEN focused validation runs
- THEN the independent oracle reports the expected failure.

#### Scenario: Closeout validation runs
- GIVEN maintainers intend to sync or archive protocol-aware simulation oracles
- WHEN focused tests, lifecycle gates, and the selected repository checks run
- THEN every positive and negative fixture produces its expected stable result.
