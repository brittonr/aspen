## ADDED Requirements

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
