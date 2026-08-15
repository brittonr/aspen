# Artifact Registry Delta: Evidence, Ledger, and Registry Boundaries

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
