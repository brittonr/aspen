## ADDED Requirements

### Requirement: Consensus acceleration profiles compose with exact base engines
r[molten.consensus.fast_path_acceleration.profile] Molten MUST represent an optional consensus acceleration profile with explicit acceleration identity and version, exact base algorithm and implementation identities, compatibility cohort, conflict-contract ref, quorum and recovery policy, supported command/read/membership/fault capabilities, environment and topology scope, resource envelope, routing profile, evidence refs, enablement posture, rollback posture, and non-claims. Acceleration MUST be opt-in and MUST NOT silently replace, promote, or weaken the base engine.

#### Scenario: Exact composite profile resolves
- GIVEN a group explicitly selects an admitted acceleration profile whose base implementation, membership, command domain, environment, and evidence cohort match the running group
- WHEN runtime construction resolves the profile
- THEN it constructs that exact composite implementation
- AND preserves the base engine as the original and recovery path.

#### Scenario: Missing acceleration does not change the base engine
- GIVEN a group selects only an admitted base engine and no acceleration profile
- WHEN the group starts
- THEN no fast-path traffic or recovery state is created
- AND base-engine behavior is not silently wrapped.

### Requirement: Acceleration compatibility fails closed
r[molten.consensus.fast_path_acceleration.compatibility] Molten MUST deny an acceleration profile that is unknown, disabled, stale, evidence-incomplete, incompatible with the exact base implementation, outside its membership/topology/fault scope, unsupported for the command or read domain, or missing required conflict and recovery capabilities. Unavailable acceleration MAY fall back to the already admitted base path only when typed policy permits fallback and MUST NOT fabricate acceleration success.

#### Scenario: Wrong base implementation denies
- GIVEN an acceleration profile was evidenced against one exact Raft implementation but the group runs another implementation or version
- WHEN compatibility admission runs
- THEN acceleration denies before protocol activation
- AND the admitted base group remains separately identifiable.

#### Scenario: Replica loss disables only the three-node fast path
- GIVEN an admitted three-replica crash-fault profile loses one replica while the base majority remains available
- WHEN compatibility and routing status are evaluated
- THEN fast-path attempts stop or fall back
- AND status reports base-path availability separately from fast-path unavailability.

### Requirement: Base engines satisfy acceleration ordering prerequisites
r[molten.consensus.fast_path_acceleration.base_prerequisites] A live acceleration profile MUST bind evidence that its exact base implementation preserves proposal order in log and execution order for conflicting commands proposed by one proposer and preserves proposer receive order in proposal order. If a buffering layer can reorder receipt and proposal, fast acknowledgement MUST wait for equivalent proposal-order evidence. If the base engine can reorder conflicting proposals at execution, the transparent acceleration profile MUST deny compatibility.

#### Scenario: Append-ordered base engine passes the prerequisite
- GIVEN the exact base implementation appends conflicting commands in proposer order and commits and executes them in log order
- WHEN composite compatibility admission validates its evidence
- THEN the ordering prerequisite may pass subject to all remaining profile requirements.

#### Scenario: Receive-time promise is invalid across a reordering buffer
- GIVEN the acceleration shim observes command A before command B but the base proposer may buffer and propose B first
- WHEN no proposal-order evidence is available
- THEN a receive-time fast acknowledgement denies or falls back
- AND the profile cannot claim transparent compatibility.

#### Scenario: Execution-reordering engine is rejected
- GIVEN the base engine may execute conflicting command B before earlier-proposed command A
- WHEN acceleration compatibility runs
- THEN the transparent profile denies before activation.

### Requirement: Live acceleration binds an extension-owned conflict contract
r[molten.consensus.fast_path_acceleration.conflict_binding] A live acceleration profile MUST bind a versioned extension-owned pure conflict classifier to exact application and command schemas. The classifier MUST report conflict whenever ordering can change application state or either command response and MUST conservatively fall back for unknown schemas, aliases, ranges, predicates, preconditions, failures, or unsupported operations. It MUST NOT receive engine internals, transport, storage, clocks, runtime executors, or ambient state.

