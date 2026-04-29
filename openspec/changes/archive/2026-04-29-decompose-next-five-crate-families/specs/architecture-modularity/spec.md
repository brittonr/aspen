## ADDED Requirements

### Requirement: Next decomposition wave is selected and ordered
The extraction roadmap SHALL identify exactly five next decomposition target families after the already-proven Redb Raft KV, coordination, protocol/wire, transport/RPC, blob/castore/cache, and KV branch/commit-DAG work. The selected target families SHALL be ordered by prerequisite value, reusable-surface impact, and self-hosting impact. If a crate such as `aspen-jobs-protocol` was already covered by protocol/wire extraction, this wave SHALL treat it only as a domain-schema dependency for the selected family and SHALL NOT redo its generic wire-compatibility contract unless that family changes the schema.

ID: architecture.modularity.next-decomposition-wave-is-selected

#### Scenario: Roadmap names exact target families
- **WHEN** the next decomposition wave is planned
- **THEN** the target list SHALL include `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`
- **AND** `foundational-types` SHALL include `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, and `aspen-constants`
- **AND** `auth-ticket` SHALL include `aspen-auth-core`, `aspen-auth`, `aspen-ticket`, and `aspen-hooks-ticket`
- **AND** `jobs-ci-core` SHALL include `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-blob`, `aspen-jobs-worker-maintenance`, `aspen-jobs-worker-replication`, `aspen-jobs-worker-shell`, `aspen-jobs-worker-sql`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, and `aspen-ci-executor-nix`
- **AND** `trust-crypto-secrets` SHALL include `aspen-trust`, `aspen-crypto`, reusable pure/state-machine surfaces in `aspen-secrets`, and runtime consumer coverage for `aspen-secrets-handler`
- **AND** `testing-harness` SHALL include `aspen-testing-core`, `aspen-testing`, `aspen-testing-fixtures`, `aspen-testing-madsim`, `aspen-testing-network`, and `aspen-testing-patchbay`
- **AND** the roadmap SHALL record why deferred candidates such as `aspen-nickel`, `aspen-plugin-api`, config/plugin APIs, and binary-shell cleanup are not in this wave

#### Scenario: Foundational work precedes dependent families
- **WHEN** implementation begins on the selected wave
- **THEN** foundational type/helper seams SHALL be worked before jobs/CI, trust/secrets, and testing-harness targets depend on their trait or storage-type boundaries
- **AND** any out-of-order implementation SHALL document the prerequisite it bypasses and the temporary compatibility guard it relies on

### Requirement: Selected target manifests are complete before readiness changes
Each selected target family SHALL have a complete extraction manifest before any crate in that family is marked `extraction-ready-in-workspace`. The manifest SHALL follow the existing crate-extraction contract: audience, owner, package metadata, license/publication policy, feature contract, dependency decisions, compatibility plan, representative consumers, downstream fixture, verification rails, and readiness state.

ID: architecture.modularity.next-decomposition-manifests-are-complete

#### Scenario: Manifest coverage exists for each selected family
- **WHEN** a task claims manifest work for the next decomposition wave is complete
- **THEN** `docs/crate-extraction/foundational-types.md`, `docs/crate-extraction/auth-ticket.md`, `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction/trust-crypto-secrets.md`, and `docs/crate-extraction/testing-harness.md` SHALL exist
- **AND** each manifest SHALL name selected crates, public API owner, readiness state, default features, optional adapter/runtime features, and first blocker

#### Scenario: Existing stub manifests are upgraded before readiness
- **WHEN** `foundational-types` or `auth-ticket` is considered for readiness promotion
- **THEN** its existing stub manifest SHALL be upgraded to the full manifest format before any readiness state changes
- **AND** the manifest SHALL link durable evidence for dependency-boundary, downstream, compatibility, and positive/negative test rails

### Requirement: Policy and checker cover the next wave
The crate-extraction policy and deterministic readiness checker SHALL cover every selected target family with explicit forbidden categories, allowed backend/domain exceptions, tested feature sets, representative consumers, and negative mutation cases.

ID: architecture.modularity.next-decomposition-policy-covers-wave

#### Scenario: Policy contains selected family entries
- **WHEN** `docs/crate-extraction/policy.ncl` is evaluated for the next decomposition wave
- **THEN** it SHALL define candidate-family entries for `foundational-types`, `auth-ticket`, `jobs-ci-core`, `trust-crypto-secrets`, and `testing-harness`
- **AND** each entry SHALL include owner, canonical class, allowed dependency categories, forbidden dependency categories, tested feature sets, and representative consumers

#### Scenario: Checker validates selected family boundaries
- **WHEN** `scripts/check-crate-extraction-readiness.rs --candidate-family <family>` is run for any selected family
- **THEN** the checker SHALL verify manifest completeness, readiness-state restrictions, direct dependency boundaries, transitive dependency boundaries, representative-consumer feature unification, downstream fixture metadata, and compatibility evidence
- **AND** negative mutations SHALL prove the checker fails on at least forbidden runtime dependency, missing owner, invalid readiness state, missing downstream fixture, and missing compatibility evidence cases for that family

### Requirement: Each selected family proves standalone usage and compatibility
Every selected family SHALL include both positive and negative verification before readiness is raised: positive standalone compile/usage evidence with canonical APIs, negative boundary tests or checker evidence for app/runtime leaks, and compatibility evidence for Aspen consumers that still depend on the family through runtime features or existing public paths.

ID: architecture.modularity.next-decomposition-standalone-and-compatibility-proof

#### Scenario: Positive downstream proof uses canonical APIs
- **WHEN** a selected family claims standalone usability
- **THEN** its downstream fixture SHALL depend on canonical reusable crates directly rather than root `aspen` or compatibility re-exports as the primary API
- **AND** saved `cargo metadata` evidence SHALL prove no forbidden root app, handler, binary shell, or concrete runtime adapter dependency is present unless the manifest documents an explicit adapter-purpose exception

#### Scenario: Negative boundary proof rejects runtime leaks
- **WHEN** a selected family claims app/runtime concerns are gated
- **THEN** tests, source audits, or checker mutations SHALL prove that root `aspen`, handler registries, node bootstrap, UI/web/gateway binaries, dogfood, trust/secrets/SQL, concrete Iroh endpoint construction, or shell executors cannot enter reusable defaults except through documented feature gates or backend-purpose exceptions

#### Scenario: Aspen compatibility consumers remain verified
- **WHEN** a selected family moves APIs, features, trait bounds, or storage boundaries
- **THEN** every affected Aspen workspace consumer named in that family's manifest SHALL compile or test through direct migration or manifest-tracked temporary compatibility paths
- **AND** each temporary compatibility path SHALL include owner, test, and removal criteria

### Requirement: Selected target first blockers are explicit
The roadmap SHALL record the first blocker for each selected target family so implementation can start with a bounded first slice and avoid broad rewrites.

ID: architecture.modularity.next-decomposition-first-blockers-are-explicit

#### Scenario: First blockers are documented
- **WHEN** the next decomposition wave OpenSpec is ready for implementation
- **THEN** `foundational-types` SHALL name the `aspen-storage-types` Redb table split and `aspen-traits` capability split as first blockers
- **AND** `auth-ticket` SHALL name portable-auth consumer migration and token/ticket serialization compatibility as first blockers
- **AND** `jobs-ci-core` SHALL name the split between scheduler/config/run-state logic and worker/executor runtime shells as first blockers
- **AND** `trust-crypto-secrets` SHALL name pure crypto/state-machine extraction away from Raft/Iroh/secrets-service shells as first blockers
- **AND** `testing-harness` SHALL name the split between reusable simulation/workload helpers and Aspen cluster bootstrap helpers as first blockers
