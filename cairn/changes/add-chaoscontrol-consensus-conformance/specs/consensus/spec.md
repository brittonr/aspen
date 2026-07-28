## ADDED Requirements

### Requirement: Committed transitions expose a chain observation
r[molten.consensus.chaoscontrol_chain_observation] Molten MUST bind a canonical initial-state ref and project bounded chain observations from committed application transitions. Each observation MUST bind group, replica, command index, operation, command, prior digest, next digest, state ref, and lifecycle generation. Projection MUST use the exact admitted ChaosControl hash contract. Accepted conformance MUST use lossless observation mode and bounded dropped-event accounting.

#### Scenario: Replicas apply the same command
- GIVEN two admitted replicas have the same prior digest and commit the same canonical command at the same index
- WHEN each projects its chain observation
- THEN both next digests match and each observation retains its own replica and state refs.

#### Scenario: Observation bypasses committed apply
- GIVEN a test adapter fabricates a chain digest without invoking the committed Molten application path
- WHEN observer-path conformance runs
- THEN the observation is rejected and cannot support external conformance evidence.

#### Scenario: Lossless observer drops a transition
- GIVEN the conformance profile requires lossless observations and dropped-event accounting reports a gap
- WHEN observer-path conformance runs
- THEN external conformance is blocked as an observer failure
- AND the gap is not relabeled as a consensus-safety violation.

### Requirement: Logical operation identity survives uncertain outcomes
r[molten.consensus.chaoscontrol_operation_identity] Molten MUST preserve the client-session and sequence identity of one logical command across acknowledgement, definite rejection, timeout, disconnect, retry, and recovery. Timeout or disconnect MUST remain an indefinite execution outcome until committed history resolves it.

#### Scenario: Lost acknowledgement follows commit
- GIVEN a command commits before the client connection fails
- WHEN the client records an indefinite result and retries
- THEN the retry keeps the same logical operation identity
- AND committed application occurs at most once.

#### Scenario: Retry changes sequence identity
- GIVEN a client retries one logical operation with a different session or sequence identity
- WHEN workload admission evaluates the trace
- THEN it rejects the trace as an invalid idempotency input.

### Requirement: The KVM fault profile is explicit and bounded
r[molten.consensus.chaoscontrol_fault_profile] Molten MUST declare a no-fault control and finite supported fault profiles for external consensus campaigns. The initial profiles MAY cover message loss, reorder, partition, quorum loss, node crash, restart, and snapshot catch-up. Unsupported disk, clock, or Byzantine faults MUST remain explicit.

#### Scenario: Supported partition profile runs
- GIVEN a profile declares a temporary partition, recovery condition, finite bounds, and expected observation path
- WHEN the external campaign runs
- THEN its receipt identifies selected, applied, and observed fault outcomes plus the recovery boundary.

#### Scenario: Unsupported fault is requested
- GIVEN a profile requests Byzantine behavior or an undefined durability fault
- WHEN Molten or ChaosControl preflight evaluates the profile
- THEN execution denies or records unsupported
- AND no receipt claims that the fault affected consensus.

### Requirement: Chain safety remains continuous
r[molten.consensus.chaoscontrol_safety] Molten MUST require equal chain digests and canonical application-state refs for every shared command index. It MUST also require stable prior observations, monotonic commit, apply-once operation identity, and deterministic recovered state refs.

#### Scenario: Digests diverge at one index
- GIVEN two replicas report different chain digests at the same command index
- WHEN later observations match at a higher index
- THEN the original safety violation remains failed.

#### Scenario: Restart preserves committed state
- GIVEN a replica restarts from an admitted snapshot and log suffix
- WHEN it catches up to an observed command index
- THEN its chain digest and application state ref match the other correct replicas at that index.

### Requirement: External liveness is conditional
r[molten.consensus.chaoscontrol_liveness] Molten MUST evaluate external progress only after a declared stable quorum, inactive disruptive faults, admitted lifecycle state, and named virtual progress horizon. Quorum-loss intervals MUST NOT become unconditional liveness failures.

#### Scenario: Stable quorum resumes progress
- GIVEN a temporary partition heals and an admitted quorum becomes ready
- WHEN a valid command commits within the named virtual progress horizon
- THEN the bounded liveness condition passes.

#### Scenario: Majority remains unavailable
- GIVEN the active fault profile still denies majority communication
- WHEN no command commits
- THEN the result records expected unavailability for that interval
- AND it does not report a chain-safety failure.
