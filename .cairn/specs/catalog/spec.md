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

### Requirement: Operator gateway readback core
r[molten.operator_gateway.readback_core] Molten MUST define a pure operator-gateway readback decision core that normalizes requested object refs, optional collection members, byte ranges, requester context, visibility policy refs, and supporting evidence refs before any HTTP, Iroh, filesystem, or response-streaming shell performs I/O.

#### Scenario: Readback request normalizes before I/O
- GIVEN an operator gateway request for a canonical artifact, receipt, bundle member, or chunk manifest range
- WHEN the readback decision core evaluates the request
- THEN it returns a pass, deny, or degraded decision with normalized refs, range, required checks, and diagnostics
- AND the imperative shell performs no response I/O until the decision is available.

#### Scenario: Malformed ref denies before lookup
- GIVEN an operator gateway request with a malformed or non-canonical object ref
- WHEN the readback decision core evaluates the request
- THEN it returns a deny decision with malformed-ref diagnostics
- AND catalog, chunk-store, Iroh, or filesystem lookup is skipped.

### Requirement: Read-only operator gateway index
r[molten.operator_gateway.readonly_index] Molten SHOULD provide read-only operator gateway indexes for visible artifact bundles, chunk collections, release evidence bundles, retention review bundles, and receipt sets without granting mutation authority.

#### Scenario: Visible bundle index is rendered
- GIVEN an operator gateway index request with policy-admitted visibility over a bundle
- WHEN Molten renders the index
- THEN it includes only visible member names, refs, sizes, and optional MIME hints
- AND the index receipt binds the request, visibility policy refs, response ref, and read-only checks.

#### Scenario: Hidden member is redacted or omitted
- GIVEN a bundle with a member hidden by confidentiality, retention, redaction, or visibility policy
- WHEN Molten renders the gateway index
- THEN the hidden member ref and sensitive name are omitted or redacted
- AND diagnostics record the omission without leaking the hidden ref.

### Requirement: Operator gateway receipts are evidence-only
r[molten.operator_gateway.receipts] Molten MUST emit canonical readback receipts for gateway read, range, and index operations, and MUST NOT treat those receipts as authority, policy admission, provenance trust, source-gate acceptance, retention clearance, execution permission, or mutation rights.

#### Scenario: Gateway receipt cannot authorize mutation
- GIVEN a passing gateway readback receipt
- WHEN a caller attempts to use it as evidence for delete, pin, unpin, install, execute, or policy mutation
- THEN the downstream gate denies unless the normal authority, policy, retention, provenance, source-gate, and resource evidence is supplied independently.

### Requirement: Replay verification binds first semantic divergence
r[molten.determinism.replay_first_divergence.verify_receipt] Replay verification receipts MUST bind the replay decision, divergence kind, expected comparison refs, actual comparison refs, and a first-divergence ref when replay denies.

#### Scenario: Effect response tamper denies with divergence ref
- GIVEN a deterministic replay fixture whose effect response ref differs from the recorded baseline
- WHEN replay verification evaluates the supplied fixture
- THEN the replay receipt decision is `deny`
- AND the divergence kind is `effect-response`
- AND the receipt records a non-empty first-divergence ref.

### Requirement: First-divergence debug records stay evidence-only
r[molten.determinism.replay_first_divergence.debug_record] Deterministic first-divergence records MUST identify the compared field, expected ref, actual ref, and safe diagnostics without replacing replay verification, authority, policy, provenance, resource, transport, source-gate, retention, release, or harness gate evidence.

#### Scenario: Debug record cannot pass a replay gate
- GIVEN a first-divergence debug record emitted for a denying replay
- WHEN a gate requires passing replay verification evidence
- THEN the debug record alone is insufficient
- AND the original replay verify receipt remains the source of the replay decision.

### Requirement: Replay uses recorded effects only
r[molten.determinism.replay_first_divergence.recorded_effects_only] Deterministic replay MUST deny attempts to satisfy replay by issuing live external effects when a required recorded effect response is absent.

