## ADDED Requirements

### Requirement: Coordination crate has no Aspen app-bundle dependency
`aspen-coordination` default features SHALL NOT depend on Aspen app bundles, handler registries, node binaries, dogfood, UI/TUI/web, trust/secrets, SQL, or concrete transport endpoints.

ID: coordination-extraction.coordination-defaults-avoid-app-bundles

#### Scenario: Default features compile without app bundles
- **WHEN** `cargo check -p aspen-coordination` is run
- **THEN** compilation SHALL succeed
- **AND** `cargo tree -p aspen-coordination --edges normal` SHALL show no packages named `aspen` (root), `aspen-cluster`, `aspen-raft`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-transport`, `aspen-node`, `aspen-cli`, `aspen-tui`, `aspen-dogfood`, `iroh`, `iroh-base`, `irpc`, or any handler crate

#### Scenario: No-default features compile without app bundles
- **WHEN** `cargo check -p aspen-coordination --no-default-features` is run
- **THEN** compilation SHALL succeed
- **AND** `cargo tree -p aspen-coordination --no-default-features --edges normal` SHALL show no packages named `aspen` (root), `aspen-cluster`, `aspen-raft`, `aspen-rpc-core`, `aspen-rpc-handlers`, `aspen-transport`, `aspen-node`, `aspen-cli`, `aspen-tui`, `aspen-dogfood`, `iroh`, `iroh-base`, `irpc`, or any handler crate

#### Scenario: Default features do not depend on aspen-core
- **WHEN** `cargo tree -p aspen-coordination --edges normal` is run
- **THEN** the output SHALL NOT include `aspen-core` or `aspen-core-shell` as a direct or transitive dependency

### Requirement: Coordination protocol crate is standalone
`aspen-coordination-protocol` SHALL depend only on serialization libraries and SHALL NOT depend on any Aspen crate.

ID: coordination-extraction.protocol-is-standalone

#### Scenario: Protocol crate has no Aspen dependencies
- **WHEN** `cargo tree -p aspen-coordination-protocol` is run
- **THEN** no dependency package below the selected root crate starting with `aspen-` SHALL appear in the dependency tree

#### Scenario: Protocol crate compiles standalone
- **WHEN** `cargo check -p aspen-coordination-protocol` is run
- **THEN** compilation SHALL succeed

### Requirement: Coordination has extraction manifest
A manifest at `docs/crate-extraction/coordination.md` SHALL document the coordination family's candidate name, class, intended audience, dependency contract, feature surface, verification rails, readiness state, and owner status.

ID: coordination-extraction.has-extraction-manifest

#### Scenario: Manifest exists with required fields
- **WHEN** `docs/crate-extraction/coordination.md` is read
- **THEN** it SHALL contain candidate name, canonical class, intended audience, public API owner, readiness state, feature contract table, dependency decisions, compatibility plan, and verification rails sections

### Requirement: Coordination is registered in extraction policy
`aspen-coordination` and `aspen-coordination-protocol` SHALL be registered as candidates in `docs/crate-extraction/policy.ncl` with appropriate classes and dependency rules.

ID: coordination-extraction.registered-in-policy

#### Scenario: Policy includes coordination candidates
- **WHEN** `docs/crate-extraction/policy.ncl` is parsed
- **THEN** it SHALL include entries for `aspen-coordination` as `service library` and `aspen-coordination-protocol` as `protocol/wire`

### Requirement: Extraction readiness checker verifies coordination
The extraction-readiness checker SHALL accept `--candidate-family coordination` and verify the coordination family against the same contract used for Redb Raft KV: manifest presence, readiness-state restrictions, dependency boundary, and forbidden transitive paths.

ID: coordination-extraction.checker-verifies-coordination

#### Scenario: Checker passes for coordination family
- **WHEN** `scripts/check-crate-extraction-readiness.rs --candidate-family coordination` is run
- **THEN** it SHALL exit 0 with all checks passing

### Requirement: Downstream consumer proof exists
A downstream-style consumer SHALL prove that coordination primitives can be used by depending only on `aspen-coordination`, `aspen-kv-types`, and `aspen-traits` without depending on the root `aspen` package or any Aspen binary/handler crate.

ID: coordination-extraction.downstream-consumer-proof

#### Scenario: Consumer compiles without root aspen dependency
- **WHEN** the downstream consumer fixture is compiled
- **THEN** `cargo metadata` for the consumer SHALL NOT list `aspen` (root package) as a dependency
- **AND** the consumer SHALL successfully use at least one coordination primitive type

### Requirement: All workspace consumers still compile
All 9 workspace crates that depend on `aspen-coordination` or `aspen-coordination-protocol` SHALL continue to compile after the dependency cleanup.

ID: coordination-extraction.consumers-still-compile

#### Scenario: Direct coordination consumers compile
- **WHEN** `cargo check` is run for each of `aspen-cluster`, `aspen-core-essentials-handler`, `aspen-job-handler`, `aspen-jobs`, `aspen-rpc-core`, `aspen-rpc-handlers`, and `aspen-testing`
- **THEN** each SHALL compile successfully

#### Scenario: Protocol consumer compiles
- **WHEN** `cargo check -p aspen-client-api` is run
- **THEN** compilation SHALL succeed

#### Scenario: Optional consumers compile
- **WHEN** `cargo check -p aspen-testing --features jobs` and `cargo check -p aspen-raft --features coordination` are run
- **THEN** both commands SHALL succeed
