# Bounded Execution Fabric Port Specification Delta

## ADDED Requirements

### Requirement: Bounded Exec is pinned as a reusable mechanism

r[molten.fabric_execution.component_pin] Molten MUST consume the reviewed `bounded-exec` component through immutable source and build identities without copying its source or broadening its claim boundary.

#### Scenario: Reviewed component is selected

- GIVEN the dependency matches revision `29dac88ecded94457572db3fdfaaaab95fa91525` and its reviewed source coordinate
- WHEN the live execution adapter builds
- THEN dependency evidence MUST bind that exact source, license, package, and platform profile.

#### Scenario: A mutable sibling path is the only source

- GIVEN configuration names only a mutable workspace checkout
- WHEN production or evidence-bearing admission runs
- THEN admission MUST deny before the adapter becomes selectable.

### Requirement: Execution is a distinct application-owned fabric port

r[molten.fabric_execution.port_contract] Molten MUST expose bounded native execution through a narrow application-owned port with canonical requests, lifecycle observations, typed failures, exact profile bindings, and explicit non-claims.

#### Scenario: A compatible port is resolved

- GIVEN a request names the exact execution port, version, schemas, profile, authority, resources, and conformance refs
- WHEN the fabric registry resolves the request
- THEN it MUST return one exact execution binding
- AND no transport, scheduling, supervision, or fixture port may substitute.

#### Scenario: An adapter defines the port contract

- GIVEN a live or simulation adapter module defines the application port or process policy
- WHEN architecture validation runs
- THEN validation MUST fail with the ownership violation.

### Requirement: Execution authority is explicit

r[molten.fabric_execution.authority] A process request MUST bind executable authorization, provenance, effect admission, workspace authority, process authority, resource grants, active generation, and policy evidence before spawn.

#### Scenario: Complete authority is admitted

- GIVEN every required authority and evidence ref matches the executable, workspace, operation, generation, and profile
- WHEN execution admission runs
- THEN it MUST produce an admitted execution plan.

#### Scenario: Executable possession is the only authority

- GIVEN the caller has executable bytes but lacks any required authority, provenance, policy, or resource binding
- WHEN execution admission runs
- THEN it MUST deny before artifact resolution or process spawn.

### Requirement: Requests bind all process inputs and limits

r[molten.fabric_execution.request] Each request MUST bind the executable artifact, arguments, logical workspace, input ref, environment, timeout, input and output bounds, poll interval, teardown bound, termination scope, outcome policy, and idempotency identity.

#### Scenario: Complete request is valid

- GIVEN every input and limit is present, supported, and within the selected profile
- WHEN pure request validation runs
- THEN it MUST return the same deterministic admitted plan for equal facts.

#### Scenario: A required limit is absent or overbound

- GIVEN any timeout, input, output, polling, teardown, argument, environment, or concurrency limit is missing or exceeds the profile
- WHEN request validation runs
- THEN it MUST deny before allocating capture buffers or starting a process.

### Requirement: Production environments are explicit

r[molten.fabric_execution.environment] The initial production profile MUST clear the inherited environment and allow only explicit bounded key-value entries admitted by policy.

#### Scenario: Cleared environment is used

- GIVEN the request contains only admitted explicit environment entries
- WHEN the live adapter constructs the process request
- THEN the child MUST receive no undeclared inherited variable.

#### Scenario: Request asks for inheritance or shell expansion

- GIVEN a request asks for inherited environment, path search, shell expansion, implicit current directory, or secret bytes
- WHEN environment admission runs
- THEN it MUST deny before spawn.

### Requirement: Process lifecycle observations remain typed

r[molten.fabric_execution.lifecycle] Molten MUST distinguish admitted, queued, started, exited, timed-out, cancelled, failed-before-start, failed-after-start, teardown-incomplete, and unknown execution states.

#### Scenario: Process exits with an observed code

- GIVEN an admitted process starts and exits with a recorded code or signal
- WHEN the adapter reports completion
- THEN the outcome MUST preserve that observation and terminal lifecycle state
- AND it MUST NOT infer application success.

#### Scenario: Cancellation races with completion

