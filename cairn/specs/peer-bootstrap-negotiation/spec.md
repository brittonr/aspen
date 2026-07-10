# Peer Bootstrap Negotiation Specification

## Purpose

Defines the `peer-bootstrap-negotiation` capability.

## Requirements

### Requirement: System MUST Define canonical bootstrap inputs for static peers, invites, endpoint ids, local discovery, catalog records, gatekeeper credentials, and control-plane membership
r[molten.peer_bootstrap.bootstrap_inputs] The system MUST Define canonical bootstrap inputs for static peers, invites, endpoint ids, local discovery, catalog records, gatekeeper credentials, and control-plane membership.

### Requirement: System MUST Define handshake records with node ids, versions, artifact/schema/effect/transport support, resources, replay support, requested groups, capabilities, and policy refs
r[molten.peer_bootstrap.handshake_record] The system MUST Define handshake records with node ids, versions, artifact/schema/effect/transport support, resources, replay support, requested groups, capabilities, and policy refs.

### Requirement: System MUST Document that Iroh transport identity alone is not Molten authority
r[molten.peer_bootstrap.no_transport_authority] The system MUST Document that Iroh transport identity alone is not Molten authority.

### Requirement: Iroh endpoint identity is transport binding, not authority
r[molten.peer_bootstrap.iroh_identity_transport_boundary] Peer bootstrap handshakes that include Iroh endpoint public identity refs MUST treat those refs as transport binding and diagnostics only; operation authority still requires explicit peer admission, capability, authority, policy, resource, replay/idempotency, and subsystem-specific evidence.

#### Scenario: Matching endpoint identity supports bootstrap binding
- GIVEN a peer handshake includes a node identity ref and matching Iroh endpoint public identity ref
- WHEN peer bootstrap admission evaluates transport binding
- THEN it may bind the endpoint identity into the peer session diagnostics and receipts.

#### Scenario: Endpoint identity alone cannot authorize operation
- GIVEN a peer presents a known Iroh endpoint public identity but lacks operation authority evidence
- WHEN the peer submits a node-control or protocol operation
- THEN admission denies before side effects and diagnostics state that endpoint identity is not operation authority.

### Requirement: System MUST Emit receipts for handshake start, negotiation result, join admission, denial, and disconnect
r[molten.peer_bootstrap.receipts] The system MUST Emit receipts for handshake start, negotiation result, join admission, denial, and disconnect.

### Requirement: System MUST Define feature vectors for runtime version, registry protocol, schema identity, Preserves boundary, handler profiles, transport support, and replay support
r[molten.peer_bootstrap.feature_vector] The system MUST Define feature vectors for runtime version, registry protocol, schema identity, Preserves boundary, handler profiles, transport support, and replay support.

### Requirement: System MUST Select the highest mutually admitted feature set and deny unsafe downgrades unless policy explicitly allows them
r[molten.peer_bootstrap.negotiation_policy] The system MUST Select the highest mutually admitted feature set and deny unsafe downgrades unless policy explicitly allows them.

### Requirement: System MUST Represent capability offers and requests as scoped, attenuated, expiring, policy-gated records
r[molten.peer_bootstrap.capability_offers] The system MUST Represent capability offers and requests as scoped, attenuated, expiring, policy-gated records.

### Requirement: System MUST Include negotiated resource limits and quotas in join agreements
r[molten.peer_bootstrap.resource_limits] The system MUST Include negotiated resource limits and quotas in join agreements.

### Requirement: System MUST Gate gossip topic and Iroh docs namespace joins through negotiated agreements and authority checks
r[molten.peer_bootstrap.topic_doc_join] The system MUST Gate gossip topic and Iroh docs namespace joins through negotiated agreements and authority checks.

### Requirement: System MUST Use peer agreements to determine remote artifact sync and catalog visibility behavior
r[molten.peer_bootstrap.remote_sync_join] The system MUST Use peer agreements to determine remote artifact sync and catalog visibility behavior.

### Requirement: System MUST Gate protocol sessions and job pools through peer agreements
r[molten.peer_bootstrap.protocol_job_join] The system MUST Gate protocol sessions and job pools through peer agreements.

