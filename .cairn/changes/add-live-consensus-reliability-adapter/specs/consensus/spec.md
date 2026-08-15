## ADDED Requirements

### Requirement: The live register profile is exact and bounded
r[molten.consensus.live_register_profile] Molten MUST define a versioned live service-registry register profile. It MUST bind group, service, key, initial value, value corpus, generator identity, seed, operation weights, clients, concurrency, operation bounds, consistency model, recovery conditions, and evidence limits. Missing or unbounded required inputs MUST deny execution.

#### Scenario: Complete register profile is admitted
- GIVEN the profile binds one admitted service-registry key and every required finite bound
- WHEN workload admission runs
- THEN it MUST produce a deterministic pre-effect workload plan and profile ref.

#### Scenario: Several keys are supplied
- GIVEN the initial profile names more than one service-registry key
- WHEN workload admission runs
- THEN it MUST reject the profile instead of implying a cross-key model.

### Requirement: The live operation generator is pure and reproducible
r[molten.consensus.live_operation_generator] Molten MUST generate bounded read and write plans from the admitted profile. Equal profile and seed inputs MUST produce equal operation plans and choice records. Generation MUST NOT perform service, network, filesystem, process, clock, random-source, fault, or checker effects.

#### Scenario: Equal generator inputs match
- GIVEN two planners receive the same admitted profile and seed
- WHEN live operation generation runs
- THEN both operation plans and complete choice records MUST match.

#### Scenario: Generated concurrency exceeds the profile
- GIVEN a generated plan exceeds the admitted client or concurrency bound
- WHEN plan validation runs
- THEN it MUST reject the plan before public client effects.

### Requirement: Live operation histories preserve client uncertainty
r[molten.consensus.live_operation_history] Molten MUST map public writes and reads to paired semantic-history events. Logical operation identity MUST survive retries. Attempts MUST have distinct identities. Acknowledged results MUST map to `ok`, definite pre-effect denials to `fail`, and timeout, disconnect, or lost response to `info`.

#### Scenario: Commit precedes lost response
- GIVEN a write commits before its client connection fails
- WHEN the adapter records the client outcome
- THEN it MUST use `info` and preserve the operation identity across retry.

#### Scenario: Transport error becomes definite failure
- GIVEN the adapter cannot prove that a transport error occurred before service admission
- WHEN it maps the outcome
- THEN it MUST NOT classify the operation as `fail`.

### Requirement: The live adapter uses public coordination paths
r[molten.consensus.live_public_adapter] The product adapter MUST perform setup, writes, reads, recovery, final reads, and teardown through admitted public coordination and node-service endpoints. It MUST NOT read consensus internals, replica storage, process memory, or private receipts to decide model outcomes.

#### Scenario: Write uses public coordination request
- GIVEN an admitted service-registry update request
- WHEN the live adapter invokes the write
- THEN it MUST use the public control-plane path and record only the client-visible result in semantic history.

#### Scenario: Adapter checks a replica log
- GIVEN an adapter uses a local replica log to decide that a public read is current
- WHEN black-box adapter conformance runs
- THEN it MUST reject the adapter for live semantic evidence.

### Requirement: The initial live fault profile is bounded
r[molten.consensus.live_fault_profile] Molten MUST declare a no-fault control and bounded process-restart, temporary-partition, heal, and recovery profiles. Clock, durability, Byzantine, queue, lock, and transaction faults or models MUST remain unsupported until separate admitted profiles define their semantics.

#### Scenario: Temporary partition heals
- GIVEN a profile declares exact partition sides, effect observations, heal conditions, and recovery bounds
- WHEN the live fault profile runs
- THEN its evidence MUST retain selected, applied, observed, healed, and recovery facts.

#### Scenario: Durability fault is requested
- GIVEN the initial profile requests unflushed-write loss
- WHEN Molten workload admission runs
- THEN it MUST reject the unsupported profile without claiming durability coverage.

### Requirement: Live recovery requires final public reads
r[molten.consensus.live_recovery] After disruptive faults heal, Molten MUST require declared service readiness and stable membership facts. The adapter MUST then read the selected key through every admitted public endpoint. Missing, conflicting, or over-bound observations MUST block a valid verdict.

#### Scenario: Replicas converge after heal
- GIVEN every endpoint becomes ready after a temporary partition
- WHEN final public reads run
- THEN every read MUST enter the semantic history for model evaluation.

#### Scenario: One endpoint remains unavailable
- GIVEN one admitted endpoint cannot complete its required final read
- WHEN the recovery bound expires
- THEN the result MUST report incomplete recovery and MUST NOT report valid linearizability.

### Requirement: Live register histories receive bounded linearizability verdicts
r[molten.consensus.live_linearizability] Molten MUST evaluate complete admitted live histories with the pinned register model and checker. It MUST retain `valid`, `invalid`, or `unknown` without promotion. A reference-checker disagreement MUST block promotion and preserve both reports.

#### Scenario: Stale public read violates the model
- GIVEN a complete history contains a read that cannot appear in any legal real-time register order
- WHEN linearizability evaluation runs
- THEN it MUST retain an invalid verdict and bounded witness.

#### Scenario: Checker search is incomplete
- GIVEN the checker exhausts an admitted search bound
- WHEN Molten classifies the result
- THEN it MUST retain `unknown` and MUST NOT report a valid service history.
