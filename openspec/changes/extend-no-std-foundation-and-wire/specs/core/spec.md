## MODIFIED Requirements

### Requirement: No-std core baseline
ID: core.no-std-core-baseline

The `aspen-core` crate MUST provide an alloc-backed `no_std` build that exposes Aspen's foundational types, traits, constants, and deterministic helper logic without requiring filesystem, process, networking, thread, async-runtime, or storage-engine dependencies.

#### Scenario: Bare dependency uses alloc-only default
ID: core.no-std-core-baseline.bare-dependency-uses-alloc-only-default

- **GIVEN** a consumer that depends on `aspen-core` without overriding features
- **WHEN** Cargo resolves the dependency
- **THEN** the default feature set MUST be empty
- **AND** the consumer MUST receive the alloc-only surface unless it explicitly opts into the documented shell compatibility path through `aspen-core-shell`

#### Scenario: Alloc-only build succeeds
ID: core.no-std-core-baseline.alloc-only-build-succeeds

- **GIVEN** a consumer that depends on `aspen-core` only for Aspen contracts and deterministic helpers
- **WHEN** the crate is built with its documented alloc-only configuration and `std` support disabled
- **THEN** the build MUST succeed
- **AND** Cargo resolution MUST NOT pull `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `anyhow`, `chrono`, `serde_json`, `redb`, `libc`, `aspen-disk`, `aspen-time`, filesystem persistence helpers, or other `std`-only runtime dependencies through that alloc-only surface

#### Scenario: Bare-default downstream consumer remains supported
ID: core.no-std-core-baseline.bare-default-downstream-consumer-remains-supported

- **GIVEN** the workspace smoke consumer `crates/aspen-core-no-std-smoke`
- **WHEN** it depends on `aspen-core` without overriding features
- **THEN** it MUST compile without enabling shell features
- **AND** it MUST be able to import foundational core types or traits from the alloc-only surface, including alloc-safe storage record types such as `aspen_core::storage::KvEntry`

#### Scenario: Alloc-only build rejects shell imports
ID: core.no-std-core-baseline.alloc-only-build-rejects-shell-imports

- **GIVEN** a compile-fail fixture built against alloc-only `aspen-core`
- **WHEN** the fixture imports a `std`-only API such as transport, simulation, or Redb table-definition helpers without enabling the documented shell path
- **THEN** compilation MUST fail
- **AND** the failure MUST identify that the requested API is outside the alloc-only surface

#### Scenario: Compile-fail verification is reviewable
ID: core.no-std-core-baseline.compile-fail-verification-is-reviewable

- **GIVEN** compile-fail fixtures under `crates/aspen-core/tests/ui/` that exercise shell imports without `std`
- **WHEN** their negative verification is saved for review
- **THEN** the exact fixture paths, commands, and stderr assertions SHALL be recorded under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the fixture set SHALL cover at least failed alloc-only imports of `aspen_core::{AppRegistry, NetworkTransport, SimulationArtifact, ContentDiscovery, DirectoryLayer}` plus failed alloc-only imports of `aspen_core::storage::SM_KV_TABLE` without the required shell features
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each compile-fail expectation

#### Scenario: Std-dependent helpers require explicit opt-in
ID: core.no-std-core-baseline.std-dependent-helpers-require-explicit-opt-in

- **GIVEN** a consumer that needs runtime helpers such as transport integration, in-memory registries, simulation persistence, or Redb table definitions
- **WHEN** the consumer opts into the documented shell compatibility path by depending on `aspen-core-shell` (often aliased locally as `aspen-core`)
- **THEN** those helpers MUST become available only through that explicit opt-in boundary
- **AND** the alloc-only `aspen-core` surface MUST remain unchanged for consumers that do not opt in

#### Scenario: Std-gated shell APIs keep current public paths
ID: core.no-std-core-baseline.std-gated-shell-apis-keep-current-public-paths

- **GIVEN** a shell API currently exposed from `aspen_core::*`, including Redb-specific storage helpers such as `aspen_core::storage::SM_KV_TABLE`
- **WHEN** a consumer enables the required shell feature for this cut through the established `aspen-core-shell` alias pattern
- **THEN** that API MUST remain available at its current public path
- **AND** the change MUST gate that path rather than rename it

#### Scenario: Module-family boundary matches documented inventory
ID: core.no-std-core-baseline.module-family-boundary-matches-documented-inventory

- **GIVEN** the documented alloc-only families `circuit_breaker`, `cluster`, `constants` scalar and numeric exports, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, `storage` record types and codecs, `traits`, `types`, `vault`, `verified`, and alloc-safe optional `sql`
- **AND** the documented shell families `app_registry`, `context`, `simulation`, `transport`, Redb-specific storage table definitions, `utils`, `test_support`, runtime-only prelude additions, and duration convenience exports
- **AND** the documented optional `std`-gated path families `layer` and `global-discovery`
- **WHEN** the boundary is reviewed
- **THEN** the alloc-only families SHALL remain available without `std`
- **AND** the shell families plus optional `std`-gated path families SHALL remain gated or shell-only as documented for this cut

#### Scenario: Alloc-only storage surface excludes Redb table definitions
ID: core.no-std-core-baseline.alloc-only-storage-surface-excludes-redb-table-definitions

- **GIVEN** an alloc-only consumer of `aspen-core`
- **WHEN** it imports `aspen_core::storage`
- **THEN** alloc-safe record types such as `KvEntry` MUST remain available
- **AND** Redb table definitions such as `SM_KV_TABLE` MUST remain unavailable unless the consumer opts into the documented shell path through `aspen-core-shell`, preserving the shell-facing import `aspen_core::storage::SM_KV_TABLE`

#### Scenario: Shell alias path proof is reviewable
ID: core.no-std-core-baseline.shell-alias-path-proof-is-reviewable

- **GIVEN** the preserved shell-facing import `aspen_core::storage::SM_KV_TABLE` through the `aspen-core-shell` alias pattern
- **WHEN** that compatibility claim is verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-core-shell`, `cargo check -p aspen-core-shell --features layer`, `cargo check -p aspen-core-shell --features global-discovery`, `cargo check -p aspen-core-shell --features sql`, and `cargo check --manifest-path crates/aspen-core/tests/fixtures/shell-alias-smoke/Cargo.toml` SHALL be saved under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the alias-form consumer fixture SHALL live at `crates/aspen-core/tests/fixtures/shell-alias-smoke/` and import `aspen_core::storage::SM_KV_TABLE` through the alias form `aspen-core = { package = "aspen-core-shell", ... }`
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves the alias-path claim

