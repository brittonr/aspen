# Artifact Registry Specification

## Purpose

Defines Molten artifact registry and local evidence ledger requirements.

## Requirements

### Requirement: Content is immutable by canonical hash
r[molten.artifacts.local_evidence_ledger.content_table] The local evidence ledger MUST store canonical artifact bytes immutably by their Preserves hash.

#### Scenario: Duplicate import is idempotent
- GIVEN an artifact already present in the ledger
- WHEN the same canonical bytes are imported again
- THEN the ledger returns the same content ref
- AND no duplicate content record is required

#### Scenario: Hash mismatch is rejected
- GIVEN bytes presented under a claimed content ref
- WHEN the bytes hash to a different ref
- THEN import fails closed

### Requirement: Indexes are derived and rebuildable
r[molten.artifacts.local_evidence_ledger.indexes] Ledger indexes MUST be derivable from stored canonical artifacts and validation receipts.

#### Scenario: Rebuild preserves query results
- GIVEN a ledger with reports, receipts, bundles, and failures
- WHEN indexes are dropped and rebuilt from content records
- THEN listing by report ref, suite ref, bundle ref, and receipt kind returns the same artifacts

### Requirement: Retention pins protect dependencies
r[molten.artifacts.local_evidence_ledger.retention_gc] GC MUST preserve every artifact reachable from a retained pin or retained receipt dependency.

#### Scenario: Pinned bundle preserves embedded report and receipts
- GIVEN a pinned sealed repro bundle
- WHEN GC runs
- THEN the bundle, embedded report, suite, gate receipt, redaction evidence, and verify receipts remain available

#### Scenario: Unpinned diagnostic failure can be collected
- GIVEN an unpinned failure artifact with no retained dependencies
- WHEN GC runs with policy allowing diagnostic cleanup
- THEN the failure artifact may be removed
- AND the GC receipt records the removed refs

### Requirement: Ledger import and export preserve canonical evidence
r[molten.artifacts.local_evidence_ledger.import_export] The local evidence ledger MUST provide import and export operations for canonical report, bundle, unpack directory, and receipt artifacts without changing their content refs.

#### Scenario: Exported bytes match imported bytes
- GIVEN a canonical artifact imported into the ledger
- WHEN the artifact is exported back to a file
- THEN the exported file bytes hash to the same content ref
- AND the ledger records import and export evidence for the operation

### Requirement: Ledger validation appends receipts
r[molten.artifacts.local_evidence_ledger.validation_receipts] Ledger validation MUST append validation, import, export, pin, and GC receipts instead of mutating stored artifact bytes or overwriting prior status.

#### Scenario: Validation rule changes append new evidence
- GIVEN an artifact that already has a validation receipt
- WHEN validation is run again under a newer rule set
- THEN the ledger stores a new validation receipt
- AND the original artifact bytes and prior receipt remain available by content ref

### Requirement: Ledger behavior has regression coverage
r[molten.artifacts.local_evidence_ledger.tests] The local evidence ledger SHOULD have regression tests for immutability, rebuildable indexes, corrupted bytes, missing dependencies, and retained dependency preservation.

#### Scenario: Corrupted storage is detected
- GIVEN ledger storage whose bytes no longer match the recorded content ref
- WHEN indexes are rebuilt or the artifact is read
- THEN the corruption is reported as a validation failure
- AND retained dependencies are not silently dropped

### Requirement: Chain links are immutable ledger artifacts
r[molten.evidence.chain_hashing.ledger_index] The local evidence ledger MUST store chain links as immutable canonical artifacts and derive chain indexes from stored link bytes and linked payload artifacts.

#### Scenario: Rebuilding indexes preserves chain heads
r[molten.evidence.chain_hashing.ledger_index.rebuild]
- GIVEN a ledger containing chain links, payload artifacts, append receipts, and checkpoints
- WHEN derived indexes are dropped and rebuilt from canonical content
- THEN chain scope/id/epoch listings, parent/child relationships, sequence lookups, payload lookups, anchors, checkpoints, and heads are reconstructed

#### Scenario: Indexed head is not authoritative without link bytes
r[molten.evidence.chain_hashing.ledger_index.head_requires_content]
- GIVEN a derived index entry claiming a chain head
- WHEN the corresponding canonical chain-link artifact is missing or hashes differently
- THEN the ledger rejects the head until the canonical link bytes are available and verified

