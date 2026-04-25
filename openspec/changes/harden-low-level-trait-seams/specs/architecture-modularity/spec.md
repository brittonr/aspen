## ADDED Requirements

### Requirement: Extraction candidates expose narrow reusable seams
Aspen SHALL require low-level extraction candidates to define the smallest stable trait or port surface needed by reusable consumers, with broader composite traits retained only for compatibility or convenience.

ID: architecture.modularity.extraction-candidates-expose-narrow-reusable-seams

#### Scenario: Candidate manifest identifies trait seams
- **WHEN** a low-level crate candidate manifest is authored or updated
- **THEN** the manifest SHALL identify its reusable trait/interface seams, composite compatibility traits, adapter implementations, public API owner, and feature gates for runtime integrations
- **AND** the manifest SHALL explain why each broad trait dependency remains necessary or how it will be split

#### Scenario: Reusable defaults avoid broad root contracts
- **WHEN** a low-level candidate is built with reusable default features
- **THEN** its API SHALL avoid requiring root `aspen-core`, cluster control, handler registries, node bootstrap, concrete transport, dogfood, UI, or binary shell contracts when a leaf trait/type crate or domain port trait is sufficient
- **AND** dependency evidence SHALL prove any remaining runtime dependency is either the crate's explicit backend purpose or hidden behind a named adapter feature

#### Scenario: Compatibility composites are not primary reusable API
- **WHEN** a composite trait preserves old full-store or full-service behavior during migration
- **THEN** downstream-style fixtures SHALL use the new narrow trait surface as the primary API
- **AND** compatibility consumers SHALL be verified separately through compile or test rails

### Requirement: Adapter crates own runtime and persistence coupling
Aspen SHALL place concrete runtime, storage, transport, and cluster coupling in adapter crates, adapter modules, or named feature surfaces rather than in low-level domain logic.

ID: architecture.modularity.adapter-crates-own-runtime-and-persistence-coupling

#### Scenario: Runtime adapter dependencies are opt-in
- **WHEN** a reusable crate needs a concrete runtime integration such as iroh transfer, OpenRaft implementation, Aspen KV persistence, or client RPC schemas
- **THEN** that integration SHALL live behind a named feature, adapter crate, or adapter module identified in the manifest
- **AND** reusable default dependency checks SHALL prove callers that do not use the adapter do not receive the concrete runtime dependency

#### Scenario: Domain logic is testable without adapters
- **WHEN** a low-level domain operation is tested
- **THEN** it SHALL be reachable through a pure function, narrow trait, or in-memory adapter without starting Aspen node runtime, handler registries, concrete iroh endpoints, Redb databases unless Redb is the explicit storage backend under test, or OpenRaft services unless OpenRaft behavior is the explicit adapter under test
- **AND** tests SHALL include positive and negative cases for the domain operation

#### Scenario: Adapter errors are mapped at the boundary
- **WHEN** an adapter calls a concrete backend and receives backend-specific errors
- **THEN** it SHALL map them to the narrow domain error type at the seam or explicitly document the backend error as part of the adapter API
- **AND** tests SHALL cover successful mapping and at least one backend failure path

### Requirement: Low-level trait migrations remain traceable
Aspen SHALL track low-level trait and adapter migrations with explicit OpenSpec evidence so crate-productization work can proceed incrementally without breaking existing consumers.

ID: architecture.modularity.low-level-trait-migrations-remain-traceable

#### Scenario: Migration tasks cite requirements
- **WHEN** tasks split or introduce a low-level trait seam
- **THEN** each task SHALL include `covers=` references to the affected low-level trait seam requirement and any modified architecture-modularity requirement
- **AND** checked tasks SHALL have durable evidence in `verification.md`

#### Scenario: Negative fixture coverage is required
- **WHEN** a low-level seam is marked complete
- **THEN** evidence SHALL include at least one negative case proving an invalid broad dependency, missing adapter metadata, unavailable feature, malformed persisted value, or stale compare/update operation fails correctly
- **AND** a positive-only test suite SHALL NOT be sufficient for completion

#### Scenario: Publication readiness waits for seam evidence
- **WHEN** a low-level crate is proposed for `extraction-ready-in-workspace`, `publishable from monorepo`, or `future repository split candidate`
- **THEN** readiness evidence SHALL include trait seam documentation, downstream-style fixture proof, negative boundary proof, and compatibility proof for affected Aspen consumers
- **AND** publishable and repository-split states SHALL still remain blocked until human license/publication policy is resolved
