# Explicit Effect Handle Scoping Specification Delta

## Purpose

Bind Molten effect admission and replay to exact Kamacite semantic operation identities so behavior drift cannot match by name or shape.

## Requirements

### Requirement: Effect surfaces bind semantic operation identities

r[molten.effects.semantic_operation_identity] Effect manifests, handler bindings, effect handles, request and response envelopes, effect logs, adapter imports, remote-execution requests, and runtime receipts MUST bind exact supported Kamacite semantic operation identities in addition to Molten-owned capability, authority, policy, resource, and lifecycle refs.

#### Scenario: Exact semantic operation is requested
- GIVEN an admitted artifact requests an operation identity listed by its manifest, handle, and handler binding
- WHEN existing capability, authority, policy, resource, scope, and lifecycle gates also pass
- THEN Molten MAY dispatch the exact operation and MUST record that identity in runtime evidence.

#### Scenario: Same-named operation has a different behavior key
- GIVEN a handler supports an older same-named or same-shaped operation identity
- WHEN an artifact requests the new identity
- THEN Molten MUST deny before handler invocation with semantic-operation-mismatch diagnostics.

### Requirement: Handler matching is exact by default

r[molten.effects.semantic_handler_matching] Molten MUST require exact semantic operation identity equality for normal handler matching and MUST NOT infer compatibility from display names, local operation strings, schema shape, shared handler code, artifact presence, or successful prior execution.

#### Scenario: Local operation name matches but canonical identity is absent
- GIVEN a runtime-local handler is registered under the same display name as the requested operation but has no exact Kamacite identity
- WHEN admission runs
- THEN the handler MUST remain unavailable for normative execution.

### Requirement: Compatibility substitution is explicit

r[molten.effects.semantic_compatibility] Substitution between different semantic operation identities MUST require a current directional Kamacite compatibility artifact plus Molten-owned admission that binds the exact use context, handler profile, policy, capability, authority, resource, lifecycle, and evidence refs.

#### Scenario: Replay-only compatibility is supplied for replay
- GIVEN a compatibility artifact permits old-to-new substitution only for recorded replay and Molten admits that exact context
- WHEN replay runs
- THEN substitution MAY proceed and the replay receipt MUST bind the compatibility decision.

#### Scenario: Replay compatibility is used for live execution
- GIVEN the same artifact is presented to a live host-backed handler
- WHEN execution admission runs
- THEN Molten MUST deny because compatibility does not cover that context or grant host authority.

### Requirement: Replay and cache identities bind operation keys

r[molten.effects.semantic_replay_cache_binding] Replay identities, effect logs, transcripts, evaluation-cache keys, job evidence, remote-execution evidence, and upgrade checks MUST include the exact semantic operation identities that can affect results and MUST deny silent reuse after operation-key drift.

#### Scenario: Default behavior identity changes
- GIVEN an operation's name and schemas remain the same but its Kamacite semantic identity changes because default behavior changed
- WHEN a prior cache or effect log is considered
- THEN reuse MUST miss or deny unless exact directional compatibility evidence is admitted for that context.

### Requirement: Semantic identity remains separate from authority

r[molten.effects.semantic_identity_non_authority] Possession or equality of a semantic operation identity, handler descriptor, or compatibility artifact MUST NOT grant effect handles, host capabilities, authority, policy approval, resource rights, transport, provenance, source-gate trust, or execution permission.

#### Scenario: Exact operation identity lacks an effect handle
- GIVEN the artifact and handler agree on the semantic operation identity but the request lacks a current scope-matching effect handle
- WHEN admission runs
- THEN Molten MUST deny before side effects under the existing handle boundary.
