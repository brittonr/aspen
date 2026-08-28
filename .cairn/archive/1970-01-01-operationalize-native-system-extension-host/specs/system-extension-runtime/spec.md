# System Extension Runtime Specification Delta

## ADDED Requirements

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
