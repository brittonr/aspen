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

### Requirement: Native-process extensions use the canonical host contract

r[molten.system_extension.native_host.profile] Molten MUST provide a native-process execution profile that preserves the accepted manifest, callback, typed-effect, generation, resource, lifecycle, state, and evidence contracts.

#### Scenario: Native profile is selected

- GIVEN an admitted manifest selects the exact native-process profile and executable cohort
- WHEN the system-extension host creates the instance
- THEN it MUST select the native executor without changing callback or lifecycle meaning.

#### Scenario: Native execution is unavailable

- GIVEN the native executor or bounded execution port is unavailable
- WHEN instance activation runs
- THEN activation MUST fail before callback invocation
- AND it MUST NOT fall back to in-process or sandboxed execution.

### Requirement: Callback framing is canonical and bounded

r[molten.system_extension.native_host.callback_protocol] Every native callback invocation MUST use a versioned canonical envelope and outcome with explicit byte, field, collection, diagnostic, and deadline bounds.

#### Scenario: Callback bytes are valid

- GIVEN the envelope binds manifest, instance, service, generation, callback, sequence, deadline, input, state, policy, resources, and port refs
- WHEN the host encodes and invokes the callback
- THEN the child MUST receive the exact canonical bytes
- AND the host MUST admit the returned canonical outcome before effects become visible.

#### Scenario: Child returns malformed or extra output

- GIVEN standard output is malformed, non-canonical, oversized, trailing, or uses an unsupported schema
- WHEN callback result admission runs
- THEN the callback MUST fail
- AND no returned state, status, checkpoint, or effect request may commit.

### Requirement: Executable admission is exact

r[molten.system_extension.native_host.executable] Installation MUST bind exact executable bytes, artifact kind, target, dependency closure, materialization, provenance, source gate, policy, authority, resources, and execution profile.

#### Scenario: Complete executable evidence passes

- GIVEN every supplied ref matches the inspected executable and selected native profile
- WHEN install admission runs
- THEN it MUST emit an admitted instance plan bound to the executable content identity.

#### Scenario: Path or artifact possession is the only evidence

- GIVEN an operator supplies a host path or executable bytes without complete admission evidence
- WHEN install admission runs
- THEN it MUST deny before process execution or durable instance publication.

### Requirement: Native callbacks run through bounded execution

r[molten.system_extension.native_host.execution] The native executor MUST invoke callbacks only through the accepted bounded execution fabric port with cleared environment, explicit input, named limits, and owned teardown.

#### Scenario: Callback process completes

- GIVEN an admitted callback request stays within every execution bound
- WHEN the bounded process exits and returns a valid outcome
- THEN the host MUST preserve the process receipt and admitted callback outcome.

#### Scenario: Callback times out or floods output

- GIVEN callback execution exceeds its deadline or output limit
- WHEN the execution port enforces the bound
- THEN the host MUST preserve timeout or truncation observations
- AND it MUST NOT silently expand the limit or infer callback success.

### Requirement: Extension instances have durable service state

r[molten.system_extension.native_host.durability] The host MUST durably record manifest, executable, active generation, lifecycle state, callback sequence, checkpoint, unresolved callbacks, unresolved effects, resource use, and evidence refs.

#### Scenario: Instance state commits

- GIVEN install or lifecycle transition admission passes
- WHEN the node publishes the new instance state
- THEN a restart MUST load the same canonical service generation and lifecycle facts.

#### Scenario: Durable state is missing or incompatible

- GIVEN required instance, checkpoint, schema, generation, or unresolved-operation state is absent or incompatible
- WHEN startup recovery runs
- THEN the instance MUST fail closed or enter quarantine
- AND no normal request callback may run.

### Requirement: Callback and effect intent precede external effects

r[molten.system_extension.native_host.intent] The host MUST persist callback intent before process start and approved effect intent before fabric-port routing.

#### Scenario: Callback start fails before process spawn

- GIVEN callback intent committed but executable setup fails before spawn
- WHEN the host records the observation
- THEN recovery MUST classify the callback as definite pre-start failure.

#### Scenario: Effect routing loses acknowledgement

- GIVEN effect intent committed and routing may have reached the adapter
- WHEN the host loses the definitive effect result
- THEN the effect MUST remain unresolved or unknown
- AND the host MUST NOT retry it automatically.

### Requirement: Effect routing uses the active manifest snapshot

r[molten.system_extension.native_host.effects] The host MUST validate callback output and route approved effects only through exact ports bound to the active manifest generation.

#### Scenario: Effect matches one active binding

- GIVEN the callback requests an allowed operation with complete authority and resources
- WHEN effect routing runs
- THEN the host MUST route it through the exact selected port
- AND the completion MUST retain callback, effect, operation, port, and generation linkage.