### Requirement: Append receipts record head transitions
r[molten.evidence.chain_hashing.append_receipts] Ledger chain appends MUST emit canonical append receipts that bind head-before, head-after, appended link ref, payload ref, and continuity checks.

#### Scenario: Idempotent append of existing head
r[molten.evidence.chain_hashing.append_receipts.idempotent]
- GIVEN a chain head already points to a link ref
- WHEN the same canonical link is appended again with the same head-before and head-after
- THEN append is idempotent
- AND the append receipt names the existing link ref

#### Scenario: Unexpected stale head is denied
r[molten.evidence.chain_hashing.append_receipts.stale]
- GIVEN a chain head has advanced since a caller last observed it
- WHEN the caller appends a link against the stale head without an admitted fork or historical policy
- THEN append fails closed
- AND a denial receipt names the stale observed head and current head

### Requirement: Checkpoints are explicit artifacts
r[molten.evidence.chain_hashing.control_plane_checkpoints] Accepted control-plane chain heads SHOULD be represented by canonical checkpoint artifacts or receipts that name chain scope/id/epoch, prior checkpoint ref, new head ref, verified range, and policy/membership refs.

#### Scenario: Checkpoint descends from prior checkpoint
r[molten.evidence.chain_hashing.control_plane_checkpoints.descends]
- GIVEN a prior accepted checkpoint for a chain scope/id/epoch
- WHEN a new checkpoint is proposed
- THEN verification confirms the new head descends from the prior checkpoint head or explicitly records an admitted reconfiguration/epoch change

### Requirement: GC preserves anchored chains
r[molten.evidence.chain_hashing.anchor_policy] Retention and GC MUST preserve chain links and payload artifacts reachable from retained anchors, heads, checkpoints, or signed append/verify receipts.

#### Scenario: Pinned checkpoint preserves segment
r[molten.evidence.chain_hashing.anchor_policy.gc]
- GIVEN a retained checkpoint naming a chain head
- WHEN GC runs
- THEN the checkpoint, verified segment to the retained anchor, append/verify receipts, and required payload artifacts remain available

### Requirement: Evidence chain head transitions are continuous
r[molten.evidence_chain_state_machine_proof.head_transition_continuity] Molten MUST prove that evidence-chain append operations advance from head-before to head-after only when the appended link, payload ref, predicate receipt ref, and continuity checks are canonical and consistent.

#### Scenario: Valid append advances one head
- GIVEN a chain head and a canonical append link whose prior head matches the observed head
- WHEN Molten appends the link
- THEN the append receipt binds head-before, head-after, appended link ref, payload ref, and predicate receipt ref
- AND the resulting head equals the appended link ref.

### Requirement: Evidence chain gaps and forks deny
r[molten.evidence_chain_state_machine_proof.gap_fork_denial] Molten MUST prove that chain verification denies missing intermediate links, stale observed heads, forked heads, duplicate sequence conflicts, and tampered payload refs before accepting a chain segment as continuous evidence.

#### Scenario: Forked head denies verification
- GIVEN two append links that claim the same prior head for the same chain scope and epoch
- WHEN Molten verifies the chain segment
- THEN verification emits a denial receipt
- AND diagnostics identify the fork or duplicate head transition.

### Requirement: Evidence chain checkpoints and anchors preserve reachable evidence
r[molten.evidence_chain_state_machine_proof.checkpoint_anchor_preservation] Molten MUST prove checkpoints, retained heads, anchors, and signed append or verify receipts preserve every reachable chain link and payload artifact required to validate the retained chain segment.

#### Scenario: Retained checkpoint protects chain segment
- GIVEN a retained checkpoint for a verified chain segment
- WHEN retention or garbage collection evaluates reachable evidence
- THEN the checkpoint, verified links, payload artifacts, append receipts, and verify receipts remain available
- AND unanchored unrelated artifacts may still be removed according to retention policy.

### Requirement: Evidence, ledger, and registry ownership is explicit
r[molten.artifact_registry.modularity.layer_ownership] Evidence construction and verification, ledger persistence, and registry or catalog discovery SHOULD be owned by separate reviewable boundaries.

