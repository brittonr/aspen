# Catalog Specification

## Purpose

Defines the `catalog` capability.

## Requirements

### Requirement: Canonical short-id prefix grammar
r[molten.catalog.short_id_canonical_prefixes] Molten MUST treat catalog short-id inputs as either canonical full content refs or lowercase hex prefixes without a `blake3:` scheme, and MUST NOT treat malformed ref-shaped strings as prefix searches.

#### Scenario: Ref-shaped malformed prefix denies
- GIVEN a catalog short-id input of `blake3:` or `blake3:<bad>`
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND candidate search is skipped with a malformed full-ref diagnostic

#### Scenario: Full canonical ref resolves exactly
- GIVEN a full canonical content ref visible in the catalog
- WHEN short-id resolution receives that full ref
- THEN the decision is `pass`
- AND the result expands to the same full ref

### Requirement: Short-id malformed denials
r[molten.catalog.short_id_malformed_denials] Molten MUST deny non-hex or uppercase short-id prefixes as canonical data-bearing denial results before downstream catalog operations receive them.

#### Scenario: Uppercase prefix denies
- GIVEN a short-id prefix containing uppercase hex characters
- WHEN short-id resolution runs
- THEN the decision is `deny`
- AND diagnostics state that short-id prefixes use lowercase hex characters

#### Scenario: Hidden-only prefix denies
- GIVEN a lowercase hex prefix that matches only hidden refs
- WHEN short-id resolution runs with those refs hidden
- THEN the decision is `deny`
- AND no hidden full ref is returned as the resolution

### Requirement: Replay receipts have ledger kinds
r[molten.determinism.replay_receipt_catalog.ledger_kind] The evidence ledger SHOULD classify `deterministic-replay-verify-v1` and `deterministic-first-divergence-v1` records with stable artifact kinds.

#### Scenario: Replay verify import is kinded
- GIVEN a generic replay verification receipt
- WHEN it is imported into the evidence ledger
- THEN the ledger artifact kind is `deterministic-replay-verify-receipt`

### Requirement: Replay verification records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_verify] The catalog SHOULD classify replay verification records by decision, divergence kind, expected/actual report refs when present, and state or output refs when present.

#### Scenario: Replay verify is found by final state
- GIVEN an imported replay verification receipt with expected report, actual report, and final-state refs
- WHEN catalog search filters by replay decision and final-state ref
- THEN the replay verification receipt is returned

### Requirement: First-divergence records are catalog-searchable
r[molten.determinism.replay_receipt_catalog.classify_divergence] The catalog SHOULD classify first-divergence records by divergence kind, actor/session/vat identifier when present, handler profile ref, expected ref, and actual ref.

#### Scenario: Divergence is found by kind
- GIVEN an imported deterministic first-divergence record for an effect-response mismatch
- WHEN catalog search filters by `replay-divergence:effect-response`
- THEN the first-divergence record is returned

### Requirement: Replay receipt catalog coverage is tested
r[molten.determinism.replay_receipt_catalog.tests] Molten SHOULD test ledger import and catalog search for generic replay verification and first-divergence evidence.

#### Scenario: Search returns replay evidence only
- GIVEN imported replay verification and first-divergence records
- WHEN catalog searches by replay decision, divergence, report refs, or final-state refs
- THEN matching replay evidence is returned without granting authority or replacing gate validation

### Requirement: Replay evidence MCP search is read-only
r[molten.catalog.replay_evidence_mcp.readonly_tool] Molten SHOULD expose generic deterministic replay evidence through a named read-only catalog MCP search tool.

#### Scenario: Replay MCP tool is allowed
- GIVEN a catalog MCP request for `search_replay_evidence`
- WHEN the MCP dispatcher checks the read-only allow-list
- THEN the request is allowed as a read-only catalog query
- AND mutating catalog tools remain denied

### Requirement: Replay evidence MCP filters map to catalog classifications
r[molten.catalog.replay_evidence_mcp.filter_args] Molten SHOULD map replay-specific MCP arguments to existing deterministic replay catalog classifications, including decision, divergence kind, actor identifier, handler profile ref, expected and actual report refs, final-state refs, output refs, and effect-log refs.

#### Scenario: Replay verify evidence is found by final state
- GIVEN an imported `deterministic-replay-verify-v1` record
- WHEN `search_replay_evidence` receives `stage`, `decision`, and `final-state-ref` filters
- THEN the MCP response includes the matching replay verification evidence

#### Scenario: First divergence evidence is found by divergence refs
- GIVEN an imported `deterministic-first-divergence-v1` record
- WHEN `search_replay_evidence` receives `stage`, `divergence`, `handler-profile-ref`, and `actual-ref` filters
- THEN the MCP response includes the matching first-divergence evidence