### Requirement: System MUST Define how future Raft/control-plane membership joins use stronger admission
r[molten.peer_bootstrap.raft_join_plan] The system MUST Define how future Raft/control-plane membership joins use stronger admission.

### Requirement: System MUST Add loopback handshake tests for compatible feature negotiation and join admission
r[molten.peer_bootstrap.loopback_tests] The system MUST Add loopback handshake tests for compatible feature negotiation and join admission.

### Requirement: System MUST Add tests that unsafe downgrade attempts are denied
r[molten.peer_bootstrap.downgrade_tests] The system MUST Add tests that unsafe downgrade attempts are denied.

### Requirement: System MUST Add tests that capability offers do not grant authority until accepted and admitted
r[molten.peer_bootstrap.capability_tests] The system MUST Add tests that capability offers do not grant authority until accepted and admitted.

### Requirement: System MUST Add Hegel property tests for negotiation determinism, no-ambient-authority, and denied-join safety
r[molten.peer_bootstrap.property_tests] The system MUST Add Hegel property tests for negotiation determinism, no-ambient-authority, and denied-join safety.

### Requirement: Live peer admission binds ticket scope
r[molten.peer_admission_state_proof.ticket_scope] Molten MUST prove that live peer admission accepts tickets only when node id, peer id, topic, endpoint, freshness, and policy evidence match the receiver and requested operation scope.

#### Scenario: Ticket for wrong topic denies
- GIVEN a live peer ticket issued for one topic
- WHEN the receiver imports or admits it for another topic
- THEN peer admission decision is `deny`
- AND the ticket cannot satisfy node-control ingress admission.

### Requirement: Transport identity is not bootstrap authority
r[molten.peer_admission_state_proof.transport_not_bootstrap] Molten MUST prove that observed transport identity, neighbor records, listener receipts, and live send receipts cannot replace explicit peer admission or bootstrap tickets.

#### Scenario: Neighbor observation cannot bootstrap
- GIVEN a live transport neighbor observation and no peer admission receipt
- WHEN node-control ingress evaluates bootstrap evidence
- THEN admission is denied before enqueue
- AND diagnostics state that transport evidence is not bootstrap authority.

### Requirement: Peer roles resolve capability tokens at use time
r[molten.capability_token.peer_roles] Molten MUST require subscriber, publisher, relay, sync participant, job worker, node-control operator, and peer promotion roles to reference capability tokens or proofsets that are resolved by capability admission at use time.

#### Scenario: Publisher role requires write token admission
- GIVEN a peer session records a requested publisher role for one topic
- WHEN the peer attempts to publish
- THEN Molten resolves the referenced write token or proofset for that peer/session/topic
- AND publish denies if capability admission does not pass.

### Requirement: Capability validation is reproducible
r[molten.capability_token.validation] Molten SHOULD validate the capability-token spine with focused capability tests, peer-session tests, subscriber tests, promotion tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Transport-as-token regression fails validation
- GIVEN a regression accepts a transport receipt as capability authority
- WHEN focused capability-token validation runs
- THEN the negative transport-as-token fixture fails
- AND the change cannot complete until transport receipts are classified as evidence only.

### Requirement: Peer session lifecycle records are canonical
r[molten.peer_session.lifecycle_model] Molten MUST define canonical peer profile and peer session records that bind peer identity, endpoint expectations, negotiated joins, admitted scopes, resource bounds, freshness, revocation state, lifecycle state, and evidence refs.

#### Scenario: Session records bind admitted evidence
- GIVEN a peer has a matching profile, live ticket, peer admission receipt, negotiated agreement, and resource evidence
- WHEN Molten builds the peer session record
- THEN the session binds the profile ref, identity refs, ticket refs, admission refs, admitted scopes, resource refs, freshness data, and lifecycle state
- AND the session ref is derived from canonical Preserves bytes.

### Requirement: Peer lifecycle transitions are explicit
r[molten.peer_session.lifecycle_transitions] Molten MUST advance peer sessions only through explicit lifecycle transition receipts and MUST deny invalid skips, stale evidence, revoked evidence, and quarantine bypasses.

