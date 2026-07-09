# Runtime Spine Specification

## Purpose

Adds a Preserves boundary adoption profile for Molten runtime and evidence artifacts.

## Requirements

### Requirement: Preserves boundary profile
r[molten.runtime_spine.preserves_boundary_profile.contract] Molten MUST define a Preserves boundary profile that records artifact family, schema label, canonical-byte requirement, BLAKE3 identity field, adapter owner, core DTO boundary, allowed consumers, and non-claims.

#### Scenario: Canonical boundary artifact passes
r[molten.runtime_spine.preserves_boundary_profile.fixtures.positive]
- GIVEN a node control envelope, ticket, workflow bundle, receipt, or evidence envelope has canonical Preserves bytes, a supported schema label, BLAKE3 identity, adapter owner, and required non-claims
- WHEN boundary profile validation runs
- THEN validation MUST pass and preserve the artifact family and schema label.

#### Scenario: Invalid boundary artifact fails
r[molten.runtime_spine.preserves_boundary_profile.fixtures.negative]
- GIVEN a boundary artifact has non-canonical bytes, missing schema label, stale BLAKE3 ref, unsupported consumer, or missing non-claims
- WHEN boundary profile validation runs
- THEN validation MUST fail with deterministic diagnostics.

### Requirement: Adapter-only core boundary
r[molten.runtime_spine.preserves_boundary_profile.validation] Molten runtime cores SHOULD consume typed DTOs produced by adapter modules rather than raw Preserves values for profile-managed surfaces.

#### Scenario: Raw Preserves core coupling is rejected
r[molten.runtime_spine.preserves_boundary_profile.validation.core_coupling]
- GIVEN a profile-managed boundary surface is marked adapter-only
- WHEN validation detects a new core module dependency on raw Preserves values for that surface
- THEN validation MUST fail with a raw-preserves-core-coupling diagnostic.

### Requirement: Boundary docs
r[molten.runtime_spine.preserves_boundary_profile.docs] Documentation MUST state that the Preserves profile proves canonical boundary identity only.

#### Scenario: Non-claim is visible
r[molten.runtime_spine.preserves_boundary_profile.docs.non_claims]
- GIVEN a profile report passes
- WHEN the supported claim is rendered
- THEN it MUST state that the profile does not prove transport liveness, actor authority correctness, replay completeness, or Valence Evidence IR acceptance.

### Requirement: Final validation
r[molten.runtime_spine.preserves_boundary_profile.final_validation] The change MUST include positive and negative fixtures plus focused validation evidence before archive.

#### Scenario: Fixture suite covers boundary health
r[molten.runtime_spine.preserves_boundary_profile.final_validation.fixtures]
- GIVEN valid and invalid Preserves boundary fixtures
- WHEN focused validation runs
- THEN valid fixtures MUST pass and invalid fixtures MUST fail closed.
