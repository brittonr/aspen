# Artifact-auth adoption

## ADDED Requirements

### Requirement: Radicle-backed source transport r[molten.artifact_auth_adoption.radicle_transport]

Molten MUST consume the accepted public artifact-auth RID through the upload-pack-only Radicle HTTPS adapter at exact revision `799459346d5416fbd7b9f55840a7371441b55afa`.

#### Scenario: Exact public source is admitted

- GIVEN an accepted publication receipt, RID, URL, commit, and source archive identity
- WHEN Molten evaluates dependency source admission
- THEN all identities MUST agree before the source is used.

#### Scenario: Unaccepted source is rejected

- GIVEN a different RID, URL, commit, visibility, or archive identity
- WHEN source admission runs
- THEN the dependency MUST be rejected.

### Requirement: Cargo, Nix, policy, and build-plan agreement r[molten.artifact_auth_adoption.radicle_agreement]

Cargo manifests, `Cargo.lock`, `flake.nix`, `flake.lock`, the release-dependency profile, and generated unit2nix plans MUST identify the same Radicle HTTPS repository and reviewed Git revision while preserving the locked Nix content identity and package graph.

#### Scenario: Owning tools preserve identity

- GIVEN regenerated Cargo and Nix locks plus both pinned unit2nix plans
- WHEN agreement validation runs
- THEN both artifact-auth packages, the exact revision, public URL, expected Nix content hash, policy rows, and plan source records MUST match.

#### Scenario: Generated identity drift fails closed

- GIVEN a stale GitHub source, mismatched revision, duplicate or missing package, changed Nix content hash, or stale build plan
- WHEN agreement validation runs
- THEN validation MUST fail.

### Requirement: Behavioral parity checks r[molten.artifact_auth_adoption.radicle_behavior]

The transport cutover MUST preserve existing Molten and artifact-auth behavior under focused core, shell, operational receipt, replay, formatting, and source-admission checks.

#### Scenario: Existing positive and negative behavior passes

- GIVEN the Radicle-backed source at the reviewed commit
- WHEN focused identity suites run
- THEN existing acceptance and rejection behavior MUST pass without Rust implementation changes.

#### Scenario: Behavioral drift blocks acceptance

- GIVEN any changed fixture result, feature graph, package version, or authority boundary
- WHEN parity is evaluated
- THEN the cutover MUST NOT be accepted.

### Requirement: No executable GitHub fallback r[molten.artifact_auth_adoption.radicle_fallback]

Molten MUST NOT retain an executable GitHub source fallback for artifact-auth.

#### Scenario: Radicle source is unavailable

- GIVEN the admitted Radicle HTTPS source cannot serve the exact object
- WHEN dependency resolution runs
- THEN resolution MUST fail visibly rather than selecting GitHub.

#### Scenario: Historical documentation remains bounded

- GIVEN documentation or immutable package metadata names historical GitHub provenance
- WHEN fallback validation runs
- THEN those records MAY remain while executable manifests, locks, policies, and generated plans contain no artifact-auth GitHub source.

### Requirement: Typed Radicle cutover evidence r[molten.artifact_auth_adoption.radicle_evidence]

Molten MUST emit typed Nickel/JSON evidence with a BLAKE3 sidecar binding publication, source, locks, policy, build plans, test observations, rollback boundary, and non-claims.

#### Scenario: Complete cutover evidence passes

- GIVEN matching publication, source, lock, policy, plan, and test observations
- WHEN receipt validation runs
- THEN it MUST accept deterministically.

#### Scenario: Missing linkage or overclaim fails

- GIVEN missing publication linkage, drifted generated identity, missing negative evidence, or weakened non-claims
- WHEN receipt validation runs
- THEN it MUST fail closed.