#### Scenario: Quarantined peer cannot reconnect implicitly
- GIVEN a peer session is marked quarantined by a canonical transition receipt
- WHEN a live transport neighbor observation or send receipt is observed for that peer
- THEN the session remains quarantined
- AND reconnect requires an explicit admitted transition out of quarantine.

### Requirement: Peer sessions do not grant authority
r[molten.peer_session.authority_boundary] Molten MUST NOT treat a peer profile, peer agreement, peer session, live endpoint, topic membership, or transport observation as authority for side-effecting operations.

#### Scenario: Connected peer lacks operation authority
- GIVEN a peer session is connected and has a valid live ticket admission
- WHEN the peer submits a node-control operation without a matching authority grant
- THEN ingress denies before enqueue or side effects
- AND diagnostics state that the peer session is not operation authority.

### Requirement: Peer diagnostics explain missing gates
r[molten.peer_session.diagnostics] Molten SHOULD produce peer diagnostics that separately report transport reachability, bootstrap admission, capability admission, authority grant, policy/resource admission, replay/idempotency status, and the next missing operator step.

#### Scenario: Missing authority is actionable
- GIVEN a peer has a valid session and bootstrap admission but no imported authority grant for the requested operation
- WHEN `molten peer diagnose` evaluates the peer
- THEN the diagnostic identifies bootstrap as present and authority as missing
- AND it names the import or grant step required before live send can pass.

### Requirement: Peer session tests cover positive and negative evidence
r[molten.peer_session.positive_negative_tests] Molten SHOULD cover peer sessions with positive lifecycle tests and negative tests for stale tickets, wrong topics, missing admissions, missing authority, revoked profiles, unsafe static config, and transport-only evidence.

#### Scenario: Transport-only negative fixture denies
- GIVEN a fixture contains live neighbor or send transport evidence but no peer admission receipt
- WHEN peer session validation evaluates bootstrap state
- THEN validation denies admission
- AND the diagnostic says transport evidence is not bootstrap authority.

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

### Requirement: Subscriber roles are scoped read capabilities
r[molten.peer_subscriber.role_model] Molten MUST model subscriber and read-only peer roles as scoped, attenuated read capabilities that bind projection kind, allowed scope, egress policy, redaction profile, resource limits, expiry, revocation, policy refs, resource refs, and evidence refs.

#### Scenario: Subscriber role admits observe-only scope
- GIVEN a peer session has a subscription grant for one remote dataspace topic projection
- WHEN the peer requests read access to that projection
- THEN Molten admits only the declared projection scope
- AND the role does not admit publish, assert, retract, control, execution, retention, import, relay, or authority-delegating operations.

### Requirement: Subscription grants and projection receipts are canonical
r[molten.peer_subscriber.subscription_grant] Molten MUST define canonical subscription grant and projection receipt records for read-only peer delivery decisions.

#### Scenario: Projection receipt binds filter decision
- GIVEN a subscriber receives a redacted service-status projection
- WHEN Molten emits the projection receipt
- THEN the receipt binds the subscription grant ref, peer/session refs, source ref, projection ref, egress policy ref, redaction decision, resource consumption, replayability flag, and diagnostics.

### Requirement: Read-only still requires authority
r[molten.peer_subscriber.read_requires_authority] Molten MUST require explicit read authority, policy, resource, and egress admission before delivering subscription data to a read-only peer.

#### Scenario: Missing read grant denies delivery
- GIVEN a peer is connected and admitted for transport but has no subscription grant for a topic
- WHEN the peer requests that topic projection
- THEN delivery denies before data egress
- AND diagnostics state that read-only access still requires explicit read authority.

### Requirement: Subscriber grants cannot upgrade to writes
r[molten.peer_subscriber.no_write_upgrade] Molten MUST deny attempts to use subscriber or read-only grants for publish, assert, retract, node-control mutation, job execution, sync import, retention clearance, authority delegation, or destructive operations.

#### Scenario: Subscriber publish attempt denies
- GIVEN a peer has a read-only subscription grant for a topic
- WHEN the peer attempts to publish or assert into that topic
- THEN Molten denies before routing the write
- AND diagnostics identify the grant as read-only.

### Requirement: Subscriber relay and republish are separate scopes
r[molten.peer_subscriber.no_relay_republish] Molten MUST deny relay, republish, cache export, and transitive subscription from read-only peers unless the subscription grant explicitly includes those attenuated scopes.

