## Purpose
Aspen maintains explicit crate boundaries, feature topology, and extraction contracts so reusable library surfaces stay independent from app/runtime shells unless explicitly opted in.
## Requirements
### Requirement: Acyclic no-std core boundary
Aspen MUST maintain an acyclic dependency boundary where the alloc-only `aspen-core` surface depends only on alloc-safe leaf crates, and `std`-bound runtime shells depend on that core rather than the reverse.

For this requirement:

- **alloc-safe** means either (a) an approved alloc-only direct prerequisite listed in this spec and configured with alloc-only features, or (b) a transitive dependency reachable only through those approved prerequisites and verified to avoid filesystem, process, thread, network, or async-runtime requirements.
- **leaf crate** means an alloc-safe dependency that provides shared types, codecs, or deterministic helpers and does not depend on Aspen shell crates or app bundles.
- **direct prerequisite** means a dependency selected directly by `crates/aspen-core/Cargo.toml` for the alloc-only build, not an arbitrary transitive dependency of another enabled crate.

ID: architecture.modularity.acyclic-no-std-core-boundary
#### Scenario: Runtime shells depend outward on core
ID: architecture.modularity.acyclic-no-std-core-boundary.runtime-shells-depend-outward-on-core

- **GIVEN** a shell facility such as transport, registry state, simulation persistence, or runtime convenience implementations
- **WHEN** its module or crate is composed
- **THEN** it MUST depend on the alloc-only core contracts and types
- **AND** the alloc-only core surface MUST NOT depend back on that shell facility

#### Scenario: Acyclic boundary proof is reviewable
ID: architecture.modularity.acyclic-no-std-core-boundary.acyclic-boundary-proof-is-reviewable

- **GIVEN** the alloc-only core and `std` shell boundary
- **WHEN** that boundary is verified for review
- **THEN** saved inventory, export-map, and source-audit artifacts under `openspec/changes/no-std-aspen-core/evidence/` SHALL show that alloc-only modules and exports do not depend on shell-only modules or crates
- **AND** `openspec/changes/no-std-aspen-core/verification.md` SHALL identify which artifacts prove the boundary remains acyclic

#### Scenario: Pure consumers avoid runtime shells
ID: architecture.modularity.acyclic-no-std-core-boundary.pure-consumers-avoid-runtime-shells

- **GIVEN** a workspace crate or external consumer that needs only Aspen core contracts
- **WHEN** it depends on the alloc-only `aspen-core` surface
- **THEN** Cargo resolution MUST succeed without enabling Aspen runtime shell features
- **AND** the consumer MUST NOT need to import shell modules just to reach foundational types and traits

### Requirement: Feature bundles are explicit and bounded
The build feature graph SHALL distinguish alloc-only core surfaces, `std`-only shell features, and higher-level convenience bundles so enabling foundational Aspen contracts does not silently pull runtime dependencies.

ID: architecture.modularity.feature-bundles-are-explicit-and-bounded
#### Scenario: Alloc-only core excludes runtime shells
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.alloc-only-core-excludes-runtime-shells

- **GIVEN** a contributor or downstream crate enables only the alloc-only `aspen-core` surface
- **WHEN** Cargo resolves the feature graph
- **THEN** the direct prerequisite set SHALL be limited to `aspen-constants`, `aspen-hlc`, `aspen-cluster-types`, `aspen-kv-types`, `aspen-traits`, `aspen-storage-types`, `async-trait`, `serde`, `bincode`, `base64`, `hex`, `snafu`, and `thiserror`
- **AND** those dependencies SHALL be configured with alloc-only feature settings, using `default-features = false` wherever a default feature set exists and enabling only features needed by the alloc-only core surface
- **AND** `std`-only shell facilities such as transport, simulation, or runtime registry helpers SHALL remain disabled

#### Scenario: Std compatibility is an explicit opt-in
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.std-compatibility-is-an-explicit-opt-in

- **GIVEN** a build profile requires shell helpers on top of the alloc-only core
- **WHEN** the profile is expressed through Cargo features or companion shell crates
- **THEN** the default feature set SHALL be empty
- **AND** the named opt-in path SHALL be `std` for shell helpers, `layer = std + dep:aspen-layer`, `global-discovery = std + dep:iroh-blobs`, `sql` as an alloc-safe optional feature, and `std + sql` for `sql` plus shell conveniences
- **AND** documentation at `docs/no-std-core.md` and compile-slice tests SHALL identify what each opt-in enables

#### Scenario: Dependency boundary is checked deterministically
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.dependency-boundary-is-checked-deterministically

- **GIVEN** the alloc-only `aspen-core` feature set
- **WHEN** its dependency graph is checked by the documented dependency-boundary verification commands
- **THEN** the direct prerequisite assertion SHALL match the alloc-only prerequisite set named above
- **AND** every transitive dependency in the saved full-graph artifact SHALL be reachable only through those approved prerequisites using alloc-safe feature settings
- **AND** every transitive dependency SHALL appear in the approved alloc-safe transitive allowlist for this change or cause verification to fail
- **AND** the full graph SHALL exclude `aspen-layer`, `aspen-time`, `aspen-disk`, `anyhow`, `n0-future`, `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `rand`, `tracing`, `chrono`, and `serde_json`
- **AND** the saved verification artifact SHALL identify the exact commands and results used for review under `openspec/changes/no-std-aspen-core/evidence/`
- **AND** `openspec/changes/no-std-aspen-core/verification.md` SHALL identify which artifact proves the dependency-boundary claim

#### Scenario: Feature-topology verification is reviewable
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.feature-topology-verification-is-reviewable

- **GIVEN** the public feature contract `default = []`, `std`, `layer = std + dep:aspen-layer`, `global-discovery = std + dep:iroh-blobs`, `sql`, and `std + sql`
- **WHEN** that contract is verified for review
- **THEN** the exact commands and results for the default, `std`, `layer`, `global-discovery`, `sql`, and `std + sql` compile slices SHALL be saved under `openspec/changes/no-std-aspen-core/evidence/`
- **AND** `openspec/changes/no-std-aspen-core/verification.md` SHALL identify which artifact proves each feature topology claim

### Requirement: Extraction inventory tracks coordination family
The crate extraction inventory at `docs/crate-extraction.md` SHALL include the coordination family with current readiness state, manifest link, owner status, and next action for both `aspen-coordination` and `aspen-coordination-protocol`.

ID: architecture-modularity.extraction-inventory-tracks-coordination

#### Scenario: Inventory row exists for coordination
- **WHEN** `docs/crate-extraction.md` is read
- **THEN** the broader candidate inventory table SHALL include a row for the coordination family
- **AND** the row SHALL link to `docs/crate-extraction/coordination.md`
- **AND** the readiness state SHALL reflect the verified extraction-readiness status
