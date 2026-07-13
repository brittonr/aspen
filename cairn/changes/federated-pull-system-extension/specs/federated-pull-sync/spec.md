## ADDED Requirements

### Requirement: Federated pull runs as an optional system extension
r[molten.federated_pull_runtime.manifest] Molten MUST require a canonical system-extension manifest for live federation, binding service identity, generation, peer configuration, protocol version, crypto, transport, DAG and content bindings, resource and rate profiles, local admission policy, evidence profile, and non-claims. Federation MUST NOT become an implicit node, transport, or global dataspace mode.

#### Scenario: Federation activates explicitly
- GIVEN a reviewed manifest and complete compatible port bindings
- WHEN system-extension admission and activation pass
- THEN the node MAY run the federated-pull service under its declared generation.

#### Scenario: Node without federation remains valid
- GIVEN no federation extension is installed
- WHEN the node runs local or cluster workloads
- THEN those workloads MUST NOT require federation state or protocols.

### Requirement: Federation uses production cryptographic adapters
r[molten.federated_pull_runtime.crypto] Molten MUST sign and verify canonical announcements, inventories, delegates, requests, and responses through admitted cryptographic adapters that bind purpose, domain, signer public ref, key generation, freshness, verifier context, and payload ref. Fixture signatures MUST NOT satisfy production federation admission.

#### Scenario: Current signed inventory is accepted as a hint
- GIVEN a current allowed peer signs an inventory with the admitted federation inventory purpose and domain
- WHEN verification passes
- THEN the inventory MAY enter candidate hint state with its signature and freshness refs.

#### Scenario: Wrong-purpose signature is denied
- GIVEN a cryptographically valid signature from another purpose or stale generation
- WHEN federation verification runs
- THEN it MUST deny before fetch planning or local import.

### Requirement: Discovery evidence remains hint-only
r[molten.federated_pull_runtime.discovery_hints] Molten MUST treat static configuration, endpoint observations, gossip, trackers, DHT or pkarr-style pointers, catalog entries, probes, announcements, and inventories as candidate-location hints. None of these inputs MAY independently import, pin, install, execute, merge, or establish local trust for content.

#### Scenario: Static peer becomes a candidate
- GIVEN an operator configures a peer with current identity and policy refs
- WHEN discovery readback runs
- THEN the peer MAY appear as a candidate with bounded freshness metadata.

#### Scenario: Reachable peer has no import authority
- GIVEN an authenticated transport peer advertises an artifact
- WHEN no receiver fetch, verification, or local admission has passed
- THEN the artifact MUST remain unimported.

### Requirement: Pull planning is receiver-owned
r[molten.federated_pull_runtime.receiver_pull] Molten MUST compute peer selection, inventory difference, missing sets, DAG/content fetch strategies, resource admission, and local import prerequisites on the receiver. Remote push messages MAY update hint state only and MUST NOT mutate trusted local registries.

#### Scenario: Receiver imports verified admitted content
- GIVEN the receiver selects a missing ref, fetches it through admitted DAG/content ports, verifies canonical identity, and passes local admission
- WHEN import commits
- THEN local state MAY record the artifact and MUST bind all prerequisite refs.

#### Scenario: Unsolicited push is ignored or denied
- GIVEN a remote peer sends bytes or an import request absent a receiver plan
- WHEN federation handles it
- THEN trusted local state MUST remain unchanged.

### Requirement: Federation sessions are bounded and fresh
r[molten.federated_pull_runtime.bounds_freshness] Molten MUST enforce per-service, per-peer, per-resource, and per-session limits for inventory entries, query frequency, concurrent sessions, bytes, DAG nodes, content chunks, retries, logical time, freshness, diagnostics, and retained status. Stale, oversized, unavailable, or over-budget work MUST deny or defer explicitly.

#### Scenario: Bounded anti-entropy session completes
- GIVEN peer inventory and missing content remain within admitted limits
- WHEN a pull session runs
- THEN every request and terminal outcome MUST remain correlated to the service generation and session ref.

#### Scenario: Oversized inventory is rejected
- GIVEN a peer sends more inventory entries or bytes than admitted
- WHEN outer record validation runs
- THEN the session MUST deny or truncate only according to explicit protocol policy before allocating unbounded state.

### Requirement: Conflict and merge policy is domain-owned
r[molten.federated_pull_runtime.conflict_boundary] Molten MUST report divergent refs, ancestry, signer, freshness, and candidate policy as canonical conflict evidence while leaving winner selection, merge, branch, reject, and publication semantics to the owning artifact or application extension. Federation MUST NOT apply a global pull-wins, last-writer, or automatic merge rule.

#### Scenario: Divergent heads produce conflict evidence
- GIVEN local and remote resources use the same logical name with different canonical heads
- WHEN inventory comparison runs
- THEN federation MUST report the divergence without replacing either head automatically.

### Requirement: Federation exposes local bounded status
r[molten.federated_pull_runtime.status_evidence] Molten MUST expose local status assertions and operator readback for configured and observed peers, freshness, inventory sessions, missing sets, planned and completed fetches, verification, admission, denial, conflicts, resources, and latest evidence refs. Readback MUST redact secrets and MUST NOT claim permanent convergence or remote correctness.

#### Scenario: Operator inspects a denied peer
- GIVEN a peer has stale signatures and failed fetches
- WHEN authorized status readback runs
- THEN it MUST show bounded denial and freshness diagnostics without exposing raw credentials or payloads.

### Requirement: Federation uses the same core in live and simulation profiles
r[molten.federated_pull_runtime.final_validation] Molten MUST run the same manifest, signed-domain validation, hint handling, missing-set planning, rate decisions, conflict classification, and local-admission prerequisites under live and deterministic-simulation adapters. It MUST test positive signed pull/import flows and negative wrong-key, wrong-purpose, revoked delegate, stale inventory, malicious peer, unsolicited content, over-budget, partition, restart, corruption, conflict, and no-push-import cases.

#### Scenario: Live and simulated pull share semantics
- GIVEN equivalent bounded no-fault profiles and canonical inventories
- WHEN one pull fixture runs through live loopback and simulation
- THEN missing-set, fetch, verification, admission, and terminal traces MUST fall within the same allowed set.

#### Scenario: Signature-only import fails conformance
- GIVEN an implementation imports content solely because an announcement signature verifies
- WHEN federation conformance runs
- THEN it MUST fail the verification-before-import and local-admission invariants.