#### Scenario: Read-only peer cannot republish inventory
- GIVEN a peer receives catalog inventory under a read-only subscription grant
- WHEN it attempts to republish that inventory as an authority-bearing announcement
- THEN Molten denies republish authority
- AND the original projection receipt remains evidence-only.

### Requirement: Subscriber roles have positive and negative tests
r[molten.peer_subscriber.positive_negative_tests] Molten SHOULD include positive subscriber projection tests and negative tests for missing read authority, egress denial, stale grants, write-upgrade attempts, unauthorized republish, read-only sync import, and Raft learner confusion.

#### Scenario: Stale grant fixture denies
- GIVEN a subscriber grant is expired or revoked
- WHEN projection delivery validation runs
- THEN the decision is deny
- AND diagnostics identify the stale or revoked grant.

### Requirement: Peer bootstrap separates Raft membership roles
r[molten.raft_membership_admission.peer_boundary] Molten MUST require a separate Raft/control-plane membership-change command and membership receipt before any peer receives a voter, non-voter, learner, or control-plane membership role.

#### Scenario: Peer agreement admits gossip but not Raft
- GIVEN a peer agreement admits a peer for a gossip topic, docs namespace, protocol session, or job pool
- WHEN the peer requests Raft/control-plane membership
- THEN peer bootstrap evidence is treated only as supporting input
- AND a separate membership preflight and commit path remains required.

### Requirement: Raft membership admission validation is reproducible
r[molten.raft_membership_admission.validation] Molten SHOULD validate membership admission changes with focused consensus tests, peer-bootstrap boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Boundary regression fails validation
- GIVEN a regression lets a peer agreement alone satisfy a Raft membership role
- WHEN focused membership admission validation runs
- THEN the negative peer-bootstrap boundary test fails
- AND the change cannot complete until separate membership admission is restored.

### Requirement: Peer promotions require explicit preflight and apply
r[molten.peer_promotion.preflight_apply] Molten MUST separate peer promotion preflight from promotion apply so operators can inspect requested role deltas, diagnostics, missing evidence, and expected session changes before mutating peer-session state.

#### Scenario: Preflight does not mutate session
- GIVEN a peer requests promotion from read-only subscriber to scoped publisher
- WHEN promotion preflight passes
- THEN Molten emits a preflight receipt and readback summary
- AND the peer session remains read-only until an explicit apply operation passes.

### Requirement: Subscriber write upgrade requires promotion
r[molten.peer_promotion.subscriber_upgrade] Molten MUST require an explicit passing promotion grant and apply receipt before a subscriber or read-only peer gains publish, assert, retract, relay, sync-import, execution, or mutation capabilities.

#### Scenario: Subscriber cannot publish before apply
- GIVEN a subscriber has a passing promotion preflight but no promotion apply receipt
- WHEN it attempts to publish to the target topic
- THEN the write denies
- AND diagnostics state that promotion has not been applied.

### Requirement: Promotion diagnostics explain role deltas
r[molten.peer_promotion.diagnostics] Molten SHOULD render diagnostics that identify current roles, requested roles, admitted and denied capability deltas, missing promotion authority, expiry/revocation, approval requirements, and next operator actions.

#### Scenario: Missing promotion authority names next step
- GIVEN a peer requests promotion to a scoped publisher role without a promotion grant
- WHEN diagnostics render the request
- THEN they report current subscriber role, requested publisher role, missing promotion authority, and the required grant/import step.

### Requirement: Promotion validation is reproducible
r[molten.peer_promotion.validation] Molten SHOULD validate promotion and demotion with focused promotion tests, peer-session tests, subscriber/read-only tests, authority tests, consensus boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Subscriber write-upgrade regression fails validation
- GIVEN a regression allows a subscriber to publish without promotion apply
- WHEN focused promotion validation runs
- THEN the negative subscriber upgrade test fails
- AND the change cannot complete until promotion apply is required again.

### Requirement: Peer session transition relation is closed
r[molten.peer_session.transition_relation_closed] Molten MUST define a reviewed finite peer-session transition relation over prior state, requested event, target state, and explicit guard facts, and MUST deny any peer-session transition not present in that relation before advancing session state.

