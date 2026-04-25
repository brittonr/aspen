## ADDED Requirements

### Requirement: KV capability traits are narrow and composable
Low-level reusable crates SHALL depend on the smallest KV capability trait that satisfies their operation set instead of requiring a monolithic cluster KV trait.

ID: low-level-trait-seams.kv-capability-traits-are-narrow-and-composable

#### Scenario: Read-only consumers depend only on read traits
- **WHEN** a reusable crate only reads one key or scans a prefix
- **THEN** its public generic bounds SHALL use read/scan capability traits instead of requiring write, delete, cluster control, or local-read capabilities
- **AND** compile evidence SHALL prove a downstream fixture can implement only those capabilities and still use the crate API

#### Scenario: Composite KV trait preserves compatibility
- **WHEN** existing Aspen consumers need the legacy full KV surface during migration
- **THEN** a composite trait SHALL preserve the full read/write/delete/scan contract without forcing all new low-level APIs to depend on it
- **AND** compatibility compile evidence SHALL prove existing full-store consumers continue to build

#### Scenario: Local and linearizable reads are explicit capabilities
- **WHEN** a reusable API can accept stale local state-machine data
- **THEN** it SHALL request a local-read capability explicitly rather than hiding local-read behavior in a broad KV trait default
- **AND** negative tests or fixture metadata SHALL show linearizable-only stores are not required to expose local reads accidentally

### Requirement: Cache persistence uses storage port traits
`aspen-cache` SHALL keep Nix cache parsing, narinfo generation, NAR hashing, and signing logic independent from Aspen KV persistence by routing persistence through narrow cache storage port traits.

ID: low-level-trait-seams.cache-persistence-uses-storage-port-traits

#### Scenario: Signing key persistence is a dedicated port
- **WHEN** cache signing keys are loaded or stored
- **THEN** reusable code SHALL depend on a `SigningKeyStore`-style port rather than directly importing root `aspen-core` or concrete KV request types
- **AND** positive tests SHALL cover successful load/save and missing-key behavior
- **AND** negative tests SHALL cover malformed persisted keys and storage errors

#### Scenario: Cache index operations are split by capability
- **WHEN** a caller only looks up cache entries
- **THEN** it SHALL be able to depend on a lookup trait without also receiving publish or stats mutation APIs
- **AND** downstream fixture evidence SHALL prove lookup-only use without an Aspen cluster runtime dependency

#### Scenario: KV-backed cache adapters are opt-in
- **WHEN** `aspen-cache` persists cache entries through Aspen KV
- **THEN** the KV adapter SHALL be behind a named feature or adapter crate that depends only on `aspen-traits` and `aspen-kv-types` for KV contracts
- **AND** default-feature dependency evidence SHALL prove root `aspen-core`, node binaries, handlers, and testing crates are absent

### Requirement: Blob offload is generic over blob capabilities
Blob/KV offload code SHALL depend on blob read/write capability traits and KV capability traits rather than a concrete blob store implementation or root Aspen runtime crate.

ID: low-level-trait-seams.blob-offload-is-generic-over-blob-capabilities

#### Scenario: Blob-aware KV wrapper accepts any read/write blob store
- **WHEN** a large KV value is offloaded to blob storage
- **THEN** the wrapper SHALL be generic over a blob capability surface sufficient for add, get, protect, and unprotect operations
- **AND** an in-memory blob-store fixture SHALL prove the wrapper does not require `IrohBlobStore`

#### Scenario: Iroh transfer concerns stay in transfer adapter traits
- **WHEN** blob transfer APIs expose iroh tickets, public keys, or peer downloads
- **THEN** those APIs SHALL be isolated in transfer-specific traits or feature-gated adapter surfaces
- **AND** non-transfer blob users SHALL compile without requesting peer transfer capabilities

#### Scenario: Blob replication keeps client RPC out of defaults
- **WHEN** blob replication uses client RPC request/response schemas
- **THEN** that dependency SHALL remain behind a replication feature or adapter crate
- **AND** default blob API dependency evidence SHALL prove `aspen-client-api` and handler crates are absent

### Requirement: Redb storage exposes storage ports below OpenRaft adapters
Reusable Redb storage SHALL expose Aspen-owned storage port traits for state, leases, snapshots, log segments, and integrity chain behavior before those ports are adapted to OpenRaft traits.

ID: low-level-trait-seams.redb-storage-exposes-ports-below-openraft-adapters

#### Scenario: State and log stores are independently testable
- **WHEN** Redb storage logic is tested outside OpenRaft
- **THEN** state-machine mutation, log append/read, lease, snapshot, and integrity behaviors SHALL be reachable through Aspen-owned storage ports
- **AND** tests SHALL include positive writes/reads and negative corruption, missing-record, and partial-commit cases

