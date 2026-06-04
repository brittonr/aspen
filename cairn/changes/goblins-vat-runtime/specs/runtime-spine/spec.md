## ADDED Requirements

### Requirement: Goblins/OCapN reference boundary
r[molten.runtime_spine.goblins_reference_boundary] The system MUST treat Spritely Goblins and OCapN/CapTP as non-normative design references for vat/object execution, object capabilities, promises, persistence, debugging, and distributed references, and MUST NOT claim Guile Goblins, Racket Goblins, OCapN, or CapTP compatibility in the first implementation.

#### Scenario: Documentation cites Goblins without compatibility claim
r[molten.runtime_spine.goblins_reference_boundary.no_compat]
- GIVEN Molten design material that cites Goblins or OCapN/CapTP
- WHEN the material describes an adopted runtime pattern
- THEN it states the Molten-specific Preserves envelope, policy, evidence, transport, storage, and execution boundaries rather than claiming implementation or wire compatibility

### Requirement: Vat model with near and far references
r[molten.runtime_spine.vat_model] The runtime MUST define vats as optional internal object territories hosted by SAM-style actors or services, and MUST distinguish near references, which may be called synchronously within the same vat turn, from far references, which cross vat, actor, process, machine, transport, persistence, or sandbox boundaries and must be called asynchronously.

#### Scenario: Near object call is synchronous inside a turn
r[molten.runtime_spine.vat_model.near_call]
- GIVEN two objects hosted by the same vat during one actor turn
- WHEN one object calls the other through a near reference
- THEN the call executes synchronously within the same transactional turn

#### Scenario: Far object call returns promise
r[molten.runtime_spine.vat_model.far_call]
- GIVEN an object reference to another vat or remote peer
- WHEN an actor invokes that reference
- THEN the runtime treats the call as asynchronous and returns a promise or vow rather than blocking for a synchronous result

### Requirement: Transactional actormap
r[molten.runtime_spine.transactional_actormap] Each vat MUST maintain a transactional actormap for object behavior/state so object state changes, object spawn/remove operations, and pending outbound actions commit only if the enclosing turn succeeds and admission passes.

#### Scenario: Actormap delta commits on successful turn
r[molten.runtime_spine.transactional_actormap.commit]
- GIVEN a turn that updates an object state, spawns a new object, and queues an outbound message
- WHEN the turn completes and admission passes
- THEN the actormap delta and queued outbound message become visible as committed runtime state

#### Scenario: Actormap delta rolls back on failed turn
r[molten.runtime_spine.transactional_actormap.rollback]
- GIVEN a turn that updates local object state and then raises an uncaught error
- WHEN the turn aborts
- THEN the object state and queued outbound actions remain as they were before the turn began

### Requirement: Object references are capabilities
r[molten.runtime_spine.object_capability_refs] Object references MUST be treated as capability-bearing authority, and authority transfer MUST occur by explicit reference passing, object creation, resolver output, admitted snapshot restore, or other policy-admitted endowment.

#### Scenario: Missing reference denies use
r[molten.runtime_spine.object_capability_refs.missing_ref]
- GIVEN an object that has not been given a reference to a protected object or service
- WHEN it attempts to use that protected object or service
- THEN the runtime provides no ambient path to that authority

#### Scenario: Reference crossing boundary has Preserves descriptor
r[molten.runtime_spine.object_capability_refs.preserves_descriptor]
- GIVEN an object reference passed through a dataspace assertion, message, protocol payload, or transport envelope
- WHEN the reference crosses the runtime boundary
- THEN the reference is represented by a canonical Preserves descriptor with scope, attenuation, and evidence sufficient for admission

### Requirement: No ambient object authority
r[molten.runtime_spine.no_ambient_object_authority] Newly created objects MUST start without ambient filesystem, network, clock, process, dataspace, store, blob, consensus, choreography, or host-resource authority unless those authorities are explicitly endowed by capability-bearing references and admitted policy.

#### Scenario: New object cannot access clock without capability
r[molten.runtime_spine.no_ambient_object_authority.clock]
- GIVEN a newly spawned object without a clock capability
- WHEN it attempts to observe wall-clock time
- THEN the runtime denies the operation or requires an explicit admitted clock reference

### Requirement: Promise and vow results for far calls
r[molten.runtime_spine.promise_vows] Far-object calls MUST return promise or vow results that represent pending success, failure, cancellation, timeout, or causal failure propagation without blocking the caller's current turn.

#### Scenario: Far call resolves successfully
r[molten.runtime_spine.promise_vows.resolve]
- GIVEN a far-object call that completes successfully on the target vat
- WHEN the result is delivered to the caller
- THEN the corresponding promise resolves with the canonical result value or reference descriptor

#### Scenario: Far call failure propagates to promise
r[molten.runtime_spine.promise_vows.failure]
- GIVEN a far-object call whose target turn aborts or whose transport/session fails
- WHEN the caller observes the promise
- THEN the promise is broken with causal failure information rather than silently succeeding

