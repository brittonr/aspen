## ADDED Requirements

### Requirement: Versioned envelope spine
r[molten.runtime_spine.envelope] The system MUST define a versioned envelope type with Serde DTO boundaries that carries sender identity, routable subject, Preserves body, blob references, capabilities, and evidence references.

#### Scenario: Native actors exchange an envelope
r[molten.runtime_spine.envelope.native_exchange]
- GIVEN two native Molten actors in the same runtime
- WHEN one actor sends a valid envelope to a subject observed by the other actor
- THEN the receiving actor observes the same envelope fields after routing

### Requirement: Canonical Preserves boundary
r[molten.runtime_spine.canonical_preserves] The system MUST define Blake3 boundary hashes over canonical Preserves bytes rather than over incidental Rust memory layout or debug formatting.

#### Scenario: Equivalent envelope encodings hash identically
r[molten.runtime_spine.canonical_preserves.stable_hash]
- GIVEN two equivalent envelope values constructed through different Rust code paths
- WHEN each envelope is converted to canonical Preserves bytes
- THEN both canonical byte streams produce the same boundary hash

### Requirement: Preserves communication boundary
r[molten.runtime_spine.preserves_comms] Every Molten communication surface that crosses a runtime, trust, transport, execution, storage, policy, or evidence boundary MUST have a canonical Preserves representation, even when Rust structs or adapter-native types are used internally.

#### Scenario: Actor and dataspace messages use Preserves boundary
r[molten.runtime_spine.preserves_comms.dataspace]
- GIVEN a local actor envelope or dataspace assertion/message
- WHEN the message crosses the actor or dataspace adapter boundary
- THEN the communicated value is representable as canonical Preserves bytes for hashing, routing, policy admission, and evidence

#### Scenario: Protocol and consensus messages use Preserves boundary
r[molten.runtime_spine.preserves_comms.protocol_consensus]
- GIVEN a choreography protocol-message envelope or a Raft command/message envelope
- WHEN the message is routed locally, transported remotely, persisted, or admitted by policy
- THEN the protocol or consensus message is represented at the boundary by canonical Preserves bytes with stable identity

#### Scenario: Large payload uses Preserves reference
r[molten.runtime_spine.preserves_comms.large_payload]
- GIVEN a communication payload too large or unsuitable to carry inline
- WHEN the payload is sent through Molten
- THEN the envelope carries canonical Preserves metadata and content references while the large bytes may be stored or transported through a blob adapter

### Requirement: Pure core boundary
r[molten.runtime_spine.core_purity] The core envelope and validation layer MUST be deterministic and MUST NOT perform filesystem, network, process, clock, scripting, or runtime scheduling effects.

#### Scenario: Core validation runs without adapters
r[molten.runtime_spine.core_purity.no_adapters]
- GIVEN an envelope fixture and no runtime adapters
- WHEN core validation checks the fixture
- THEN validation returns only deterministic data derived from the fixture

### Requirement: Snafu error boundary
r[molten.runtime_spine.error_boundary] The system MUST use structured error types at core validation and adapter boundaries so callers can distinguish invalid input, denied operations, unavailable adapters, and persistence failures.

#### Scenario: Adapter failure is structured
r[molten.runtime_spine.error_boundary.adapter_failure]
- GIVEN a runtime adapter that cannot complete a requested side effect
- WHEN the adapter reports the failure
- THEN the caller receives a structured error category and source context rather than an unstructured string

### Requirement: Runtime reference boundaries
r[molten.runtime_spine.runtime_references] The system MUST document BEAM/OTP and Lunatic as non-normative design references for actor lifecycle, supervision, mailboxes, links/monitors, scheduling, and Wasm hostcall ergonomics, and MUST NOT claim BEAM distribution, OTP behavior, Erlang/Elixir API, or Lunatic API compatibility.

#### Scenario: Reference material does not become compatibility claim
r[molten.runtime_spine.runtime_references.non_compatibility]
- GIVEN runtime design material that cites BEAM/OTP or Lunatic
- WHEN Molten describes a borrowed runtime pattern
- THEN the material states the Molten-specific envelope, policy, evidence, and transport boundaries instead of claiming protocol or API compatibility

### Requirement: Local dataspace adapter
r[molten.runtime_spine.local_dataspace] The system MUST provide a local runtime adapter that routes envelopes through actor, assertion, subscription, and dataspace concepts without leaking those mechanisms into the pure core.

#### Scenario: Subscription receives matching envelope
r[molten.runtime_spine.local_dataspace.subscription]
- GIVEN a local actor subscribed to a subject pattern
- WHEN another local actor sends a matching envelope
- THEN the subscribed actor receives the envelope through the runtime adapter

### Requirement: Declarative startup configuration
r[molten.runtime_spine.config] The system MUST evaluate Nickel-authored configuration into typed startup configuration before runtime dispatch begins.

#### Scenario: Config declares an actor and subscription
r[molten.runtime_spine.config.actor_subscription]
- GIVEN a Nickel configuration that declares a native actor and a subscription
- WHEN Molten loads the configuration
- THEN the runtime starts with the declared actor and subscription represented as typed Rust config