#### Scenario: Layer responsibility is clear
- GIVEN code involving evidence artifacts, local ledger storage, and catalog discovery is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility as evidence, ledger, registry, catalog, or shell

### Requirement: Evidence verification can run without storage
r[molten.artifact_registry.modularity.evidence_without_storage] Evidence constructors, parsers, and verifiers SHOULD be callable over in-memory canonical values without requiring local ledger persistence or registry discovery.

#### Scenario: In-memory evidence parses
- GIVEN a valid canonical evidence value represented in memory
- WHEN the evidence parser or verifier evaluates it
- THEN it returns typed evidence data or pass diagnostics without reading the local ledger

#### Scenario: Stale chain denies before registry promotion
- GIVEN an evidence chain value with a stale link, missing predicate, wrong checkpoint, or malformed payload ref
- WHEN evidence verification evaluates it
- THEN verification denies before ledger storage or registry discovery can promote it as trusted evidence

### Requirement: Discovery remains non-authoritative
r[molten.artifact_registry.modularity.discovery_non_authority] Ledger presence, registry classification, catalog search, or MCP discovery MUST NOT grant authority, provenance, policy, retention, source-gate, execution, or replay trust by itself.

#### Scenario: Registry-only discovery is not admission
- GIVEN an artifact is discoverable through the registry or catalog
- WHEN a trust-boundary operation evaluates the artifact
- THEN discovery alone is insufficient without the required evidence and admission refs

### Requirement: Evidence/ledger/registry boundaries have positive and negative tests
r[molten.artifact_registry.modularity.tests] Boundary refactors SHOULD include positive tests for valid stored and discovered evidence and negative tests for malformed stored artifacts, registry-only discovery, stale chain links, or missing predicate receipts.

#### Scenario: Stored malformed artifact does not become valid evidence
- GIVEN a malformed artifact is present in local storage
- WHEN registry discovery lists it
- THEN downstream evidence verification still denies the malformed artifact before promotion

### Requirement: Canonical artifact identity receipts
r[molten.artifacts.canonical_id_receipts] Molten MUST emit canonical artifact identity receipts that bind artifact kind, identity domain, canonical payload ref, schema refs, dependency summary refs, policy refs, provenance refs, supported hash algorithm, and identity checks.

#### Scenario: Repeated canonicalization is stable
- GIVEN the same artifact payload, artifact kind, canonicalizer version, schema refs, and dependency summary refs
- WHEN Molten derives artifact identity twice
- THEN both derivations produce the same artifact ref
- AND the identity receipt records the same canonical payload ref and checks.

#### Scenario: Identity receipt rejects missing payload ref
- GIVEN an artifact identity request omits the canonical payload ref
- WHEN Molten validates the identity receipt input
- THEN identity derivation denies before install or use
- AND diagnostics name the missing canonical payload boundary.

### Requirement: Normalized payload boundary
r[molten.artifacts.normalized_payload_boundary] Molten MUST derive artifact ids from reviewed canonical artifact representations when such representations exist, rather than from mutable names, file paths, raw source text, or rendered diagnostics.

#### Scenario: Reviewed canonical form is used
- GIVEN a supported Preserves schema, Nickel contract, Steel predicate, Trellis projection, transcript, or Wasm component artifact
- WHEN Molten installs the artifact
- THEN it normalizes the artifact into the reviewed canonical representation before hashing
- AND the install receipt binds the canonical payload ref.

#### Scenario: Raw source hash cannot satisfy executable identity
- GIVEN a caller presents only a raw source-text hash for an artifact kind with a reviewed canonicalizer
- WHEN Molten evaluates install or use admission
- THEN it denies the executable or policy-bearing role
- AND reports that raw source text is not authoritative identity.

### Requirement: Artifact identity domains are separated
r[molten.artifacts.domain_separated_identity] Molten MUST use explicit artifact-kind identity domains so byte-identical payloads in different artifact roles cannot collide semantically.

#### Scenario: Identical bytes in different domains stay distinct
- GIVEN identical canonical bytes are classified as a schema artifact and as a policy artifact
- WHEN Molten derives artifact refs
- THEN the refs differ by domain
- AND each receipt records the artifact-kind domain used for hashing.