### Requirement: Bounded promise pipelining
r[molten.runtime_spine.promise_pipelining] The runtime MUST support bounded promise pipelining, allowing calls to be queued against unresolved future references while enforcing queue length, lifetime, payload size, authority scope, and policy visibility limits.

#### Scenario: Pipelined call forwards after promise resolves
r[molten.runtime_spine.promise_pipelining.forward]
- GIVEN a promise that is expected to resolve to an object reference
- WHEN an actor queues a pipelined call against that promise and the promise resolves successfully
- THEN the queued call is forwarded in order to the resolved reference subject to policy admission

#### Scenario: Broken promise breaks pipelined calls
r[molten.runtime_spine.promise_pipelining.break]
- GIVEN pipelined calls queued against a promise
- WHEN the promise breaks before resolving to a reference
- THEN the queued calls fail with causal failure propagation and do not perform target side effects

#### Scenario: Pipeline bound denies excess queue
r[molten.runtime_spine.promise_pipelining.bounds]
- GIVEN a promise pipeline that has reached its configured queue or lifetime bound
- WHEN another pipelined call is requested
- THEN the runtime rejects or delays the request before unbounded memory or authority growth occurs

### Requirement: Revocable and attenuated proxies
r[molten.runtime_spine.revocable_proxies] The runtime MUST support proxy references that can narrow authority, enforce policy, log use, transform payloads, or revoke access, and revocation MUST clean up dependent assertions, subscriptions, pending calls, and live references where applicable.

#### Scenario: Revocation invalidates proxy
r[molten.runtime_spine.revocable_proxies.revoke]
- GIVEN a live proxy reference used to assert a subscription and queue far calls
- WHEN the proxy is revoked
- THEN further use is denied and dependent subscriptions or pending calls are retracted, cancelled, or failed according to the proxy policy

#### Scenario: Attenuated proxy narrows authority
r[molten.runtime_spine.revocable_proxies.attenuate]
- GIVEN a proxy that allows only a subset of methods or assertion patterns
- WHEN a caller attempts a disallowed operation through the proxy
- THEN the runtime denies the operation before it reaches the underlying reference

### Requirement: Rights amplification with sealers or branded tokens
r[molten.runtime_spine.rights_amplification] The runtime MUST support a sealer/unsealer or branded-token pattern for rights amplification, allowing authorized objects to prove private relationships or recover sealed authority without relying on ambient identity checks.

#### Scenario: Authorized unsealer reveals sealed authority
r[molten.runtime_spine.rights_amplification.unseal]
- GIVEN a sealed value created by a private sealer and an object holding the corresponding unsealer
- WHEN the object unseals the value
- THEN it recovers only the sealed authority or data and can record the brand/provenance as evidence

#### Scenario: Wrong unsealer cannot amplify rights
r[molten.runtime_spine.rights_amplification.wrong_unsealer]
- GIVEN a sealed value and an unrelated unsealer
- WHEN an object attempts to unseal the value
- THEN the runtime rejects the operation and grants no additional authority

### Requirement: Distributed reference lifetimes
r[molten.runtime_spine.distributed_ref_lifetimes] Far references MUST have explicit session, handoff, bootstrap, and lifetime or garbage-tracking rules so remote resources can be released and stale references can be denied.

#### Scenario: Session-scoped reference expires on disconnect
r[molten.runtime_spine.distributed_ref_lifetimes.disconnect]
- GIVEN a far reference whose descriptor is scoped to a transport session
- WHEN the session disconnects without admitted handoff or persistence
- THEN the reference becomes invalid and dependent pending calls fail or are retracted

#### Scenario: Handoff creates new admitted scope
r[molten.runtime_spine.distributed_ref_lifetimes.handoff]
- GIVEN a far reference that must outlive its current session
- WHEN an admitted handoff or bootstrap protocol grants a replacement descriptor
- THEN the new descriptor carries its own scope, attenuation, expiry, and evidence references

### Requirement: Safe object serialization
r[molten.runtime_spine.safe_object_serialization] Vat and object serialization MUST preserve object state and authority graphs, and objects that provide self-portraits or snapshot recipes MUST be able to describe only state and authority they already hold.

#### Scenario: Snapshot preserves authority graph
r[molten.runtime_spine.safe_object_serialization.authority_graph]
- GIVEN a vat containing objects with references to each other and to external resources
- WHEN the vat is snapshotted
- THEN the snapshot records object state and reference graph descriptors without introducing new authority

#### Scenario: Malicious portrait cannot claim new authority
r[molten.runtime_spine.safe_object_serialization.no_escalation]
- GIVEN an object snapshot portrait that claims a reference the object did not hold
- WHEN the serializer validates the portrait
- THEN the claimed reference is rejected or excluded before snapshot admission

### Requirement: Object upgrade recipes
r[molten.runtime_spine.object_upgrade] Restored object snapshots MUST use explicit behavior/schema versions and admitted upgrade recipes when object representations change across Molten versions.

