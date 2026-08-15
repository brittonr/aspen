# Artifact Registry Delta: Canonical Artifact IDs

## ADDED Requirements

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