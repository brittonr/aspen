# System Extension Runtime Specification

## Purpose

Defines the `system-extension-runtime` capability.

## Requirements

### Requirement: System extensions have a canonical manifest
r[molten.system_extension.manifest] Aspen MUST define a canonical system-extension manifest distinct from an ordinary plugin manifest. It MUST bind extension and service identity, implementation artifact, supported callback groups, required and optional fabric ports, capability refs, resource envelope, execution profile, state schema, upgrade compatibility, evidence profile, and non-claims. Unknown fields that affect authority or execution, unknown callback groups, incompatible port versions, missing required bindings, or unauthorized execution profiles MUST deny activation.

#### Scenario: Complete manifest is admitted
- GIVEN a manifest has canonical identities, supported callbacks, compatible port requirements, reviewed resources, provenance, policy, and execution profile
- WHEN system-extension admission runs
- THEN it produces a canonical admitted-manifest ref
- AND no runtime authority exists until a service instance is activated.

#### Scenario: Plugin metadata cannot substitute for a system manifest
- GIVEN an ordinary plugin declares operation strings resembling service callbacks or fabric ports
- WHEN system-extension admission runs
- THEN admission denies because the artifact lacks a system-extension manifest and profile.

### Requirement: Service lifecycle is explicit and generation-fenced
r[molten.system_extension.lifecycle] Aspen MUST model installation, admission, initialization, start, running, checkpoint, recovery, drain, failure, restart, upgrade, rollback, shutdown, and removal as explicit legal transitions. Every callback, timer, stream, effect, checkpoint, and recovery result MUST bind the active service generation. Stale, future, inactive, drained, or removed generations MUST deny mutation and externally visible output.

#### Scenario: Legal lifecycle reaches running state
- GIVEN an admitted service instance initializes and starts successfully
- WHEN the lifecycle core applies those results in order
- THEN the instance enters running state with a canonical generation and activation ref.

#### Scenario: Stale generation callback is rejected
- GIVEN an instance has been replaced by a newer generation
- WHEN a delayed callback result from the old generation arrives
- THEN the result is discarded or denied according to the callback contract
- AND it cannot emit effects or outputs for the active generation.

### Requirement: The host invokes executable service callbacks
r[molten.system_extension.callbacks] Aspen MUST execute admitted initialize, start, request, message, stream-open, stream-event, timer, health, checkpoint, recover, drain, and shutdown callbacks when declared by the extension. A modeled lifecycle or hostcall receipt without actual callback invocation MUST NOT satisfy executable system-extension conformance.

#### Scenario: Request callback executes
- GIVEN a running service declares the request callback and has matching authority and resources
- WHEN a canonical request event is delivered
- THEN the host invokes extension code, validates its returned transition and effects, and returns or schedules the canonical response.

#### Scenario: Unsupported callback denies deterministically
- GIVEN an event targets a callback not declared by the active manifest
- WHEN dispatch runs
- THEN dispatch denies before invoking extension code
- AND emits a bounded diagnostic naming the callback and service generation.

### Requirement: Callback effects use admitted typed ports
r[molten.system_extension.typed_effects] Aspen MUST route extension effects through typed, versioned fabric-port bindings admitted for the active generation. Extension code MUST NOT gain ambient filesystem, network, clock, process, environment, storage, membership, placement, consistency, or secret access from process context, artifact installation, or callback identity.

#### Scenario: Declared effect reaches its port
- GIVEN a callback returns a valid transport effect matching an admitted binding and resource envelope
- WHEN the host validates the effect
- THEN the host forwards it to that bound transport port and correlates its completion event.

#### Scenario: Ambient or undeclared effect is denied
- GIVEN a callback requests direct socket access or an undeclared durable-store operation
- WHEN effect validation runs
- THEN validation denies before I/O
- AND identifies the missing or prohibited port authority.

### Requirement: Service execution is bounded and backpressured
r[molten.system_extension.backpressure] Aspen MUST enforce admitted bounds for callback concurrency, queued events, in-flight bytes, open streams, scheduled timers, effect requests, deadlines, cancellation, and shutdown grace. Overload behavior MUST be explicit, deterministic where required, observable, and unable to create unbounded hidden extension queues.

#### Scenario: Work remains inside bounds
- GIVEN a running service is below its admitted limits
- WHEN an event arrives
- THEN the host schedules it and accounts for its work and bytes against the active generation.