#### Scenario: Callback requests an unbound or stale effect

- GIVEN the callback requests an absent, incompatible, disabled, over-authorizing, or stale-generation port
- WHEN effect admission runs
- THEN the host MUST deny before external adapter activity.

### Requirement: Effect completion re-enters extension semantics

r[molten.system_extension.native_host.effect_completion] Effect completion MUST enter as a generation-fenced callback event and MUST NOT directly define service success or mutate extension semantic state.

#### Scenario: Linked completion is delivered

- GIVEN a terminal effect observation matches the active pending effect and generation
- WHEN completion processing runs
- THEN the host MAY invoke the extension with the canonical completion event
- AND the extension MUST decide its semantic transition.

#### Scenario: Completion is stale or duplicated

- GIVEN a completion belongs to another generation, operation, effect, or already consumed sequence
- WHEN completion admission runs
- THEN it MUST deny without invoking the extension.

### Requirement: Service ingress is versioned and acknowledged

r[molten.system_extension.native_host.ingress] Installed services MUST expose requests only through a versioned transport profile with exact endpoint, ALPN, framing, peer, service, manifest, generation, authority, policy, resource, and acknowledgement bindings.

#### Scenario: Ingress request reaches the service

- GIVEN transport, peer, capability, authority, policy, resource, and service-generation admission passes
- WHEN a client submits a bounded canonical request
- THEN the host MUST durably classify acceptance before returning service acknowledgement.

#### Scenario: Transport accepts but callback admission fails

- GIVEN transport delivered a request but callback admission denies it
- WHEN the client observes the result
- THEN the service MUST return a typed denial
- AND transport acceptance MUST NOT be reported as service acceptance.

### Requirement: Recovery classifies unresolved operations

r[molten.system_extension.native_host.recovery] Startup MUST classify unresolved callbacks and effects as not-started, running-observed, terminal, unknown, or stale before invoking recovery logic.

#### Scenario: Committed callback intent has no start evidence

- GIVEN callback intent exists without child-start evidence
- WHEN recovery inventory runs
- THEN it MUST classify the callback as not-started
- AND policy MAY decide whether a new callback attempt is allowed.

#### Scenario: Effect outcome is unknown

- GIVEN an effect may have executed without a definitive terminal observation
- WHEN recovery runs
- THEN the host MUST preserve unknown state and exact operation identity
- AND it MUST require extension-owned reconciliation before retry.

### Requirement: Lifecycle operations are operator-visible

r[molten.system_extension.native_host.operator] Molten MUST expose bounded install, start, request, status, recover, drain, stop, and remove operations with canonical receipts and offline readback.

#### Scenario: Operator drains and stops an instance

- GIVEN an active instance has no unclassified callbacks or effects and all drain guards pass
- WHEN the operator drains and stops it
- THEN the host MUST stop new ingress, complete bounded teardown, and record terminal lifecycle evidence.

#### Scenario: Removal has unresolved work

- GIVEN an instance has unresolved callbacks, unresolved effects, active sessions, stale cleanup, or missing terminal evidence
- WHEN removal admission runs
- THEN removal MUST deny with the blocking identities.

### Requirement: Workload-neutral composition is enforced

r[molten.system_extension.native_host.neutrality] Node composition MUST select native extension implementations by admitted contract and artifact identity without workload-specific core branches.

#### Scenario: A Kiln extension is installed

- GIVEN a separately published Kiln extension satisfies the generic native host contract
- WHEN composition admits its exact manifest and executable
- THEN the generic host MAY run it without a Kiln branch in Molten core.

#### Scenario: Node core switches on a workload name

- GIVEN node-core code selects callback, effect, durability, or recovery behavior from a Kiln or other workload label
- WHEN architecture validation runs
- THEN validation MUST fail with the semantic leakage site.

### Requirement: Native host conformance covers failure paths

r[molten.system_extension.native_host.validation] Molten MUST test pure lifecycle, executable admission, callback framing, bounded execution, durability, intent ordering, effects, ingress, recovery, operator workflows, and dependency direction.

#### Scenario: Separate-process service fixture passes

- GIVEN an exact local pilot cohort and a conforming external callback executable
- WHEN install, activate, request, effect, checkpoint, restart, recover, drain, and stop run
- THEN the parent and child evidence, durable state, lifecycle, and offline verification MUST pass.

#### Scenario: Required negative coverage is absent

- GIVEN no test covers malformed output, timeout, cancellation, stale generation, unknown effect, incompatible checkpoint, or removal denial
- WHEN change closeout runs
- THEN the change MUST remain incomplete.