#### Scenario: Representative std consumers remain supported
ID: core.no-std-core-baseline.representative-std-consumers-remain-supported

- **GIVEN** the representative workspace consumers `crates/aspen-cluster`, `crates/aspen-client`, `crates/aspen-cli`, `crates/aspen-rpc-handlers`, and the root `aspen` bundle slice built with `--no-default-features --features node-runtime`
- **WHEN** they opt into the documented `std` compatibility path
- **THEN** they MUST continue to compile against `aspen-core`
- **AND** the verification matrix MUST keep those consumer slices green

#### Scenario: Compile-slice verification is reviewable
ID: core.no-std-core-baseline.compile-slice-verification-is-reviewable

- **GIVEN** the alloc-only crate, alloc-safe `sql`, the shell package `aspen-core-shell` with its relevant feature combinations, the smoke consumer, and representative std consumers
- **WHEN** their compile slices are verified for review
- **THEN** the exact commands and results for `cargo check -p aspen-core`, `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core --no-default-features --features sql`, `cargo check -p aspen-core-shell`, `cargo check -p aspen-core-shell --features layer`, `cargo check -p aspen-core-shell --features global-discovery`, `cargo check -p aspen-core-shell --features sql`, `cargo check -p aspen-core-no-std-smoke`, `cargo check -p aspen-cluster`, `cargo check -p aspen-client`, `cargo check -p aspen-cli`, `cargo check -p aspen-rpc-handlers`, and `cargo check -p aspen --no-default-features --features node-runtime` SHALL be saved under `openspec/changes/extend-no-std-foundation-and-wire/evidence/`
- **AND** the saved smoke-consumer artifact SHALL prove alloc-only `aspen_core::storage::KvEntry` remains reachable after the storage split
- **AND** `openspec/changes/extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each slice