#### Scenario: Conflict contract permits a supported independent command
- GIVEN the admitted contract establishes that two canonical commands are independent within its evidenced domain
- WHEN a fast attempt evaluates the in-flight set
- THEN the classifier may permit the attempt without granting commit by itself.

#### Scenario: Schema drift denies acceleration
- GIVEN a command uses a schema version outside the conflict contract's bound cohort
- WHEN classification runs
- THEN acceleration denies or falls back before issuing a fast acknowledgement.

### Requirement: Dual paths preserve canonical operation identity and normalized outcomes
r[molten.consensus.fast_path_acceleration.dual_path] The live fast and original paths MUST carry the same canonical command, client session and sequence, group, extension/service generation, engine and fencing epochs, application schema, and policy/authority/resource cohort. Extensions MUST receive only normalized commit, denial, retryable, cancelled, or uncertain outcomes with engine-specific evidence opaque. Path convergence MUST apply and authorize the operation at most once.

#### Scenario: Fast success remains one application commit
- GIVEN the fast path returns a valid commit before the original path finishes
- WHEN the original path later orders the same command
- THEN the extension observes one normalized committed operation
- AND duplicate application or authoritative reply is suppressed.

#### Scenario: Conflicting attempt falls back transparently
- GIVEN a valid command cannot satisfy the conflict or fast-quorum condition
- WHEN typed fallback policy permits the original path
- THEN the same operation continues on the base engine
- AND the extension does not need engine-specific retry logic.

### Requirement: Live acceleration preserves view-change recovery ordering
r[molten.consensus.fast_path_acceleration.recovery] A live acceleration profile MUST require same-view fast acknowledgements, compatible promises from every active original-path proposer, an independent acceleration view, recovery of every possibly fast-committed command from the last normal view, agreement and accepted-set carry-forward for recovery state, original-path commit of the recovery set or an explicit no-op marker, and completion of that boundary before admitting new-view commands. Recovered commands MUST precede every conflicting command that could otherwise execute in the new view.

#### Scenario: New proposer recovers a replied command first
- GIVEN a proposer fails after a client receives a valid fast commit but before original-path commit
- WHEN the next view recovers
- THEN the command is recovered and committed through the base engine before conflicting new-view work is admitted.

#### Scenario: Interrupted recovery cannot admit normal work
- GIVEN recovery accepts a recovery set but fails before its original-path marker commits
- WHEN another view begins
- THEN accepted recovery state carries forward
- AND the group remains outside normal fast admission until the marker commits.

### Requirement: Adaptive path selection is typed and bounded
r[molten.consensus.fast_path_acceleration.adaptive_policy] Molten MUST define adaptive path selection through a typed Nickel profile with named finite observation windows, attempt and probe bounds, topology/locality inputs, fast/base latency observations, conflict outcomes, resource thresholds, health requirements, and backoff rules. A pure decision core MUST return original-only or fast-attempt, while a thin shell gathers bounded telemetry and executes the decision. Unreviewed external heuristics or hidden numeric thresholds MUST NOT become defaults.

#### Scenario: Saturation backs off to the original path
- GIVEN bounded telemetry shows the fast path consumes resources beyond the admitted profile while the base path remains healthy
- WHEN routing policy evaluates the next operation
- THEN it reduces or disables fast attempts within configured bounds
- AND records the reason without declaring the base engine unhealthy.

#### Scenario: Missing telemetry fails safely
- GIVEN required health or resource observations are stale or unavailable
- WHEN adaptive policy evaluates an attempt
- THEN it chooses original-only or denies according to explicit policy
- AND does not infer a healthy fast path from missing data.

