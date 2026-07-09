# Artifact Registry Delta: Claim Authority Evidence Readback

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