- GIVEN cancellation and child completion can race
- WHEN the adapter observes the terminal boundary
- THEN it MUST report the observed terminal state or unknown state
- AND it MUST NOT fabricate cancellation success.

### Requirement: Output capture and publication are bounded

r[molten.fabric_execution.output] Standard output and standard error MUST have independent capture bounds, retained-prefix facts, truncation facts, byte counts, content refs, and publication outcomes.

#### Scenario: Output fits the selected bounds

- GIVEN both output streams remain within their capture and publication limits
- WHEN the process completes and content publication succeeds
- THEN the execution receipt MUST bind both stream refs, counts, truncation flags, and content receipts.

#### Scenario: Child floods an output stream

- GIVEN a child writes beyond an output bound
- WHEN capture reaches the limit
- THEN the adapter MUST retain only the admitted prefix and mark truncation
- AND memory use MUST remain within the reserved capture capacity.

#### Scenario: Content publication fails

- GIVEN process execution completed but output publication fails
- WHEN the shell builds the final observation
- THEN it MUST preserve the process result and report publication failure
- AND it MUST NOT claim the missing output is available.

### Requirement: Generation and operation linkage fence completions

r[molten.fabric_execution.generation] Every execution request and completion MUST bind extension, service, generation, callback, effect, operation, executable, profile, and idempotency identities.

#### Scenario: Matching completion is delivered

- GIVEN the completion matches every active request identity and generation
- WHEN completion admission runs
- THEN it MAY reach the consuming extension callback.

#### Scenario: Completion belongs to a replaced generation

- GIVEN a completion names an inactive generation, substituted executable, or different operation
- WHEN completion admission runs
- THEN it MUST deny before extension state changes.

### Requirement: Unknown outcomes require reconciliation

r[molten.fabric_execution.uncertainty] Failure after process start without definitive completion and teardown evidence MUST remain unknown and MUST NOT trigger automatic retry.

#### Scenario: Host fails after process start

- GIVEN the adapter recorded process start but lost completion or teardown observation
- WHEN recovery evaluates the operation
- THEN it MUST report unknown with the exact operation identity
- AND the consuming service MUST reconcile before any retry decision.

#### Scenario: Failure occurs before spawn

- GIVEN artifact, authority, resource, or process setup fails before spawn
- WHEN the adapter reports failure
- THEN it MUST classify a definite pre-start failure
- AND it MUST record that no child start was observed.

### Requirement: Simulation preserves the execution command algebra

r[molten.fabric_execution.simulation] Molten MUST provide a deterministic execution adapter that consumes the live canonical request and emits scripted canonical lifecycle observations without spawning a process.

#### Scenario: Equal simulation inputs replay

- GIVEN equal admitted requests, scripted observations, initial state, profile, and seed
- WHEN deterministic simulation runs twice
- THEN canonical plans, lifecycle events, receipts, and final state refs MUST match.

#### Scenario: Simulation uses a mock-only request

- GIVEN a simulation bypasses the live request, lifecycle, or outcome contract
- WHEN same-core conformance runs
- THEN conformance MUST fail before simulation evidence can satisfy a fabric gate.

### Requirement: Execution conformance covers positive and negative paths

r[molten.fabric_execution.validation] Molten MUST test pure admission, live execution, simulation, cancellation, timeout, output flood, teardown, generation, uncertainty, dependency direction, and receipt validation.

#### Scenario: Live and simulation adapters conform

- GIVEN each adapter implements the same versioned port contract
- WHEN shared conformance runs
- THEN required positive and negative cases MUST pass for both supported profiles.

#### Scenario: A required negative case is absent

- GIVEN no test covers missing authority, overbound output, timeout, cancellation, stale generation, teardown failure, or unknown completion
- WHEN change closeout runs
- THEN the change MUST remain incomplete.

### Requirement: Execution evidence retains non-claims

r[molten.fabric_execution.nonclaims] Execution profiles, bindings, observations, and receipts MUST NOT claim sandboxing, hermeticity, executable trust, child correctness, network isolation, platform equivalence, application success, or release readiness.

#### Scenario: Process exits successfully

- GIVEN a child exits with a consumer-accepted code and all output is retained
- WHEN operator status renders the result
- THEN status MUST describe a bounded process observation
- AND it MUST retain all execution and application non-claims.