#### Scenario: Admitted peer reaches connected through reviewed steps
- GIVEN a peer session has passed discovery, invitation, handshake, negotiation, bootstrap admission, and authority/resource guard checks
- WHEN the connect event is evaluated against the reviewed transition relation
- THEN Molten emits a passing transition receipt
- AND the after-state is connected with the guard evidence refs bound.

#### Scenario: Discovered peer cannot jump to connected
- GIVEN a peer session is only discovered and has no admitted bootstrap or negotiation evidence
- WHEN a connected target state is requested
- THEN the transition decision is deny
- AND the prior session state ref remains unchanged.

### Requirement: Terminal and quarantine states require explicit recovery
r[molten.peer_session.terminal_quarantine_guards] Molten MUST prevent expired, revoked, or quarantined peer sessions from becoming admitted or connected again unless an explicit recovery or re-admission event with current policy, freshness, revocation, bootstrap, and resource evidence passes.

#### Scenario: Revoked peer cannot reconnect from transport observation
- GIVEN a peer session is revoked
- WHEN a live transport neighbor observation or send receipt is supplied as reconnect evidence
- THEN the transition decision is deny
- AND diagnostics state that transport evidence cannot exit the revoked state.

#### Scenario: Quarantined peer recovers with explicit evidence
- GIVEN a peer session is quarantined and current policy permits a recovery workflow
- WHEN recovery evidence, current freshness, revocation checks, bootstrap admission, and resource evidence are supplied
- THEN Molten may emit a passing recovery transition receipt to the reviewed recovery target state.

### Requirement: Peer transition receipts bind state refs
r[molten.peer_session.transition_receipt_binding] Peer-session transition receipts MUST bind the peer/session identity, from-state, requested event, target-state, before-state ref, after-state ref or preserved-state ref, guard evidence refs, decision, diagnostics, and an evidence-only caveat that peer session state does not grant operation authority.

#### Scenario: Denied transition preserves state ref
- GIVEN a peer transition request is denied for a wrong topic or missing guard evidence
- WHEN the transition receipt is emitted
- THEN the receipt binds the original before-state ref as the preserved state
- AND no connected, admitted, or authority-bearing state is minted by the denial.

### Requirement: Peer transition tests cover the relation
r[molten.peer_session.transition_trace_tests] Molten SHOULD include positive and negative peer-session transition tests, including bounded generated traces, that cover reviewed state progression, invalid skips, wrong topics, stale tickets, revoked evidence, quarantine bypass, missing admissions, missing authority, and transport-only evidence.

#### Scenario: Generated peer trace rejects invalid edge
- GIVEN a generated peer-session transition trace includes a state/event pair outside the reviewed relation
- WHEN the trace is evaluated
- THEN the invalid edge emits a deny receipt
- AND all later state assertions derive from the preserved prior state.

### Requirement: Peer claim authority is separate from peer admission
r[molten.claim_authority.peer_diagnostics] Peer diagnostics SHOULD report external claim authority as a separate gate from transport reachability, peer bootstrap admission, peer session lifecycle, handoff import, authority grants, policy/resource admission, provenance, replay/idempotency, and execution readiness.

#### Scenario: Friendly peer lacks claim authority
- GIVEN a peer session is connected and bootstrap-admitted
- AND no admitted `claim:attest` capability proofset exists for the requested claim domain
- WHEN `molten peer diagnose` or equivalent readback evaluates the peer
- THEN diagnostics report bootstrap/session as present and claim authority as missing
- AND the next step names the needed capability/UCAN/Basalt proof or claim admission.

### Requirement: Peer sessions can be claim context but not claim authority
r[molten.claim_authority.peer_session_context] Peer sessions, live tickets, peer admissions, and handoff bundles MAY be referenced as holder/session/context evidence for claim capability requests, but MUST NOT satisfy claim authority without matching capability admission and UCAN/Basalt proof receipts.

#### Scenario: Session-bound proof admits claim context
- GIVEN a capability proofset holder and session match a connected peer session
- AND UCAN/Basalt/capability admission passes for `claim:attest` on the requested selector and scope
- WHEN an external claim is admitted
- THEN the claim admission may bind the peer session ref as context evidence.