### Requirement: Replay evidence MCP search is evidence-only
r[molten.catalog.replay_evidence_mcp.tests] Molten SHOULD test replay evidence MCP readback and receipt binding without treating search results as authority, policy admission, provenance trust, source-gate acceptance, or replay verification.

#### Scenario: Replay MCP receipt binds readback only
- GIVEN replay evidence search through MCP
- WHEN the call succeeds
- THEN the MCP receipt binds the request, response, and catalog receipt
- AND the receipt keeps the read-only and mutating-tools-denied checks

### Requirement: Replay rollups summarize verification evidence
r[molten.determinism.replay_rollup.schema] Molten SHOULD emit `deterministic-replay-rollup-v1` evidence over bounded sets of generic replay verification receipts.

#### Scenario: Mixed replay receipts are summarized
- GIVEN one passing replay verification receipt and one denying replay verification receipt
- WHEN a replay rollup is generated
- THEN the rollup records total, pass, deny, and divergence counts
- AND the rollup decision is `deny`

### Requirement: Replay rollups reject stale inputs
r[molten.determinism.replay_rollup.validation] Molten SHOULD make replay rollups deny when an expected replay receipt ref does not match the supplied receipt value or when an input is not a replay verification receipt.

#### Scenario: Mismatched replay receipt ref denies
- GIVEN a replay rollup input with an expected ref for different content
- WHEN the rollup is generated
- THEN the rollup decision is `deny`
- AND diagnostics include the expected and actual refs

### Requirement: Replay rollups are catalog-searchable
r[molten.determinism.replay_rollup.catalog] The evidence ledger and catalog SHOULD classify replay rollups by artifact kind, decision, pass count, deny count, total count, and divergence kinds present.

#### Scenario: Replay rollup is found by decision
- GIVEN an imported replay rollup
- WHEN catalog search filters by `deterministic-replay-rollup` and `replay-rollup-decision:pass`
- THEN the replay rollup is returned

### Requirement: Replay rollup readback is tested
r[molten.determinism.replay_rollup.tests] Molten SHOULD test replay rollup generation, stale input denial, catalog search, and replay MCP readback while preserving evidence-only semantics.

#### Scenario: Replay rollup MCP readback is evidence only
- GIVEN an imported replay rollup
- WHEN replay evidence MCP search filters by rollup stage
- THEN the rollup is returned with read-only MCP receipt evidence
- AND the rollup does not replace individual replay verification or gate validation

### Requirement: Replay indexes group replay evidence
r[molten.determinism.replay_index.schema] Molten SHOULD emit `deterministic-replay-index-v1` evidence over bounded sets of generic replay verification receipts and replay rollups.

#### Scenario: Raw receipts and rollups are indexed
- GIVEN a replay rollup and a raw replay verification receipt
- WHEN a replay index is generated
- THEN the index records total, pass, deny, raw-receipt, rollup, and divergence counts
- AND the index records referenced replay receipt and rollup refs

### Requirement: Replay indexes reject stale inputs
r[molten.determinism.replay_index.validation] Molten SHOULD make replay indexes deny when an expected replay evidence ref does not match the supplied value or when an input is not a replay verify receipt or replay rollup.

#### Scenario: Mismatched replay rollup ref denies
- GIVEN a replay index input with an expected ref for different content
- WHEN the index is generated
- THEN the index decision is `deny`
- AND diagnostics include the expected and actual refs
- AND the mismatched input is not counted as valid replay evidence

### Requirement: Replay indexes are catalog-searchable
r[molten.determinism.replay_index.catalog] The evidence ledger and catalog SHOULD classify replay indexes by artifact kind, decision, total count, pass count, deny count, raw receipt count, rollup count, divergence kinds, report refs, final-state refs, receipt refs, and rollup refs.

#### Scenario: Replay index is found by stage and final state
- GIVEN an imported replay index
- WHEN catalog search filters by `deterministic-replay-index`, `replay-index-decision`, and a final-state ref
- THEN the replay index is returned

### Requirement: Replay indexes have MCP readback
r[molten.determinism.replay_index.mcp] Replay evidence MCP search SHOULD return replay indexes through read-only search filters while preserving evidence-only semantics.

#### Scenario: Replay index MCP readback is evidence only
- GIVEN an imported replay index
- WHEN replay evidence MCP search filters by `stage=index`
- THEN the index is returned with read-only MCP receipt evidence
- AND the index does not replace individual replay verification, rollup, harness gate, policy, source-gate, release, provenance, transport, or authority checks

### Requirement: Replay index behavior is tested
r[molten.determinism.replay_index.tests] Molten SHOULD test replay index generation, stale input denial, catalog search, and replay MCP readback.