### Requirement: Clap CLI surface
r[molten.runtime_spine.cli] The system MUST expose command-line operations through a Clap-based CLI surface.

#### Scenario: CLI parses config path
r[molten.runtime_spine.cli.config_path]
- GIVEN a user-provided runtime configuration path
- WHEN Molten parses CLI arguments
- THEN the selected command receives a typed configuration path value

### Requirement: Tracing observability
r[molten.runtime_spine.observability] The system MUST emit structured tracing spans or events at runtime and adapter boundaries.

#### Scenario: Adapter decision emits trace event
r[molten.runtime_spine.observability.adapter_decision]
- GIVEN an adapter admission decision
- WHEN the runtime records the decision
- THEN a structured tracing event identifies the adapter, decision, and envelope subject

### Requirement: Iroh remote bridge
r[molten.runtime_spine.remote_bridge] The system MUST bridge envelope-sized messages over Iroh gossip, large immutable payloads over Iroh blobs, and replicated mutable document/state surfaces over Iroh docs.

#### Scenario: Remote peer receives envelope and blob reference
r[molten.runtime_spine.remote_bridge.blob_reference]
- GIVEN two Molten peers joined to the same authorized topic
- WHEN one peer publishes an envelope with a content reference
- THEN the other peer receives the envelope over gossip and can fetch the referenced payload through the blob adapter

### Requirement: Blob reference bridge
r[molten.runtime_spine.blob_refs] The system MUST provide a blob adapter for content-addressed payload references carried by runtime envelopes, while keeping large payload bytes out of the canonical envelope body.

#### Scenario: Envelope declares external blob reference
r[molten.runtime_spine.blob_refs.declared]
- GIVEN an envelope carrying a canonical content reference for an external payload
- WHEN the blob adapter stores or fetches the payload
- THEN the adapter verifies the bytes against the declared reference before the payload is admitted

### Requirement: Iroh docs bridge
r[molten.runtime_spine.docs_bridge] The system MUST expose Iroh docs through a runtime adapter that records envelope-level evidence for application-visible document mutations.

#### Scenario: Document mutation emits evidence
r[molten.runtime_spine.docs_bridge.mutation_evidence]
- GIVEN a Molten actor with an admitted document mutation capability
- WHEN the actor applies a mutation through the Iroh docs adapter
- THEN the runtime records the document namespace, mutation reference, and admission evidence in an envelope or receipt

### Requirement: Remote content admission
r[molten.runtime_spine.remote_admission] The system MUST reject remote envelopes when declared blob references or canonical envelope hashes fail validation.

#### Scenario: Tampered blob is rejected
r[molten.runtime_spine.remote_admission.tampered_blob]
- GIVEN a remote envelope that declares a content reference
- WHEN the fetched payload does not match the declared reference
- THEN the runtime rejects the payload before delivering it to actors

### Requirement: Sandboxed Wasmtime actors
r[molten.runtime_spine.wasmtime_hostcalls] The system MUST expose sandboxed Wasmtime actor hostcalls for envelope send, subscription, blob read, and blob write while denying ambient filesystem and network access.

#### Scenario: Wasm actor sends through hostcall
r[molten.runtime_spine.wasmtime_hostcalls.send]
- GIVEN a Wasmtime actor with an admitted send capability
- WHEN the actor calls the send hostcall with a valid envelope
- THEN the runtime applies admission checks and routes the envelope only if admitted

### Requirement: Deny-by-default WASI capabilities
r[molten.runtime_spine.wasi_capabilities] The system MUST use Wasmtime-WASI only through explicit capability wiring and MUST deny ambient filesystem, clock, environment, and socket access by default.

#### Scenario: WASI filesystem access is denied without capability
r[molten.runtime_spine.wasi_capabilities.filesystem_denied]
- GIVEN a Wasmtime actor without an admitted filesystem capability
- WHEN the actor attempts to access a host filesystem path through WASI
- THEN the runtime denies the access before exposing host path contents

### Requirement: WIT component admission
r[molten.runtime_spine.wit_components] The system MUST support WIT/component bindings for typed actor interfaces and wasmparser-based module inspection before actor admission.

#### Scenario: Invalid component import is rejected
r[molten.runtime_spine.wit_components.invalid_import]
- GIVEN a Wasm component declaring an import outside the admitted hostcall surface
- WHEN the runtime inspects the component before admission
- THEN the component is rejected before instantiation

### Requirement: Trusted Steel orchestration
r[molten.runtime_spine.steel_orchestration] The system MUST expose Steel orchestration APIs that operate through the same envelope spine as native, remote, and Wasmtime actors.

#### Scenario: Steel script spawns and inspects actors
r[molten.runtime_spine.steel_orchestration.spawn_inspect]
- GIVEN a trusted Steel orchestration script
- WHEN the script spawns an actor and inspects runtime state
- THEN those operations use public runtime APIs and produce inspectable envelope or receipt evidence

