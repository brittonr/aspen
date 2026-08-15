## ADDED Requirements

### Requirement: Peer handoff bundles are canonical
r[molten.peer_handoff.bundle_model] Molten MUST define a canonical peer handoff bundle that binds ticket, peer admission or session evidence, expected peer/node/topic/scope, accepted capabilities, policy refs, resource refs, optional authority grants, freshness, revocation state, and supporting receipt refs.

#### Scenario: Bundle binds peer and scope
- GIVEN an operator exports a peer handoff for a node-control topic or remote workload scope
- WHEN the bundle is serialized
- THEN the bundle records the expected peer id, receiver node id, topic or scope, member refs, freshness, and supporting receipt refs
- AND the bundle ref is derived from canonical Preserves bytes.

### Requirement: Peer handoff verify and gate fail closed
r[molten.peer_handoff.verify_gate] Molten MUST verify and gate peer handoff bundles before import by checking member refs, embedded member integrity, expected bindings, freshness, duplicate members, malformed members, and wrong-scope evidence.

#### Scenario: Wrong peer binding denies gate
- GIVEN a peer handoff bundle names one peer id but contains a peer admission for another peer
- WHEN the handoff gate validates the bundle
- THEN the gate decision is deny
- AND no bundle member is imported into the target state root.

### Requirement: Peer handoff bundles are not authority
r[molten.peer_handoff.authority_boundary] Molten MUST NOT treat a peer handoff bundle, verify receipt, gate receipt, or import receipt as operation authority, provenance, source-gate, resource, retention, execution, or transport trust.

#### Scenario: Handoff without authority cannot run operation
- GIVEN a peer handoff bundle contains a valid ticket and peer admission but no matching authority grant for a node-control operation
- WHEN the sender applies the bundle for that operation
- THEN apply denies or dry-runs with an authority-missing diagnostic
- AND no live operation is sent unless an explicit matching authority grant is present.

### Requirement: Handoff import and apply are separated
r[molten.peer_handoff.import_apply] Molten SHOULD separate handoff import from handoff apply so operators can store verified members without triggering live sends, remote execution, destructive cleanup, or other side effects.

#### Scenario: Import stores evidence without sending
- GIVEN a verified peer handoff bundle with ticket and admission evidence
- WHEN the operator imports the bundle into a sender state root
- THEN import stores the permitted evidence members and emits an import receipt
- AND no network send or subsystem operation is performed by import alone.

### Requirement: Handoff diagnostics are actionable
r[molten.peer_handoff.diagnostics] Molten SHOULD diagnose missing handoff members, stale tickets, wrong endpoint/topic/scope bindings, missing peer admission, missing authority grant, and transport-only evidence with next-step guidance.

#### Scenario: Stale ticket diagnostic names refresh path
- GIVEN a handoff bundle contains an expired or stale live ticket
- WHEN the handoff gate validates it
- THEN the diagnostic names the stale ticket condition
- AND recommends refreshing the bound live ticket before apply.

### Requirement: Peer handoff tests cover boundaries
r[molten.peer_handoff.positive_negative_tests] Molten SHOULD include positive handoff verify/import/apply tests and negative tests for malformed members, wrong scope, missing admission, stale ticket, duplicate member, transport-only evidence, and authority-bound operation denial.

#### Scenario: Duplicate member fixture denies
- GIVEN a handoff bundle repeats a ticket or peer admission member with conflicting refs
- WHEN the verifier evaluates the bundle
- THEN it emits a deny decision
- AND the diagnostics identify duplicate or conflicting members.