#### Scenario: Wrong-domain substitution denies
- GIVEN a dependency requires a schema artifact ref
- WHEN a caller supplies a policy artifact ref with identical payload bytes
- THEN Molten denies substitution unless explicit compatibility evidence is admitted.

### Requirement: Non-canonical install attempts fail closed
r[molten.artifacts.install_rejects_noncanonical] Molten MUST reject artifact install or use attempts that rely on mutable names, raw source text, rendered logs, unsupported hash algorithms, or missing canonical payload refs as identity.

#### Scenario: Exact canonical artifact installs
- GIVEN an artifact has a supported canonical payload, BLAKE3 domain, dependency summary, and required evidence refs
- WHEN install admission evaluates the artifact
- THEN Molten emits a passing identity receipt before downstream policy and capability gates run.

#### Scenario: Unsupported hash algorithm denies
- GIVEN an artifact identity claim uses an unsupported hash algorithm for a Molten-owned artifact ref
- WHEN Molten validates the claim
- THEN it denies before registry mutation
- AND diagnostics state that Molten-owned identity requires BLAKE3 unless an explicit interop contract applies.

### Requirement: Canonical identity validation covers positive and negative paths
r[molten.artifacts.canonical_identity_validation] Molten MUST include positive and negative validation fixtures for stable ids, repeated normalization, wrong domains, canonicalizer drift, unsupported kinds, raw-source-only identity, and tampered canonical bytes.

#### Scenario: Positive fixture proves stable canonical identity
- GIVEN a fixture with canonical bytes and expected artifact ref
- WHEN validation runs
- THEN the fixture passes by recomputing the expected ref.

#### Scenario: Negative fixture proves tamper denial
- GIVEN a fixture whose canonical bytes no longer match the expected artifact ref
- WHEN validation runs
- THEN validation emits deny evidence
- AND no install receipt is accepted as passing identity evidence.

### Requirement: Dependency edges are canonical records
r[molten.artifacts.dependency_edge_records] Molten MUST model direct dependency facts as canonical records that bind source ref, target ref, relation, requiredness, scope, and evidence refs for artifact, schema, policy, effect, capability, handler profile, storage, transcript, migration, and release relationships.

#### Scenario: Direct schema dependency is recorded
- GIVEN an artifact declares that its input payload uses schema ref S
- WHEN Molten admits the artifact metadata
- THEN it records a direct dependency edge from the artifact ref to schema ref S
- AND the edge has a stable relation and evidence refs.

#### Scenario: Opaque inferred dependency is not authoritative
- GIVEN a source file text mentions a schema name but no canonical dependency edge exists
- WHEN Molten computes normative impact analysis
- THEN the text mention is diagnostic only
- AND mutation gates require a canonical edge or explicit evidence explaining the omission.

### Requirement: Reverse dependency indexes are rebuildable
r[molten.artifacts.reverse_dependency_index] Molten MUST maintain reverse dependency indexes that can be deterministically rebuilt from canonical registry and ledger dependency-edge records.

#### Scenario: Rebuild produces stable reverse index
- GIVEN the same sorted dependency-edge records and redaction policy
- WHEN Molten rebuilds the reverse index twice
- THEN both rebuilds produce identical reverse index digests
- AND the rebuild receipt binds the source edge set.

#### Scenario: Stale reverse index denies normative impact gate
- GIVEN a reverse index digest does not match the current edge set
- WHEN an upgrade, retention, or release gate requires impact evidence
- THEN Molten denies use of the stale index
- AND diagnostics require rebuild or diagnostic-only treatment.

### Requirement: Impact queries emit planning receipts
r[molten.artifacts.impact_query_receipts] Molten MUST emit impact query receipts that bind query subject, relation filters, direct dependents, requested transitive dependents, redaction decisions, index refs, and diagnostics.

#### Scenario: Upgrade planning receives dependents
- GIVEN a schema ref is targeted for migration
- WHEN Molten computes an impact query
- THEN the receipt lists direct and requested transitive dependents
- AND the upgrade session can bind that receipt as planning evidence.