#### Scenario: Handoff import alone cannot attest
- GIVEN a peer handoff bundle imports a ticket and peer admission for an external cluster
- WHEN the peer presents a claim without matching claim authority proof
- THEN claim admission denies
- AND diagnostics state that handoff evidence is not a claim-attestation capability.

### Requirement: Peer claim diagnostics have positive and negative tests
r[molten.claim_authority.peer_diagnostic_tests] Peer diagnostic tests SHOULD include positive claim-authority readback and negative cases for missing proof, stale session, revoked peer profile, handoff-only evidence, transport-only evidence, wrong selector, and wrong claim kind.

#### Scenario: Transport-only diagnostic names missing proof
- GIVEN only live transport evidence exists for a peer
- WHEN peer diagnostics evaluate external claim authority
- THEN diagnostics classify transport as reachable or observed only
- AND report claim authority denied until capability/UCAN/Basalt evidence passes.

### Requirement: Live topology profile constrains node-control peers
r[molten.peer_bootstrap.live_topology_profile] Molten SHOULD define canonical live topology profiles that declare expected node ids, peer ids, topics, endpoint constraints, allowed ALPN surfaces, ticket refs, peer-admission refs, and optional role labels for live node-control workflows.

#### Scenario: Matching topology profile admits preflight
- GIVEN a live topology profile declares node `node:receiver`, peer `peer:operator`, topic `node-control`, and a matching live ticket ref
- WHEN a live-send or workflow-bundle apply preflight evaluates those command inputs
- THEN topology preflight passes and records the topology profile ref for receipt binding.

#### Scenario: Wrong peer or topic denies
- GIVEN a live topology profile is scoped to one expected peer and topic
- WHEN a command attempts to send using a different peer or topic
- THEN topology preflight denies before join or publish
- AND diagnostics name the mismatched peer or topic.

### Requirement: Live transport profile controls retry and timeout policy
r[molten.peer_bootstrap.live_transport_profile] Molten SHOULD define canonical live transport profiles for retry attempts, join timeout, publish timeout, relay/direct preference, listener event bounds, and diagnostic expectations. Transport profile values MUST be admitted under the runtime limit hard caps before use.

#### Scenario: Admitted transport profile drives live send
- GIVEN a transport profile selects retry and timeout values within hard caps
- WHEN live-send preflight admits the profile
- THEN the live-send shell uses the admitted effective values
- AND the send receipt binds the profile ref or effective values.

#### Scenario: Timeout above hard cap denies
- GIVEN a transport profile selects a join timeout above the runtime hard cap
- WHEN live transport preflight evaluates the profile
- THEN preflight denies before network join
- AND diagnostics identify the selected timeout and cap.

### Requirement: Live profile receipts bind constraints
r[molten.peer_bootstrap.live_profile_receipts] Live node-control receipts that use topology or transport profiles SHOULD bind the selected profile refs, effective peer/topic/endpoint constraints, and admitted retry/timeout values while continuing to bind normal ticket, peer-admission, authority, policy, resource, and evidence refs separately.

#### Scenario: Receipt records topology and transport refs
- GIVEN a live-send operation uses both topology and transport profiles
- WHEN it emits transport and send receipts
- THEN the receipts record the topology profile ref, transport profile ref, effective peer/topic constraints, and admitted retry/timeout values.

#### Scenario: Explicit flags remain reviewable
- GIVEN an operator uses explicit peer/topic/timeout flags without profiles
- WHEN live receipts are emitted
- THEN receipts record the explicit effective values and a no-profile caveat
- AND downstream review can distinguish explicit flags from reviewed profiles.

### Requirement: Topology and transport profiles are non-authority
r[molten.peer_bootstrap.live_profiles_non_authority] Live topology and transport profiles MUST NOT grant authority, policy admission, resource rights, provenance trust, source-gate acceptance, retention clearance, execution permission, or capability delegation.

#### Scenario: Transport-as-authority attempt denies
- GIVEN a caller supplies a matching topology and transport profile but omits the required authority or policy evidence for a live mutation
- WHEN live ingress or send admission evaluates the request
- THEN admission denies
- AND diagnostics state that live profile evidence is not authority or policy.