### Requirement: Deny-by-default adapter effects
r[molten.runtime_spine.deny_by_default] The system MUST deny adapter side effects by default unless an explicit policy gate admits the requested operation.

#### Scenario: Missing capability denies send
r[molten.runtime_spine.deny_by_default.missing_capability]
- GIVEN an actor without a matching send capability
- WHEN the actor requests a send side effect
- THEN the runtime denies the side effect before any local or remote delivery occurs

### Requirement: Nickel and Steel contract selection
r[molten.runtime_spine.nickel_steel_contracts] The system MUST use Nickel contracts for static declarative policy, schema, resource, ability, adapter-option, and configuration gates, and MUST use Steel contracts only for explicitly reviewed dynamic predicates or trusted callables that cannot be represented as static Nickel data.

#### Scenario: Static policy uses Nickel contract
r[molten.runtime_spine.nickel_steel_contracts.static_nickel]
- GIVEN a runtime action governed by static resource prefixes, allowed abilities, or adapter options
- WHEN Molten evaluates the action before side effects
- THEN the admission path uses a Nickel-authored contract artifact and records the contract id and normalized source hash in evidence

#### Scenario: Dynamic predicate uses Steel contract
r[molten.runtime_spine.nickel_steel_contracts.dynamic_steel]
- GIVEN a runtime action that requires an explicitly reviewed dynamic predicate or trusted callable
- WHEN Molten evaluates the action before side effects
- THEN the admission path uses a Steel contract backend and records the backend, contract id, decision, and receipt reference in evidence

### Requirement: Basalt contract enforcement
r[molten.runtime_spine.basalt_contracts] The system MUST support Basalt-backed UCAN contract enforcement for capability-bearing runtime requests that are governed by Nickel policy artifacts or Steel contract backends.

#### Scenario: UCAN contract admits bounded request
r[molten.runtime_spine.basalt_contracts.admit]
- GIVEN a runtime request with a Basalt contract id, resource, ability, and matching UCAN capability grant
- WHEN the policy layer evaluates the request
- THEN the operation is admitted only for the matching resource and ability

### Requirement: Policy gate integration
r[molten.runtime_spine.policy_gate] The system MUST support bounded policy gates using Trellis predicates for capabilities, replay checks, leases, routing limits, and content integrity.

#### Scenario: Policy gate records admission decision
r[molten.runtime_spine.policy_gate.receipt]
- GIVEN an envelope that requests a gated operation
- WHEN the policy layer evaluates the operation
- THEN the runtime records whether the operation was admitted or rejected and which bounded predicate was applied

### Requirement: Cairn receipt validation
r[molten.runtime_spine.cairn_receipts] The system MUST validate action-envelope and lifecycle receipts through Cairn surfaces before treating them as runtime evidence.

#### Scenario: Invalid receipt is not evidence
r[molten.runtime_spine.cairn_receipts.invalid]
- GIVEN an envelope with an attached Cairn receipt reference
- WHEN the referenced receipt fails Cairn validation
- THEN the runtime excludes that receipt from admitted evidence

### Requirement: Octet/Valence evidence references
r[molten.runtime_spine.valence_evidence] The system MUST support Octet/Valence evidence references for function object, module, and provenance claims without treating those references as proof of semantic correctness.

#### Scenario: Function object evidence is bounded
r[molten.runtime_spine.valence_evidence.boundary]
- GIVEN an envelope that references function object evidence
- WHEN the runtime displays or evaluates the evidence reference
- THEN it reports the bounded evidence claim and does not claim general semantic equivalence

### Requirement: Redb local store adapter
r[molten.runtime_spine.redb_store] The system MUST support a Redb-backed adapter for durable local metadata, receipt indexes, replay caches, and content-reference bookkeeping while keeping filesystem effects out of the pure core.

#### Scenario: Store adapter records receipt index
r[molten.runtime_spine.redb_store.receipt_index]
- GIVEN an admitted runtime operation that emits a receipt reference
- WHEN the Redb store adapter persists the local index entry
- THEN later inspection can recover the receipt reference without re-running pure admission logic

### Requirement: Integration evidence
r[molten.runtime_spine.integration_evidence] The system MUST provide end-to-end evidence that runtime configuration, local routing, remote bridge handling, and policy admission preserve envelope boundaries across adapters.

#### Scenario: Configured route emits boundary evidence
r[molten.runtime_spine.integration_evidence.config_route]
- GIVEN a runtime configuration that starts a local actor, remote bridge, and policy gate
- WHEN an admitted envelope traverses those adapters
- THEN the emitted evidence links the configuration, local route, remote bridge, and policy decision without granting extra authority

### Requirement: Hegel property tests
r[molten.runtime_spine.property_tests] The system MUST use Hegel property-based tests for envelope, admission, and adapter invariants that are too broad for hand-written examples alone.

#### Scenario: Generated envelopes preserve canonical identity
r[molten.runtime_spine.property_tests.generated_envelopes]
- GIVEN a generated valid envelope
- WHEN the property test converts it through the supported DTO and canonical encoding boundaries
- THEN the envelope identity and canonical hash remain stable