#### Scenario: Missing recorded effect denies replay
- GIVEN a deterministic replay fixture missing a required recorded effect response
- WHEN replay verification evaluates the fixture
- THEN the replay receipt decision is `deny`
- AND diagnostics include recorded-effects-only replay semantics.

### Requirement: Replay fixture CLI emits pass and tamper-denial receipts
r[molten.determinism.replay_first_divergence.cli_fixture] The replay-fixture CLI SHOULD record deterministic fixtures, generate tampered fixture variants, verify supplied fixtures, and write replay verification receipts that expose pass or deny decisions plus first-divergence refs.

#### Scenario: CLI verifies tampered fixture denial
- GIVEN a replay fixture recorded by the CLI
- WHEN an operator generates an effect-response tampered fixture and verifies it with `--receipt-out`
- THEN the command succeeds with a `deny` replay decision
- AND the receipt file binds the first-divergence ref.

### Requirement: Replay fixture divergence behavior is tested
r[molten.determinism.replay_first_divergence.tests] Molten SHOULD test unchanged replay pass behavior, tampered replay denial for each supported divergence kind, missing-recorded-effect denial, and CLI receipt output for first-divergence refs.

#### Scenario: Tamper matrix covers divergence kinds
- GIVEN replay fixture tests generate one tampered fixture per supported semantic comparison class
- WHEN the tests verify each fixture
- THEN each case denies with the expected divergence kind
- AND each denial binds safe canonical first-divergence evidence.

### Requirement: Catalog provides linked semantic views
r[molten.catalog.share_like_linked_views] Molten MUST provide linked read-only catalog views over artifact refs, names, aliases, tags, channels, dependencies, dependents, schemas, effects, handler profiles, docs, transcripts, receipts, upgrade sessions, impact queries, and release snapshots.

#### Scenario: Artifact view links exact refs
- GIVEN an artifact has name metadata, dependency edges, schema refs, effect manifest refs, docs, and receipts
- WHEN a caller shows the artifact in the catalog
- THEN the view renders exact refs and links to related records
- AND names appear as metadata, not identity.

#### Scenario: Missing index is diagnostic only
- GIVEN a catalog index is stale or missing for a relation
- WHEN a caller asks for a linked view
- THEN Molten reports the missing or stale index
- AND does not invent dependency or trust facts from rendered text.

### Requirement: MCP catalog tools are read-only by default
r[molten.catalog.mcp_readonly_tools] Molten MUST expose MCP-style read-only tools for artifact search/show, dependency and dependent queries, receipt lookup, transcript lookup, impact queries, evidence explanation, and release snapshot inspection.

#### Scenario: Read-only dependency query succeeds
- GIVEN a caller invokes a dependency query tool with read authority
- WHEN the catalog has visible dependency edges
- THEN the tool returns structured results and redaction receipts.

#### Scenario: Mutation request through read-only tool denies
- GIVEN a caller invokes a read-only catalog MCP profile and asks it to update an alias or install an artifact
- WHEN the tool validates the request
- THEN it denies mutation
- AND points to the explicit gated subsystem path.

### Requirement: Catalog queries bind redaction decisions
r[molten.catalog.redaction_authorization] Molten MUST bind authorization and redaction decisions into catalog query receipts for private contents, sensitive policy outcomes, secret refs, capabilities, retention-sensitive records, and denied evidence details.

#### Scenario: Authorized private view shows content
- GIVEN a caller has admitted read authority for a private artifact
- WHEN the catalog renders that artifact
- THEN the query receipt records the authority evidence
- AND the view may include the authorized private fields.

#### Scenario: Public view redacts sensitive content
- GIVEN a public caller searches artifacts and a matching record contains secret refs or private capability details
- WHEN the catalog renders results
- THEN sensitive fields are redacted
- AND the query receipt records the redaction reason.

### Requirement: Catalog output grants no mutation authority
r[molten.catalog.no_catalog_mutation_authority] Molten MUST treat catalog and MCP discovery output as explanation evidence only; it MUST NOT grant install, alias update, policy change, capability, storage mutation, retention, release, transport, or execution authority.