#### Scenario: Redacted dependency is not leaked
- GIVEN an impact query runs under a public catalog profile and a dependent ref is private
- WHEN Molten renders the query result
- THEN the receipt records a redaction decision
- AND the public view does not reveal the hidden target.

### Requirement: Index rebuilds are deterministic
r[molten.artifacts.index_rebuild_determinism] Molten MUST sort and canonicalize dependency edges, duplicate handling, cycle diagnostics, and reverse-index entries so the same registry and ledger inputs produce the same index digest.

#### Scenario: Duplicate edges are normalized
- GIVEN two equivalent dependency-edge records are present
- WHEN the reverse index is rebuilt
- THEN Molten records duplicate diagnostics
- AND the canonical index output is deterministic.

#### Scenario: Cycle diagnostics are stable
- GIVEN artifacts depend on each other cyclically
- WHEN impact analysis computes transitive dependents
- THEN traversal terminates with stable cycle diagnostics
- AND no unbounded walk or nondeterministic order is used.

### Requirement: Dependency index validation covers positive and negative paths
r[molten.artifacts.dependency_index_validation] Molten MUST include positive and negative fixtures for complete graphs, missing edges, duplicate edges, cycles, stale indexes, and unauthorized hidden dependency exposure.

#### Scenario: Complete graph fixture passes
- GIVEN a fixture with declared artifacts, edges, and expected reverse dependents
- WHEN validation runs
- THEN Molten recomputes the expected index digest and passes.

#### Scenario: Missing edge fixture denies
- GIVEN a fixture omits a required dependency edge for an artifact that declares the target in canonical metadata
- WHEN validation runs
- THEN Molten emits deny diagnostics
- AND impact evidence from that graph cannot satisfy mutation gates.

### Requirement: Name views are canonical metadata records
r[molten.artifacts.name_view_records] Molten MUST model names, aliases, tags, and channels as canonical metadata view records that point to immutable artifact refs or artifact-set refs and bind scope, issuer, policy refs, evidence refs, previous-view refs, and tombstones.

#### Scenario: Name update preserves artifact identity
- GIVEN a name view points from `policy/main` to artifact ref A
- WHEN an authorized update points `policy/main` to artifact ref B
- THEN Molten emits a new view receipt
- AND artifact refs A and B remain immutable and addressable.

#### Scenario: Unauthorized view update denies
- GIVEN a caller lacks the capability or policy evidence required to update a name view
- WHEN it submits a pointer update
- THEN Molten denies the update before mutating metadata
- AND records that no artifact identity changed.

### Requirement: Normative uses pin exact refs
r[molten.artifacts.exact_ref_pinning] Molten MUST require dependencies, transcript expectations, remote execution requests, migration recipes, storage type bindings, and policy admissions to record exact artifact refs after any name resolution.

#### Scenario: Transcript records resolved artifact ref
- GIVEN a transcript stanza refers to a human-readable artifact name
- WHEN Molten admits the transcript for replayable execution
- THEN the transcript or admission receipt records the exact resolved artifact ref
- AND future replay does not depend on mutable name lookup.

#### Scenario: Name-only execution request denies
- GIVEN a remote execution request names an entrypoint by mutable name only
- WHEN Molten evaluates execution admission
- THEN it denies until the request carries an exact artifact ref or admitted resolution receipt.

### Requirement: Ambiguous name resolution fails closed
r[molten.artifacts.name_ambiguity_denial] Molten MUST deny normative name resolution when multiple candidates match and no admitted scope or channel policy selects exactly one target.

#### Scenario: Scoped resolution selects one target
- GIVEN two artifacts share a display name in different scopes
- WHEN a request includes an admitted scope that selects one candidate
- THEN Molten resolves to that exact artifact ref
- AND records the scope decision in diagnostics.

#### Scenario: Ambiguous display name denies
- GIVEN a display name matches multiple candidate artifact refs and no scope policy disambiguates them
- WHEN the name is used for install, execution, migration, storage, policy, or release admission
- THEN Molten denies before side effects
- AND diagnostics list the candidate refs.

### Requirement: Name views are non-authority
r[molten.artifacts.name_views_non_authority] Molten MUST treat name views, aliases, tags, and channels as discovery metadata only; they MUST NOT grant capability, provenance, policy trust, source-gate trust, retention rights, transport trust, or execution authority.

