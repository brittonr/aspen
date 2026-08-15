# Artifact Auth Adoption Specification

## Purpose

Defines the `artifact-auth-adoption` capability.

## Requirements

### Requirement: Molten adopts one immutable reviewed source

r[molten.artifact_auth_adoption.source] Molten MUST consume its authentication and binding packages from one immutable reviewed `OnixResearch/onix-artifact` revision. Cargo, Nix, policy, plan, workspace, license, and consumer identities MUST agree.

#### Scenario: Unified source identity is admissible

- GIVEN Cargo and Nix resolve revision `c932138d880ddf4c2967f4c024b489b5c0022bf1`
- WHEN Molten evaluates dependency admission
- THEN it SHALL accept the two authentication packages and binding package while rejecting floating, duplicate, mixed, local, widened, or incompatible sources.

### Requirement: Molten retains runtime and authority semantics

r[molten.artifact_auth_adoption.authority] Molten MUST retain entropy, key generation/storage/signing, opaque handles, rotation writes, capability and federation authority, Preserves/Iroh transport, runtime policy, and evidence composition while treating standalone authentication as one bounded input.

#### Scenario: Authentication passes without runtime authority

- GIVEN a standalone signature and policy decision passes
- WHEN membership, capability, transport, runtime, deployment, or release admission runs
- THEN Molten MUST still require its product-owned checks and MUST NOT promote standalone success into product authority.

### Requirement: Cutover requires explained dual-run evidence

r[molten.artifact_auth_adoption.cutover] Molten MUST dual-run legacy and standalone paths over identical observations, classify every preimage, identity, decision, issue, and non-claim difference, reject unrelated-failure false parity, and preserve a bounded legacy rollback until standalone authority is explicitly admitted.

#### Scenario: Unexplained drift blocks cutover

- GIVEN any unexplained compatibility, currentness, or source-identity difference
- WHEN Molten evaluates cutover admission
- THEN the legacy path SHALL remain authoritative and the exact blocker SHALL be recorded without weakening runtime or authority gates.

### Requirement: Unified Artifact workspace source transport
r[molten.artifact_auth_adoption.radicle_transport]

Molten MUST consume the three packages from the immutable unified Artifact repository. It MUST retain the predecessor Radicle identity as historical evidence only.

#### Scenario: Exact unified source is admitted

- GIVEN the repository, revision, Nix NAR hash, workspace members, and source entry identities agree
- WHEN Molten evaluates the dependency source
- THEN it MAY use the source without admitting transfer mechanics or Artifact-owned authority.

#### Scenario: Mixed or widened source is rejected

- GIVEN a different repository, revision, NAR hash, package set, source identity, or executable predecessor dependency
- WHEN source admission runs
- THEN the dependency MUST be rejected.

### Requirement: Cargo, Nix, policy, and build-plan agreement
r[molten.artifact_auth_adoption.radicle_agreement]

Cargo manifests, `Cargo.lock`, `flake.nix`, `flake.lock`, the release profile, and both unit2nix plans MUST identify one Artifact repository and revision. The consumer graph MUST include only authentication and binding packages from that source.

#### Scenario: Owning tools preserve identity

- GIVEN Cargo, Nix, and unit2nix regenerated their artifacts
- WHEN agreement validation runs
- THEN all three packages, the revision, repository, Nix content identity, policy rows, and plan sources MUST match.

#### Scenario: Generated identity drift fails closed

- GIVEN a predecessor, mixed, local, mismatched, duplicate, missing, widened, or stale generated source
- WHEN agreement validation runs
- THEN validation MUST fail.

### Requirement: Behavioral parity checks
r[molten.artifact_auth_adoption.radicle_behavior]

The source migration MUST preserve existing Molten authentication and binding behavior under focused positive and negative checks without Rust implementation changes.

#### Scenario: Existing behavior passes

- GIVEN the unified source at the reviewed revision
- WHEN focused identity and binding suites run
- THEN existing acceptance and rejection behavior MUST pass.

#### Scenario: Behavioral drift blocks acceptance

- GIVEN a changed fixture result, feature graph, package version, source entry point, or authority boundary
- WHEN parity is evaluated
- THEN the migration MUST not be accepted.

### Requirement: No mixed or automatic source fallback
r[molten.artifact_auth_adoption.radicle_fallback]

Molten MUST use only the admitted unified source in manifests, locks, policy, and generated plans. It MUST NOT automatically select the predecessor repository, a sibling path, or a floating source.

#### Scenario: Unified source is unavailable

- GIVEN the admitted source cannot serve the exact object
- WHEN dependency resolution runs
- THEN resolution MUST fail visibly.

#### Scenario: Historical evidence remains bounded

- GIVEN documentation and archived receipts name predecessor sources
- WHEN active-source validation runs
- THEN historical evidence MAY remain while executable inputs use only the unified source.

### Requirement: Typed source migration evidence
r[molten.artifact_auth_adoption.radicle_evidence]

Molten MUST emit typed Nickel and JSON evidence with a BLAKE3 sidecar. The evidence MUST bind source and consumer sets, source-byte identity, policy, plans, checks, rollback, and non-claims.

#### Scenario: Complete migration evidence passes

- GIVEN matching source, lock, policy, plan, parity, and check observations
- WHEN receipt validation runs
- THEN it MUST accept deterministically.

#### Scenario: Missing linkage or overclaim fails

- GIVEN source drift, package widening, stale plans, missing negative evidence, automatic rollback, absolute evidence paths, or weakened non-claims
- WHEN receipt validation runs
- THEN it MUST fail closed.