### Requirement: Native host evidence preserves non-claims

r[molten.system_extension.native_host.nonclaims] Native host receipts MUST NOT claim sandboxing, hermeticity, executable trust, callback correctness, effect success, transport delivery, distributed availability, or production readiness.

#### Scenario: Local pilot service succeeds

- GIVEN the separate-process service fixture and restart recovery pass
- WHEN operator status reports readiness
- THEN it MUST label the result as local live pilot evidence
- AND it MUST retain native execution, transport, durability, workload, and release non-claims.

### Requirement: Native callbacks receive exact materialized values

r[molten.system_extension.native_host.value_materialization] The native host MUST supply bounded, identity-checked bytes for every callback payload, prior state, effect completion, and recovery checkpoint required by the selected callback.

#### Scenario: Materialized callback values match their references

- GIVEN the value port returns bytes for every required callback reference
- WHEN callback framing runs
- THEN the host MUST verify each BLAKE3 identity and byte bound
- AND the child MUST receive the verified bytes in the canonical v2 envelope.

#### Scenario: A required value is missing or corrupt

- GIVEN a payload, state, completion, or checkpoint reference has no exact bounded bytes
- WHEN callback preparation runs
- THEN the callback MUST fail before process start
- AND the host MUST NOT use a reference-only fallback.

### Requirement: Native callback results publish exact values

r[molten.system_extension.native_host.value_publication] The native host MUST admit and publish bounded output, effect-request, next-state, and checkpoint bytes before their references become visible to host or provider semantics.

#### Scenario: Returned values publish successfully

- GIVEN the child returns canonical reference-and-byte values
- WHEN callback result admission runs
- THEN every reference MUST match its bytes
- AND publication MUST complete before state replacement or provider routing.

#### Scenario: Returned bytes are absent or substituted

- GIVEN the child returns a reference without bytes or bytes with another identity
- WHEN result admission runs
- THEN the callback MUST fail closed
- AND no state, checkpoint, output, or provider effect may become visible.

### Requirement: Value effects have durable intent and uncertainty

r[molten.system_extension.native_host.value_intent] The native host MUST persist callback intent before materialization and publication intent before value publication.

#### Scenario: Publication fails before acceptance

- GIVEN publication intent committed and the value port rejects before accepting bytes
- WHEN the host records the result
- THEN the publication operation MUST become terminal
- AND semantic state MUST remain unchanged.

#### Scenario: Publication acceptance is uncertain

- GIVEN publication may have accepted bytes but no definitive result is available
- WHEN recovery inventory runs
- THEN the publication operation MUST remain unknown
- AND the host MUST NOT republish or route dependent effects automatically.

### Requirement: Semantic state survives restart as materialized content

r[molten.system_extension.native_host.semantic_state] The durable native instance MUST track latest semantic state separately from lifecycle checkpoint state and MUST recover both by exact content identity.

#### Scenario: Request updates semantic state

- GIVEN a callback publishes valid next-state bytes
- WHEN callback completion commits
- THEN the instance MUST store the new state reference
- AND the next callback MUST receive those exact bytes.

#### Scenario: Restart observes unresolved value work

- GIVEN a restart finds unresolved materialization or publication operations
- WHEN native recovery classification runs
- THEN it MUST preserve their exact identity and uncertainty
- AND normal ingress MUST remain blocked until explicit reconciliation.

### Requirement: Native protocol v2 is exact and non-fallback

r[molten.system_extension.native_host.value_protocol] A materializing native host profile MUST use only the exact v2 envelope, outcome, ALPN, and framing cohort.

#### Scenario: Version two is selected

- GIVEN the executable and host profile select the same v2 cohort
- WHEN install and callback admission run
- THEN the host MUST use the v2 value protocol for every callback.

#### Scenario: Legacy or mixed protocol is supplied

- GIVEN any schema, ALPN, framing, executable cohort, or value requirement selects v1 or a mixed version
- WHEN admission runs
- THEN installation or callback admission MUST fail without fallback.

### Requirement: Materialization conformance includes negative paths

r[molten.system_extension.native_host.value_validation] Conformance MUST test exact bytes, process separation, restart, missing values, corrupt values, bounds, legacy framing, publication rejection, publication uncertainty, and blocked dependent effects.

#### Scenario: Separate-process materialization passes

- GIVEN a conforming external executable and exact v2 profile
- WHEN ingress, callback, state publication, effect publication, checkpoint, restart, and recovery run
- THEN parent-observed identities and durable operation ordering MUST pass.

#### Scenario: Negative evidence is absent

- GIVEN required identity, bound, legacy, rejection, uncertainty, or restart tests are absent
- WHEN closeout runs
- THEN the change MUST remain incomplete.