#### Scenario: Catalog result supports operator decision only
- GIVEN a catalog query returns a candidate artifact ref
- WHEN an operator chooses to execute it
- THEN execution still requires the normal artifact, capability, policy, provenance, effect, resource, and source-gate admissions.

#### Scenario: Catalog receipt cannot authorize deletion
- GIVEN a catalog impact query lists no visible dependents
- WHEN a destructive retention operation is requested
- THEN the retention gate still requires retention and dependency impact evidence
- AND the catalog query alone is insufficient authority.

### Requirement: Catalog discovery validation covers positive and negative paths
r[molten.catalog.unison_discovery_validation] Molten MUST include positive and negative fixtures for linked views, read-only queries, redaction, private content denial, mutation attempts, stale indexes, and Unison Share or UCM API compatibility denial.

#### Scenario: Linked view fixture passes
- GIVEN visible artifacts, edges, transcripts, and receipts
- WHEN validation runs
- THEN the catalog returns stable linked refs with query receipt evidence.

#### Scenario: Unison API compatibility claim denies
- GIVEN a catalog endpoint claims compatibility with Unison Share or UCM APIs
- WHEN validation checks the non-claim boundary
- THEN it denies the claim
- AND records that those systems are prior art only.

### Requirement: Replay coverage matrices are catalog-searchable
r[molten.catalog.replay_coverage.matrix_search] The catalog SHOULD classify replay coverage matrices by artifact kind, decision, subsystem names, workflow names, replay eligibility classes, missing-evidence diagnostics, and referenced replay index refs.

#### Scenario: Matrix is found by subsystem
- GIVEN an imported replay coverage matrix with a node-control row
- WHEN catalog search filters by `replay-coverage-subsystem:node-control`
- THEN the matrix evidence is returned without granting replay pass authority.

### Requirement: Replay coverage readback is read-only
r[molten.catalog.replay_coverage.readonly] Replay coverage MCP or catalog readback MUST remain read-only and MUST NOT replace replay verification, subsystem gates, source gates, policy, provenance, authority, transport, release, or retention checks.

#### Scenario: MCP readback binds request only
- GIVEN an MCP request searches replay coverage matrices
- WHEN the request succeeds
- THEN the response receipt binds the read-only request and response
- AND mutating catalog tools remain denied.

### Requirement: Release readback denies stale replay identity
r[molten.release.replay_freshness.readback] Release and dogfood readback SHOULD deny replay verify or replay index evidence whose run identity does not match the release or dogfood subject identity it claims to cover.

#### Scenario: Changed artifact ref denies release readback
- GIVEN release evidence with a replay index recorded for a different artifact ref
- WHEN release readback validates replay freshness
- THEN readback emits a deny receipt
- AND diagnostics identify the stale artifact component.

#### Scenario: Missing identity denies readback
- GIVEN release evidence with a replay verify receipt that lacks required run identity binding
- WHEN release readback validates replay freshness
- THEN readback denies before accepting the replay evidence as release review material
- AND diagnostics identify the missing identity field.

### Requirement: Replay identity is catalog-searchable
r[molten.catalog.replay_freshness.identity_search] The catalog SHOULD classify replay verification receipts, replay indexes, and release replay bindings by run identity ref, artifact ref, handler profile ref, policy refs, replay profile, freshness decision, and stale-component diagnostics when present.

#### Scenario: Search finds replay evidence by identity
- GIVEN imported replay evidence with a run identity ref
- WHEN catalog search filters by that run identity ref
- THEN matching replay evidence is returned as read-only discovery evidence.

### Requirement: Replay freshness readback remains evidence-only
r[molten.catalog.replay_freshness.evidence_only] Replay freshness receipts and catalog search results MUST NOT grant authority, policy admission, provenance trust, source-gate acceptance, release promotion, transport trust, resource rights, retention authority, or execution trust.

#### Scenario: Fresh replay does not replace source gate
- GIVEN replay freshness validation passes for a release subject
- WHEN release promotion evaluates source-gate requirements
- THEN the fresh replay evidence remains insufficient by itself
- AND source-gate evidence is still required separately.
