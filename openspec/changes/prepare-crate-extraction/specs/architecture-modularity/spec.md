## ADDED Requirements

### Requirement: Extractable crate boundaries are explicit
Aspen SHALL classify reusable crate candidates with explicit ownership, dependency, feature, documentation, release-metadata, and verification contracts before code is moved into a standalone repository or published package.

ID: architecture.modularity.extractable-crate-boundaries-are-explicit

#### Scenario: Candidate has an extraction manifest
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-extraction-manifest

- **WHEN** a crate is proposed for extraction
- **THEN** the repository SHALL document its intended audience, public API owner, crate category, default feature set, optional feature set, internal Aspen dependencies, external dependencies, binary dependencies, and release-readiness status
- **AND** the manifest SHALL identify which dependencies must be removed, hidden behind features, or kept as part of the reusable surface before extraction

#### Scenario: Extraction artifacts have canonical locations
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.extraction-artifacts-have-canonical-locations

- **WHEN** extraction readiness artifacts are authored
- **THEN** the inventory summary SHALL live at `docs/crate-extraction.md`, per-candidate manifests SHALL live under `docs/crate-extraction/`, the typed dependency policy SHALL live at `docs/crate-extraction/policy.ncl`, and change-specific verification evidence SHALL live under `openspec/changes/prepare-crate-extraction/evidence/`
- **AND** each evidence artifact SHALL be referenced from the change's `verification.md` before any related task is checked complete

#### Scenario: Candidate defines documentation and release metadata
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defines-documentation-and-release-metadata

- **WHEN** a candidate is marked extraction-ready
- **THEN** its manifest SHALL identify the crate description, license policy, README or crate-level documentation entrypoint, repository/homepage policy, feature table, public API stability owner, semver or compatibility policy, publish readiness, and any package metadata still required before external publication
- **AND** the readiness evidence SHALL record whether the candidate remains workspace-internal, publishable from the monorepo, or ready for a future repository split

#### Scenario: Candidate defaults avoid app bundles
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-defaults-avoid-app-bundles

- **WHEN** a reusable library candidate is built with default features
- **THEN** it SHALL NOT depend on Aspen node binaries, root `aspen` app bundles, RPC handler registries, dogfood binaries, UI crates, or runtime-only integration crates unless that dependency is the candidate's explicit purpose
- **AND** each runtime integration dependency SHALL require a named opt-in feature or adapter crate

#### Scenario: Typed dependency policy is the source of truth
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.typed-dependency-policy-is-source-of-truth

- **WHEN** dependency-boundary verification runs for an extraction candidate
- **THEN** it SHALL evaluate the candidate against `docs/crate-extraction/policy.ncl`
- **AND** that policy SHALL define candidate classes, forbidden crate categories, allowed dependency categories, feature-gated exception rules, tested feature sets, representative workspace consumers, and readiness-state restrictions
- **AND** the policy SHALL reject publishable or repository-split readiness labels unless license and publication policy have been explicitly resolved

#### Scenario: Dependency checks catch transitive and re-export leaks
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.dependency-checks-catch-transitive-and-reexport-leaks

- **WHEN** dependency-boundary verification runs for an extraction candidate
- **THEN** it SHALL check direct dependencies, transitive dependency paths, feature unification through workspace consumers, and re-export crates that can re-enable forbidden defaults
- **AND** the check SHALL fail if an app-bundle, runtime-only crate, or forbidden default feature is reachable through any path that the candidate claims is reusable by default

#### Scenario: Candidate has standalone verification rails
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-has-standalone-verification-rails

- **WHEN** a candidate is marked extraction-ready
- **THEN** the repository SHALL provide standalone compile, test, feature-topology, dependency-boundary, positive example, and negative boundary verification rails for that candidate
- **AND** those rails SHALL be saved as reviewable OpenSpec evidence for the change that marks the candidate ready

#### Scenario: Candidate is proven outside Aspen app bundle assumptions
ID: architecture.modularity.extractable-crate-boundaries-are-explicit.candidate-is-proven-outside-aspen-app-bundle-assumptions

- **WHEN** standalone verification is claimed for an extraction candidate
- **THEN** at least one proof SHALL build a downstream-style consumer that depends only on the candidate crate or candidate crate family through explicit path dependencies and does not depend on the root `aspen` package, Aspen node binaries, handler bundles, dogfood crates, or compatibility re-exports as its primary API
- **AND** that proof SHALL include the candidate's default feature set and any named reusable feature set claimed by the extraction manifest
- **AND** the proof environment SHALL either use a checked-in repo-local consumer fixture or a baseline clone/worktree with sibling path dependencies explicitly wired so Cargo cannot silently resolve sibling dependencies back to the live Aspen checkout