### Requirement: Production acceleration requires exact live and performance evidence
r[molten.consensus.fast_path_acceleration.production_admission] Molten MUST deny production acceleration until the exact base engine is production-admitted and accepted evidence covers the fast-path hazard model, deterministic same-core simulation, distinct-process live convergence, conflict-domain semantics, static membership and placement, transport and durable recovery, partitions and crashes, recovery ordering, stale-epoch fencing, bounded resources, original-path equivalence with acceleration disabled, safe fallback, operator recovery, rollback, and environment-scoped latency/throughput/tail/resource measurements. External code, model checks, or benchmark results MUST NOT satisfy these requirements.

#### Scenario: Measured geo workload admits a bounded profile
- GIVEN the exact implementation and environment pass model, simulation, live failure, conflict, non-regression, resource, recovery, and performance thresholds with operator approval and rollback evidence
- WHEN production admission evaluates that cohort
- THEN it may admit acceleration only for the bound profile and environment.

#### Scenario: No measured benefit denies promotion
- GIVEN correctness evidence passes but the target workload is not RTT-bound or acceleration causes unacceptable throughput, tail-latency, availability, resource, or recovery regression
- WHEN production admission runs
- THEN acceleration remains experimental or disabled
- AND the admitted base engine remains available.

### Requirement: Acceleration evidence is semantic and bounded
r[molten.consensus.fast_path_acceleration.evidence] Molten MUST emit canonical evidence for acceleration and group admission, implementation/configuration/fencing epochs, selected fast commits or commit ranges, fallback classes, recovery sets and markers, material failures, aggregate attempt/success/conflict/latency/resource observations, benchmark decisions, rollback state, and non-claims. The default profile MUST NOT require standalone authority receipts for every fast acknowledgement, replication packet, heartbeat, vote, or timer event.

#### Scenario: Recovery evidence validates offline
- GIVEN a live view change recovers prior fast commands and commits a recovery marker
- WHEN the run directory is verified offline
- THEN evidence binds the group, base and acceleration profiles, views and epochs, recovery set, original-path marker, resulting commit boundary, decision, and non-claims.

#### Scenario: Packet trace is not authoritative commit evidence
- GIVEN diagnostic tracing records fast acknowledgements
- WHEN no valid semantic commit or recovery evidence binds them
- THEN the trace cannot authorize application mutation or production admission.

### Requirement: Initial acceleration non-claims are explicit
r[molten.consensus.fast_path_acceleration.nonclaims] The initial acceleration profile MUST deny dynamic membership, leadership transfer, Byzantine faults, interactive transactions, cross-group atomicity, unsupported range/predicate command semantics, unsupported read modes, and any environment or base implementation outside its admitted cohort. Acceleration evidence MUST NOT prove extension semantic correctness, global ordering, transport security, authority, policy, release readiness, or performance outside measured bounds.

#### Scenario: Interactive transaction is rejected
- GIVEN a workflow requires multiple interactive read/write rounds or atomic commit across groups
- WHEN it requests the initial fast-path profile
- THEN acceleration denies as unsupported
- AND does not reinterpret the workflow as independent commands.

### Requirement: Acceleration validation covers success, failure, and non-regression
r[molten.consensus.fast_path_acceleration.validation] Molten MUST include positive and negative tests for descriptor identity and compatibility, conflict binding, normalized outcomes, canonical operation identity, duplicate suppression, same-view quorum and proposer promises, fallback, partitions, quorum loss, proposer crash, durable restart, interrupted and cascading recovery, recovery markers, static-membership limits, adaptive routing, stale telemetry, resource exhaustion, cancellation, drain, evidence, operator rollback, base-path equivalence, performance admission, non-claims, and production denial.

#### Scenario: Disabled acceleration matches the base profile
- GIVEN identical admitted base groups and workloads with one group lacking acceleration and one selecting acceleration in disabled mode
- WHEN the equivalence suite executes
- THEN canonical base outcomes and declared performance/resource bounds match within the reviewed profile
- AND the disabled group emits no fast-path protocol traffic.

#### Scenario: False recovery ordering fails validation
- GIVEN a fixture admits a conflicting new-view command before the prior-view recovery marker
- WHEN live or simulated validation runs
- THEN it fails the recovery-order invariant
- AND cannot satisfy production acceleration admission.
