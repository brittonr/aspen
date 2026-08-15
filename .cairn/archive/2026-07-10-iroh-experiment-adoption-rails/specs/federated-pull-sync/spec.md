# Federated Pull Sync Delta: Iroh Locator Discovery

## ADDED Requirements

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
