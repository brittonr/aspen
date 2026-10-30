# Federated Pull Sync Specification

## Purpose

Defines the `federated-pull-sync` capability.

## Requirements

### Requirement: System MUST Define signed announcement and inventory records for federated resources
r[molten.federation.announcement_model] The system MUST Define signed announcement and inventory records for federated resources.

### Requirement: System MUST Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas
r[molten.federation.resource_types] The system MUST Define initial federated resource types for artifacts, chunk manifests/chunks, docs/catalog metadata, receipts, provenance, transcripts, protocols, and schemas.

### Requirement: System MUST Document that federation does not imply global Raft, global dataspace, or pushed state
r[molten.federation.no_global_consistency] The system MUST Document that federation does not imply global Raft, global dataspace, or pushed state.

### Requirement: System MUST Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge
r[molten.federation.receipts] The system MUST Emit receipts for discovery, query, fetch, verification, admission, import, denial, and merge.

### Requirement: System MUST Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow
r[molten.federation.pull_workflow] The system MUST Implement receiver-driven query, missing-set calculation, fetch plan, verification, and local admission flow.

### Requirement: System MUST Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import
r[molten.federation.verification_layers] The system MUST Verify origin signatures, delegate/capability signatures, content/chunk hashes, and local policy before import.

### Requirement: System MUST Support static/configured peers as the first discovery mode
r[molten.federation.static_discovery] The system MUST Support static/configured peers as the first discovery mode.

### Requirement: System MUST Apply resource and rate-limit policy to announcements, inventory queries, and fetches
r[molten.federation.rate_limits] The system MUST Apply resource and rate-limit policy to announcements, inventory queries, and fetches.

### Requirement: System MUST Represent sync status, imported resources, denials, and peer availability as local dataspace assertions
r[molten.federation.dataspace_status] The system MUST Represent sync status, imported resources, denials, and peer availability as local dataspace assertions.

### Requirement: System MUST Integrate with remote artifact sync and content-addressed chunk-store fetch/verification
r[molten.federation.remote_sync_chunk_store] The system MUST Integrate with remote artifact sync and content-addressed chunk-store fetch/verification.

### Requirement: System MUST Add loopback tests for signed announcement, pull fetch, verification, import, and denial
r[molten.federation.loopback_tests] The system MUST Add loopback tests for signed announcement, pull fetch, verification, import, and denial.

### Requirement: System MUST Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants
r[molten.federation.property_tests] The system MUST Add Hegel property tests for receiver-driven sync, no-push-import, and verification-before-import invariants.

### Requirement: Federation read-only roles are hint-only
r[molten.peer_subscriber.federation_readonly] Molten MUST treat federation inventory, catalog, and anti-entropy subscriptions as readback or hint surfaces that cannot import artifacts, mutate registries, or establish trust without receiver-driven verification and admission.

#### Scenario: Read-only inventory cannot import artifact
- GIVEN a read-only subscriber receives a federation inventory item for an artifact
- WHEN the receiver has not fetched, hash-verified, and admitted the artifact through local policy
- THEN the artifact remains unimported
- AND the inventory projection is recorded only as hint/readback evidence.

### Requirement: Anti-entropy status is explicit local state
r[molten.eventual_surface.anti_entropy_status] Molten SHOULD represent anti-entropy query results, missing-set calculations, fetch plans, imports, denials, and peer availability as local dataspace assertions with canonical evidence refs.

#### Scenario: Missing set is visible without importing trust
- GIVEN a peer advertises inventory for a federated surface
- WHEN the receiver computes its missing set
- THEN Molten records local status assertions for missing, already-present, denied, and planned fetch refs
- AND those assertions do not import remote content until verification and admission pass.

### Requirement: Remote sync imports require verification and admission
r[molten.eventual_surface.remote_sync_boundary] Molten MUST treat federation announcements, gossip hints, docs observations, and inventory records as propagation hints that cannot import artifacts or mutate registries without receiver-driven verification and local admission.

#### Scenario: Announcement alone cannot import artifact
- GIVEN a peer announces an artifact ref over gossip or federation inventory
- WHEN the receiver has not fetched, hash-verified, and admitted the artifact
- THEN the artifact remains unimported
- AND any status assertion records the announcement as a hint rather than local trust.

### Requirement: Eventual surface validation is reproducible
r[molten.eventual_surface.validation] Molten SHOULD validate eventual propagation surfaces with focused merge-law tests, remote dataspace tests, federation anti-entropy tests, remote sync boundary tests, formatting, and Cairn validation before archiving.