### Requirement: Reusable Redb Raft KV stack is layered
Aspen SHALL prepare the Redb-backed Raft KV system as a reusable vertical slice with separate type, storage, consensus-node, transport-adapter, and Aspen-integration layers.

ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered

#### Scenario: First Redb Raft KV layer map is traceable
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layer-map-is-traceable

- **WHEN** the first Redb Raft KV extraction slice is designed or implemented
- **THEN** the design, tasks, and evidence SHALL map current `aspen-kv-types`, `aspen-raft-types`, `aspen-redb-storage`, `aspen-raft` storage modules, `aspen-raft-network`, and Aspen integration crates to their target KV-type, Raft-type, storage, consensus-node, transport-adapter, and Aspen-integration layers
- **AND** any deviation from the proposed `aspen-kv-types`, `aspen-raft-kv-types`, `aspen-redb-storage`, `aspen-raft-kv`, `aspen-raft-network`, and `aspen-raft` compatibility split SHALL be documented before implementation proceeds

#### Scenario: First Redb Raft KV layers have manifests
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.first-redb-raft-kv-layers-have-manifests

- **WHEN** the first Redb Raft KV slice is prepared
- **THEN** each target layer SHALL have exactly one manifest file under `docs/crate-extraction/` recording owner, canonical crate or compatibility path, readiness state, dependency policy class, representative consumers/re-exporters, and verification rails
- **AND** the required first-slice manifest files SHALL be `aspen-kv-types.md`, `aspen-raft-kv-types.md`, `aspen-redb-storage.md`, `aspen-raft-kv.md`, `aspen-raft-network.md`, and `aspen-raft-compat.md`
- **AND** consolidation of multiple first-slice layers into one manifest SHALL be out of scope for this change unless the design is updated before implementation starts

#### Scenario: OpenRaft dependency boundary is explicit
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.openraft-dependency-boundary-is-explicit

- **WHEN** a Redb Raft KV target layer exposes or implements OpenRaft traits, type configuration, storage traits, network traits, errors, or log/state data structures
- **THEN** its manifest SHALL classify each `openraft` dependency path as public API, implementation detail, or feature-gated adapter surface
- **AND** the default reusable feature set SHALL NOT use `openraft` exposure to require root Aspen app configuration, Aspen bootstrap, handler registries, dogfood defaults, trust, secrets, SQL, coordination, or concrete iroh endpoint construction unless the manifest marks the layer as the explicit adapter for that concern
- **AND** upstream OpenRaft compatibility beyond Aspen's vendored version SHALL be recorded as deferred unless license, versioning, and compatibility policy are updated

#### Scenario: Storage layer has no Aspen node dependency
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.storage-layer-has-no-aspen-node-dependency

- **WHEN** the Redb storage layer is built for reusable consumption
- **THEN** it SHALL provide the Redb log/state-machine storage implementation, snapshot integrity helpers, chain hashing, KV versioning, CAS validation, and lease/TTL storage helpers without depending on Aspen node bootstrap, RPC handlers, cluster orchestration, trust, secrets, SQL, coordination, or binaries
- **AND** app-specific integration SHALL live in a higher layer or compatibility re-export

#### Scenario: Consensus node layer has a reusable boundary
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-node-layer-has-reusable-boundary

- **WHEN** the reusable consensus-node layer is built with default reusable features
- **THEN** it SHALL depend only on reusable KV, Raft type, storage, time/resource-limit, and consensus orchestration contracts needed to run the replicated KV node
- **AND** Aspen bootstrap, cluster orchestration, trust, secrets, SQL, coordination, client RPC API, handler registries, dogfood defaults, and binaries SHALL live in Aspen integration layers or named opt-in features

#### Scenario: Consensus KV facade hides Aspen app identity
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.consensus-kv-facade-hides-aspen-app-identity

- **WHEN** an external project uses the reusable Redb Raft KV facade
- **THEN** it SHALL be able to configure node identity, membership metadata, storage path, resource limits, and KV operations through reusable types rather than root Aspen application configuration
- **AND** Aspen-specific names, dogfood defaults, and feature bundles SHALL be optional wrappers instead of required public API concepts

#### Scenario: Transport adapter is explicit
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.transport-adapter-is-explicit

- **WHEN** the reusable KV stack is consumed without transport features
- **THEN** it SHALL expose storage and consensus contracts that do not require concrete iroh endpoint construction
- **AND** the iroh/IRPC network adapter SHALL be enabled through an explicit transport feature or adapter crate