#### Scenario: OpenRaft adapter owns OpenRaft coupling
- **WHEN** reusable Redb storage implements OpenRaft traits
- **THEN** the OpenRaft-specific trait implementation SHALL be treated as an adapter layer over Aspen-owned storage ports
- **AND** manifests and dependency checks SHALL classify OpenRaft exposure as public API, implementation detail, or named adapter feature

#### Scenario: Single-fsync invariant remains visible through ports
- **WHEN** storage ports are introduced or moved
- **THEN** evidence SHALL prove the single Redb transaction path still commits log entries and state-machine mutations atomically
- **AND** chain-integrity and snapshot-integrity verification SHALL remain reachable from the reusable storage crate

### Requirement: Branch and commit DAG use domain store traits
`aspen-kv-branch` and `aspen-commit-dag` SHALL route persistence through domain store traits so branch overlay logic and commit history logic are not tied directly to a full Aspen KV store surface.

ID: low-level-trait-seams.branch-and-commit-dag-use-domain-store-traits

#### Scenario: Commit DAG persistence uses a commit store trait
- **WHEN** commit metadata is loaded, saved, diffed, or garbage-collected
- **THEN** the domain logic SHALL depend on a `CommitStore`-style trait with bounded operations rather than a broad cluster KV trait
- **AND** tests SHALL cover valid commits, missing parents, malformed commit payloads, and over-limit graph traversal

#### Scenario: Branch tips use a branch-tip store trait
- **WHEN** branch tips are read or updated
- **THEN** compare/update semantics SHALL be exposed through a branch-tip persistence trait that can be implemented by KV-backed and in-memory fixtures
- **AND** negative tests SHALL cover stale-tip conflicts and failed compare/update operations

#### Scenario: KV adapters are compatibility layers
- **WHEN** Aspen KV backs branch/DAG persistence
- **THEN** the KV-backed implementation SHALL live as an adapter over the domain store traits
- **AND** compatibility evidence SHALL prove current Aspen consumers still compile through direct migration or documented re-exports

### Requirement: Time and logical clocks are injected at reusable seams
Reusable crates SHALL receive time and logical-clock behavior through explicit provider traits rather than reading clocks directly or exposing concrete clock library types unnecessarily.

ID: low-level-trait-seams.time-and-logical-clocks-are-injected-at-reusable-seams

#### Scenario: Wall-clock reads use TimeProvider
- **WHEN** reusable low-level code needs current Unix time for stats, TTLs, leases, deadlines, or timestamps
- **THEN** the code SHALL accept an `aspen-time::TimeProvider` or an explicit timestamp parameter rather than reading the ambient wall clock directly
- **AND** tests SHALL include simulated-time positive cases and boundary/error cases for expired, non-expired, and saturating timestamp computations

#### Scenario: HLC behavior is behind a logical clock trait
- **WHEN** consumers need causally ordered timestamps
- **THEN** they SHALL depend on a logical-clock trait with `now` and `observe`-style operations instead of depending directly on `uhlc` types unless the crate is explicitly the HLC adapter
- **AND** tests SHALL cover monotonic generation, observe/update success, and rejected future or malformed timestamps

### Requirement: Low-level seam verification is durable and negative
Every low-level seam introduced by this change SHALL include positive usage proof, negative boundary proof, and compatibility proof before tasks are checked complete.

ID: low-level-trait-seams.low-level-seam-verification-is-durable-and-negative

#### Scenario: Positive downstream fixtures use canonical traits
- **WHEN** a trait seam is claimed reusable
- **THEN** a downstream-style fixture SHALL use the canonical trait or adapter directly without root `aspen`, node binaries, handlers, dogfood crates, or compatibility re-exports as the primary API
- **AND** the fixture metadata SHALL be saved as OpenSpec evidence

#### Scenario: Negative boundary checks reject broad dependencies
- **WHEN** dependency-boundary verification runs for affected low-level crates
- **THEN** it SHALL fail if reusable defaults reach root `aspen-core` where a leaf trait/type dependency is sufficient, handler crates, node bootstrap, concrete transport, or app-runtime integration crates
- **AND** negative mutation evidence SHALL prove the checker catches at least one forbidden dependency and one missing trait/adaptor metadata field

#### Scenario: Compatibility consumers remain verified
- **WHEN** existing Aspen consumers are migrated to narrow traits or adapter crates
- **THEN** compile or test rails SHALL prove those consumers still work
- **AND** compatibility rails SHALL be separate from downstream-style reusable proofs