#### Scenario: Replay index validation covers denial and discovery
- GIVEN mixed replay evidence and a stale input case
- WHEN tests generate and import replay indexes
- THEN passing discovery and deny diagnostics are both covered

### Requirement: Release gates bind replay indexes
r[molten.release.replay_index_binding.gate] Molten SHOULD bind replay evidence index refs into dogfood release gate evidence while preserving evidence-only semantics.

#### Scenario: Release gate carries replay index refs
- GIVEN a passing local dogfood run with generated replay index evidence
- WHEN a release gate receipt is emitted
- THEN the release gate records at least one replay evidence index ref
- AND the release gate records that replay index evidence is evidence-only

### Requirement: Release readback denies stale replay indexes
r[molten.release.replay_index_binding.readback] Molten SHOULD deny release readback when replay index evidence is missing, malformed, stale, tampered, or not bound by the release gate.

#### Scenario: Tampered replay index denies Nix release verification
- GIVEN Nix dogfood evidence that references a replay index
- WHEN the replay index file is replaced with non-index content
- THEN release verification emits a deny receipt with replay index diagnostics

### Requirement: Release bundles carry replay index members
r[molten.release.replay_index_binding.bundle] Molten SHOULD include replay index Preserves members in release evidence bundles, signed-member checks, and release export member verification.

#### Scenario: Required signed members include replay index
- GIVEN release bundle verification with signed members required
- WHEN the replay index member lacks a valid signed receipt
- THEN bundle verification denies

### Requirement: Release replay bindings are discoverable
r[molten.release.replay_index_binding.catalog] The catalog SHOULD classify release artifacts that bind replay indexes with replay release-binding classifications and replay index refs.

#### Scenario: Release binding is found by replay index ref
- GIVEN an imported release artifact that binds a replay index
- WHEN replay evidence MCP search filters by `stage=release-binding` and replay index ref
- THEN the release binding artifact is returned

### Requirement: Release replay binding behavior is tested
r[molten.release.replay_index_binding.tests] Molten SHOULD test replay index emission, stale/tampered readback denial, signed bundle requirements, catalog/MCP discovery, and evidence-only checks.

#### Scenario: Replay index remains evidence only
- GIVEN release evidence with a valid replay index
- WHEN release readback passes
- THEN the replay index remains evidence only
- AND it does not replace source, policy, provenance, Octet, Cairn, signed keyring, authority, resource, transport, release promotion, or harness gate checks

### Requirement: Release outputs include raw replay verify evidence
r[molten.release.replay_verify_export.local_output] Molten SHOULD emit raw generic deterministic replay verify receipts from local dogfood release outputs alongside replay indexes.

#### Scenario: Local dogfood writes replay verify evidence
- GIVEN a passing local dogfood run
- WHEN the operator requests a replay verify output path
- THEN Molten writes a `deterministic-replay-verify-v1` receipt
- AND the receipt remains evidence-only release review material

### Requirement: Release readback binds replay verify refs
r[molten.release.replay_verify_export.readback] Molten SHOULD bind replay verify refs in Nix dogfood release evidence and verification receipts, and deny missing, stale, malformed, tampered, or index-mismatched replay verify evidence.

#### Scenario: Replay index must contain replay verify ref
- GIVEN Nix dogfood release evidence with a replay verify receipt and replay index
- WHEN readback validates the output path
- THEN the replay index must list the replay verify ref
- AND mismatches deny release readback

### Requirement: Release bundles include replay verify members
r[molten.release.replay_verify_export.bundle] Molten SHOULD include replay verify Preserves members in release bundles and signed-member verification.

#### Scenario: Required signed members include replay verify
- GIVEN release bundle verification with signed members required
- WHEN the replay verify member lacks a valid signed receipt
- THEN bundle verification denies

### Requirement: Release exports include replay verify members
r[molten.release.replay_verify_export.archive] Molten SHOULD include replay verify Preserves and signed replay verify members in release export manifests and archive verification.

#### Scenario: Export archive carries replay verify evidence
- GIVEN a passing release export
- WHEN the archive is inspected or verified
- THEN the archive contains replay verify Preserves and signed replay verify members
- AND tampered or missing replay verify members deny archive verification

### Requirement: Replay verify release export behavior is tested
r[molten.release.replay_verify_export.tests] Molten SHOULD test replay verify output, readback binding, signed bundle requirements, export membership, and evidence-only caveats.

#### Scenario: Replay verify remains evidence only
- GIVEN release evidence with replay verify and replay index refs
- WHEN release readback, bundle verification, and export verification pass
- THEN replay verify evidence remains evidence only
- AND it does not replace source, policy, provenance, Octet, Cairn, signed keyring, authority, resource, transport, retention, release promotion, or release acceptance checks