#### Scenario: Queue limit is reached
- GIVEN the service queue is at its admitted limit
- WHEN additional work arrives
- THEN the host applies the declared reject, delay, or upstream-backpressure behavior
- AND does not allocate an unbounded queue.

### Requirement: System extensions are node-supervised
r[molten.system_extension.supervision] Aspen MUST integrate each active service instance with node supervision, health reporting, restart budgets, failure classification, cancellation propagation, graceful drain, shutdown, and cleanup. Restart MUST preserve the admitted generation unless policy requires a new generation; upgrade and rollback MUST create explicit generation transitions.

#### Scenario: Recoverable failure restarts within budget
- GIVEN a service callback fails with a restartable class and restart budget remains
- WHEN supervision handles the failure
- THEN it cancels in-flight work, runs required cleanup or recovery, and starts a new attempt bound to the admitted generation.

#### Scenario: Restart budget is exhausted
- GIVEN repeated failures exhaust the admitted restart budget
- WHEN the next failure occurs
- THEN the instance enters a failed or quarantined state
- AND no further callbacks run until explicit policy or operator action admits recovery.

### Requirement: Execution profiles preserve one typed contract
r[molten.system_extension.execution_profiles] Aspen MAY provide native-process, in-process-native, or sandboxed execution profiles, but every profile MUST preserve the same canonical callback, typed-effect, generation, resource, lifecycle, and evidence contracts. More privileged profiles MUST require separate explicit admission and MUST NOT become fallback paths for unsupported sandboxed behavior.

#### Scenario: Two profiles run one conformance fixture
- GIVEN an extension fixture supports two admitted execution profiles
- WHEN callback conformance runs against each profile
- THEN canonical observable transitions and typed effects match apart from declared profile metadata.

#### Scenario: Sandbox incompatibility does not escalate privilege
- GIVEN a sandboxed extension requests an unsupported operation
- WHEN the host cannot satisfy it in that profile
- THEN activation or dispatch denies
- AND the host does not silently rerun it in a native profile.

### Requirement: Lifecycle evidence is canonical and bounded
r[molten.system_extension.evidence] Aspen MUST emit canonical evidence for system-extension admission, activation, generation changes, checkpoint and recovery boundaries, failure classification, drain, shutdown, and cleanup. Per-event or per-effect receipts MUST be optional and bounded by an explicit evidence profile rather than required on the default service hot path.

#### Scenario: Recovery evidence binds state
- GIVEN an instance recovers from an admitted checkpoint
- WHEN recovery completes
- THEN evidence binds the extension identity, service generation, checkpoint ref, port-binding ref, recovery outcome, and non-claims.

#### Scenario: Callback success is not distributed correctness proof
- GIVEN callbacks execute and lifecycle evidence validates
- WHEN evidence is exported
- THEN it does not claim consensus, durable persistence, protocol compatibility, or extension semantic correctness without separate evidence.

### Requirement: Operators can inspect active system extensions
r[molten.system_extension.operator_readback] Aspen MUST expose bounded operator readback for extension and service identity, active generation, execution profile, lifecycle state, bound port refs, resource envelope and current usage, health, restart state, checkpoint ref, and latest lifecycle evidence ref without exposing secrets or unbounded payloads.

#### Scenario: Active service is inspectable
- GIVEN a service is running
- WHEN an authorized operator requests status
- THEN readback reports its canonical active state and refs.

#### Scenario: Secret-bearing state stays redacted
- GIVEN a callback or port binding contains secret material
- WHEN status is rendered
- THEN only approved redacted refs or metadata are shown.

### Requirement: System-extension validation covers positive and negative execution
r[molten.system_extension.final_validation] Aspen MUST include positive executable callbacks and negative tests for malformed manifests, illegal transitions, unsupported callbacks, stale generations, unauthorized effects, overload, deadline expiry, cancellation, crashes, restart exhaustion, failed checkpoints, failed recovery, drain races, cleanup failures, and execution-profile escalation.

#### Scenario: Executable service fixture passes
- GIVEN a conforming extension fixture implements declared callbacks and effects
- WHEN the host runs its lifecycle and request flow
- THEN actual callback invocation, effects, responses, lifecycle state, and evidence validate.

#### Scenario: Receipt-only fixture is insufficient
- GIVEN a fixture fabricates structurally valid callback receipts without executing extension code
- WHEN executable-host conformance runs
- THEN validation fails with a missing invocation or execution-binding diagnostic.