#### Scenario: Hint-only import regression fails
- GIVEN a regression imports an artifact from a federation announcement without content verification
- WHEN focused eventual-surface validation runs
- THEN the negative remote-sync boundary fixture fails
- AND the change cannot complete until receiver-driven verification is restored.

### Requirement: Iroh locator records are canonical discovery evidence
r[molten.iroh_discovery.locator_records] Molten MUST define canonical locator announcement, query, result, and probe receipt records for signed peer claims, tracker responses, pkarr resolutions, static peer configuration, and catalog hints.

#### Scenario: Signed announcement becomes locator evidence
- GIVEN a peer publishes a signed availability claim for a content ref or chunk manifest ref
- WHEN Molten imports the claim through the locator boundary
- THEN it records a canonical locator announcement with signer, subject ref, availability class, freshness, and evidence-only caveats
- AND the announcement does not import, pin, install, expose, or execute content.

#### Scenario: Locator query preserves receiver criteria
- GIVEN a receiver queries for peers that may hold a resource
- WHEN the query is recorded
- THEN the locator query binds the requested content ref, completeness preference, verification preference, freshness policy, and resource bounds
- AND the query result remains a set of candidates rather than an admission decision.

#### Scenario: Probe receipt is bounded evidence only
- GIVEN a receiver probes a candidate peer for declared size or sampled chunk availability
- WHEN the probe completes
- THEN Molten emits a locator probe receipt with decision, diagnostics, peer ref, subject ref, and probe scope
- AND the receipt states that sampled availability is not proof of full possession.

### Requirement: Locator evidence is hint-only until receiver verification passes
r[molten.iroh_discovery.hint_only_boundary] Molten MUST treat locator announcements, tracker query results, pkarr pointer resolutions, endpoint observations, topic membership, and probe receipts as hint-only evidence that cannot import artifacts, mutate registries, expose bytes, or establish trust without receiver-driven fetch, hash verification, and local admission.

#### Scenario: Tracker result alone cannot import artifact
- GIVEN a tracker returns a peer for an artifact ref
- WHEN the receiver has not fetched, hash-verified, and admitted the artifact through local gates
- THEN the artifact remains unimported
- AND the locator result is recorded only as discovery evidence.

#### Scenario: Complete claim still requires verification
- GIVEN a peer claims complete availability for a manifest
- WHEN the receiver decides whether to install the manifest
- THEN Molten requires verified fetched bytes and local admission before install
- AND the complete claim only influences fetch planning and diagnostics.

#### Scenario: Transport-observed peer cannot satisfy federation authority
- GIVEN a peer is reachable over Iroh and appears in a gossip topic
- WHEN it advertises content without matching capability, policy, resource, and verification evidence
- THEN federation import denies
- AND diagnostics report transport and locator evidence separately from authority.

### Requirement: Pkarr pointers are optional locator inputs
r[molten.iroh_discovery.pkarr_optional_locator] Molten MAY resolve pkarr-style public-key indexed latest pointers as optional content locator inputs, but resolved records MUST bind signer, key, freshness, resolved subject ref, and evidence-only caveats.

#### Scenario: Fresh pointer becomes candidate locator
- GIVEN an operator enables pkarr locator discovery for a known public key
- WHEN resolution returns a fresh content ref
- THEN Molten records a locator result with the resolved ref, key, signer, and freshness diagnostics
- AND downstream sync still performs receiver-driven verification before import.

#### Scenario: Stale pointer denies as locator
- GIVEN a pkarr resolution is stale, malformed, signed by the wrong key, or points to an unsupported subject
- WHEN Molten validates the pointer
- THEN locator import denies with diagnostics
- AND no fetch, install, pin, or trust decision is produced from that pointer.

### Requirement: Locator diagnostics are visible in federation readback
r[molten.iroh_discovery.locator_readback] Molten SHOULD expose locator announcement, query, result, and probe receipts in federation/catalog/operator readback as discovery status while preserving separate authority, provenance, source-gate, policy, resource, retention, and execution gates.

#### Scenario: Operator sees missing admission separately
- GIVEN locator readback shows several candidate peers for a manifest
- WHEN no candidate has produced verified fetch and admission receipts
- THEN readback reports candidates as available hints
- AND diagnostics name the missing verification and admission evidence needed before import.

#### Scenario: Denied probe remains diagnostic
- GIVEN a peer probe denies because the peer is unreachable or returns inconsistent metadata
- WHEN catalog readback summarizes the locator state
- THEN the denied probe is visible as diagnostic evidence
- AND it does not mark the content as absent from all peers or corrupt without further evidence.

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