#### Scenario: Single-fsync storage invariant remains verifiable
ID: architecture.modularity.reusable-redb-raft-kv-stack-is-layered.single-fsync-storage-invariant-remains-verifiable

- **WHEN** storage code moves between crates or gains a reusable facade
- **THEN** the single-transaction Redb log plus state-machine write path SHALL remain covered by tests or evidence that prove log entries and state mutations are committed atomically through the intended storage path
- **AND** chain-integrity and snapshot-integrity checks SHALL remain available to the reusable storage crate

### Requirement: Binaries are thin imperative shells over libraries
Aspen SHALL keep binaries as thin imperative shells that parse configuration, start runtimes, and call library APIs; reusable logic SHALL live in library crates with testable functional cores.

ID: architecture.modularity.binaries-are-thin-imperative-shells-over-libraries

#### Scenario: Binary-only code does not own reusable behavior
ID: architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.binary-only-code-does-not-own-reusable-behavior

- **WHEN** behavior is needed by both an Aspen binary and another Rust project
- **THEN** that behavior SHALL live in a library crate and the binary SHALL call it through a narrow shell
- **AND** tests for reusable logic SHALL target the library surface rather than the binary entrypoint

#### Scenario: Extracted libraries keep compatibility re-exports during migration
ID: architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.extracted-libraries-keep-compatibility-reexports-during-migration

- **WHEN** code is moved from an Aspen app crate into a reusable library crate
- **THEN** existing Aspen consumers SHALL either be migrated directly or receive temporary compatibility re-exports with a documented removal plan
- **AND** the compatibility path SHALL NOT become the only tested public API

#### Scenario: Aspen compatibility consumers stay verified during migration
ID: architecture.modularity.binaries-are-thin-imperative-shells-over-libraries.aspen-compatibility-consumers-stay-verified-during-migration

- **WHEN** a reusable crate boundary moves code that existing Aspen binaries, dogfood flows, handlers, bridges, gateways, web/TUI tools, or workspace libraries consume
- **THEN** the migration evidence SHALL include compile or test rails proving those Aspen compatibility consumers still work through direct migration or documented compatibility re-exports
- **AND** those rails SHALL be separate from the downstream-style reusable consumer proof

### Requirement: Extraction inventory is maintained
Aspen SHALL maintain a reviewable inventory of reusable crate candidates and classify each candidate by extraction readiness and expected crate family.

ID: architecture.modularity.extraction-inventory-is-maintained

#### Scenario: Inventory classifies candidate families
ID: architecture.modularity.extraction-inventory-is-maintained.inventory-classifies-candidate-families

- **WHEN** the extraction inventory is updated
- **THEN** each candidate SHALL be classified as a leaf type/helper crate, protocol/wire crate, storage/backend crate, runtime adapter crate, service library, or binary shell
- **AND** the inventory SHALL identify whether the candidate is ready now, needs dependency cleanup, needs API renaming, needs feature gating, needs tests/docs, or should remain Aspen-internal

#### Scenario: Inventory entries link to manifests and owners
ID: architecture.modularity.extraction-inventory-is-maintained.inventory-entries-link-to-manifests-and-owners

- **WHEN** a candidate appears in the extraction inventory
- **THEN** the inventory SHALL link to that candidate's extraction manifest or explicitly mark the manifest as not yet created
- **AND** every extraction-ready candidate SHALL have a public API owner and manifest link recorded before it can be marked ready

#### Scenario: Required family manifest stubs exist
ID: architecture.modularity.extraction-inventory-is-maintained.required-family-manifest-stubs-exist

- **WHEN** the initial broader extraction inventory is created
- **THEN** the repository SHALL include manifest stubs at `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, and `docs/crate-extraction/protocol-wire.md`
- **AND** service/runtime candidates not covered by those stubs SHALL be represented by inventory rows that record owner status, readiness state, next action, and either a manifest link or the exact value `manifest not yet created`

#### Scenario: Inventory includes high-value candidates beyond Raft KV
ID: architecture.modularity.extraction-inventory-is-maintained.inventory-includes-high-value-candidates-beyond-raft-kv

- **WHEN** the initial inventory is created
- **THEN** it SHALL assess at least the foundational type crates, auth/ticket crates, transport/RPC crates, coordination primitives, blob/castore/cache crates, commit DAG/KV branch crates, CI core/executor crates, jobs protocol/executor crates, trust/crypto crates, plugin API, Nickel config support, testing harness crates, and binaries
- **AND** the foundational and wire assessments SHALL explicitly include `crates/aspen-storage-types`, `crates/aspen-client-api`, `crates/aspen-cluster-types`, and `crates/aspen-traits`
- **AND** each assessment SHALL name the next preparation step for reuse
