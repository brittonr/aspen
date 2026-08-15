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