#### Scenario: Snapshot restore applies admitted upgrade
r[molten.runtime_spine.object_upgrade.apply]
- GIVEN a snapshot containing an older object schema version and an admitted upgrade recipe
- WHEN the vat is restored
- THEN the runtime applies the recipe deterministically and records upgrade evidence

#### Scenario: Missing upgrade rejects incompatible snapshot
r[molten.runtime_spine.object_upgrade.missing]
- GIVEN a snapshot with an unsupported object schema version and no admitted upgrade recipe
- WHEN the runtime attempts to restore it
- THEN restore is rejected before the object becomes live

### Requirement: Time-travel distributed debugging hooks
r[molten.runtime_spine.time_travel_debugging] The runtime MUST expose trace, snapshot, and replay hooks sufficient to reconstruct object state at prior turns, inspect causality, and correlate object events with dataspace, choreography, consensus, policy, and receipt events subject to debugging authority.

#### Scenario: Debugger reconstructs prior turn state
r[molten.runtime_spine.time_travel_debugging.reconstruct]
- GIVEN admitted snapshots and turn trace records for a vat
- WHEN an authorized debugger selects a prior turn id
- THEN the runtime can reconstruct or present the object state and pending causal events for that point in execution

#### Scenario: Debugging respects authority
r[molten.runtime_spine.time_travel_debugging.authority]
- GIVEN a trace containing secret object state or references
- WHEN a caller lacks the required debugging capability
- THEN the inspection surface redacts or denies access to protected state and references

### Requirement: Authority graph inspection
r[molten.runtime_spine.authority_graph_inspection] The runtime SHOULD expose an authority-aware inspection surface for object reference graphs, proxy chains, attenuations, revocations, and snapshot descriptors.

#### Scenario: Operator inspects attenuated reference graph
r[molten.runtime_spine.authority_graph_inspection.inspect]
- GIVEN an authorized operator inspecting a vat
- WHEN the operator requests the reference graph
- THEN the runtime reports objects, references, proxy boundaries, attenuations, revocation state, and evidence references subject to redaction policy

### Requirement: Portable encrypted storage
r[molten.runtime_spine.portable_encrypted_storage] Content, snapshots, large payloads, and document artifacts SHOULD use provider-independent storage principles: content addressing, encryption before storage, chunking, mutable containers built from immutable chunks, and read/write authority represented as explicit capabilities.

#### Scenario: Blob provider cannot read encrypted content
r[molten.runtime_spine.portable_encrypted_storage.encrypted]
- GIVEN a snapshot or large payload stored through a blob adapter
- WHEN the blob provider stores the chunks
- THEN the provider sees only encrypted chunks and metadata that does not include plaintext without a read capability

#### Scenario: Content ref is network independent
r[molten.runtime_spine.portable_encrypted_storage.network_independent]
- GIVEN an immutable encrypted content artifact addressed by hash
- WHEN the artifact is fetched from Iroh blobs, local store, or another admitted provider
- THEN the same content reference and integrity checks apply regardless of provider location

### Requirement: Vat integration tests
r[molten.runtime_spine.vat_integration_tests] The system MUST include integration tests for near synchronous calls, far asynchronous calls, actormap rollback, pending action commit, reference passing, proxy revocation, and promise failure propagation.

#### Scenario: Far call and revocation integration test
r[molten.runtime_spine.vat_integration_tests.far_revoke]
- GIVEN two vats connected through a local far-reference adapter
- WHEN one vat calls through a proxy and the proxy is later revoked
- THEN the first call follows normal promise resolution rules and later calls fail due to revocation

### Requirement: Snapshot integration tests
r[molten.runtime_spine.snapshot_integration_tests] The system MUST include integration tests for object snapshot/restore, authority preservation, denied authority escalation, and version upgrade recipes.

#### Scenario: Restored vat preserves allowed references only
r[molten.runtime_spine.snapshot_integration_tests.restore]
- GIVEN a vat snapshot with an object reference graph and an attempted unauthorized extra reference
- WHEN the snapshot is restored
- THEN admitted references are restored and unauthorized authority is denied or excluded

### Requirement: Promise pipeline property tests
r[molten.runtime_spine.promise_property_tests] The system MUST use Hegel property-based tests for bounded promise pipelines, resolution and failure ordering, queue cleanup, and causal failure propagation within supported bounds.

#### Scenario: Generated pipeline preserves order or fails causally
r[molten.runtime_spine.promise_property_tests.pipeline_order]
- GIVEN a generated bounded promise pipeline and generated resolution or failure event
- WHEN the model processes the pipeline
- THEN forwarded calls preserve queue order on success and all queued calls fail causally on promise break

### Requirement: Actormap property tests
r[molten.runtime_spine.actormap_property_tests] The system MUST use Hegel property-based tests for generated actormap turn deltas to verify commit and rollback invariants.

#### Scenario: Generated failed turn preserves prior actormap
r[molten.runtime_spine.actormap_property_tests.rollback]
- GIVEN a generated actormap state and generated turn delta that aborts
- WHEN the model rolls the turn back
- THEN the resulting actormap equals the prior committed state