#### Scenario: Name assists discovery only
- GIVEN a catalog query finds an artifact by name
- WHEN the operator requests details
- THEN Molten may render the name and exact ref together
- AND any subsequent use still requires normal admission gates.

#### Scenario: Trusted-looking name does not bypass gates
- GIVEN an artifact is named `trusted/release`
- WHEN a caller attempts execution without required provenance or policy evidence
- THEN Molten denies execution
- AND reports that the name has no trust authority.

### Requirement: Name view validation covers positive and negative paths
r[molten.artifacts.name_view_validation] Molten MUST include positive and negative fixtures for deterministic name resolution, exact ref pinning, ambiguous names, stale channels, unauthorized pointer updates, and name-only execution denial.

#### Scenario: Valid pointer fixture passes
- GIVEN an authorized name view update and exact target artifact ref
- WHEN validation runs
- THEN the view receipt verifies and the target ref is unchanged.

#### Scenario: Stale channel fixture denies
- GIVEN a release channel view has a freshness or revocation policy that is no longer satisfied
- WHEN validation runs for normative use
- THEN Molten denies resolution
- AND emits stale-view diagnostics.

### Requirement: Claim subject selectors are canonical and hash-agnostic
r[molten.claim_authority.subject_selectors] Molten MUST define canonical claim-domain or subject-selector records that can name exact refs, ref prefixes, artifact classes, namespaces, schema ids, release channels, cluster ids, or policy-defined subject sets without assuming one content hash algorithm.

#### Scenario: Selector names a content-ref domain without hash-specific authority
- GIVEN a subject selector names a set of content refs
- WHEN claim authority admission evaluates the selector
- THEN the selector is treated as the resource being authorized
- AND the hash algorithm used by an individual subject ref does not grant authority by itself.

#### Scenario: Broad selector requires visible attenuation
- GIVEN a subject selector uses a wildcard, namespace, prefix, or policy-defined subject set
- WHEN a capability token authorizes claims for that selector
- THEN admission requires attenuation, caveat, policy, or resource evidence that makes the broad authority explicit.

### Requirement: Claim artifacts remain evidence candidates until admitted
r[molten.claim_authority.registry_readback] The ledger, registry, catalog, and MCP readback surfaces MAY classify and discover claim selectors, authority claims, and claim admission receipts, but discovery MUST NOT admit claims or satisfy downstream trust-boundary gates by itself.

#### Scenario: Registry-only claim does not pass
- GIVEN an `authority-claim-v1` artifact is stored in the ledger and visible through catalog search
- WHEN a downstream gate requires an admitted external claim
- THEN the gate denies unless a matching passing `authority-claim-admission-v1` receipt and its linked capability/UCAN/Basalt receipts are supplied.

#### Scenario: Malformed claim cannot be promoted by discovery
- GIVEN a malformed or stale authority claim artifact is discoverable in the registry
- WHEN claim admission evaluates it
- THEN admission denies before registry classification or catalog visibility can promote it as trusted evidence.

### Requirement: Claim readback is evidence-only
r[molten.claim_authority.readback_non_authority] Claim summaries, search results, diagnostics, and MCP responses MUST render claim kind, subject selector, issuer, decision, and linked proof refs without granting authority, provenance, source-gate, retention, execution, release, deployment, transport, or policy trust.

#### Scenario: Catalog summary cannot authorize import
- GIVEN a catalog summary shows a passing claim admission for a subject
- WHEN an artifact import or install gate evaluates that subject
- THEN the gate still requires the exact policy-selected claim admission and all normal provenance, authority, policy, resource, and source-gate inputs.

### Requirement: Claim registry behavior has positive and negative tests
r[molten.claim_authority.registry_tests] Registry and catalog coverage SHOULD include positive readback for selectors, claims, and passing admissions, plus negative tests proving registry-only, malformed, stale, missing-proof, wrong-kind, and discovery-as-authority cases deny.

#### Scenario: Discovery-as-authority fixture denies
- GIVEN a claim is visible through ledger, catalog, and MCP readback
- AND the matching capability admission receipt is absent
- WHEN a downstream gate attempts to use the catalog result as authority
- THEN the gate denies with a missing claim-admission diagnostic.
