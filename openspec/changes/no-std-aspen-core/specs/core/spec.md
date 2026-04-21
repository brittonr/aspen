## ADDED Requirements

### Requirement: No-std core baseline

The `aspen-core` crate MUST provide an alloc-backed `no_std` build that exposes Aspen's foundational types, traits, constants, and deterministic helper logic without requiring filesystem, process, networking, thread, or async-runtime dependencies.

#### Scenario: Bare dependency uses alloc-only default

- **GIVEN** a consumer that depends on `aspen-core` without overriding features
- **WHEN** Cargo resolves the dependency
- **THEN** the default feature set MUST be empty
- **AND** the consumer MUST receive the alloc-only surface unless it explicitly opts into `std`

#### Scenario: Alloc-only build succeeds

- **GIVEN** a consumer that depends on `aspen-core` only for Aspen contracts and deterministic helpers
- **WHEN** the crate is built with its documented alloc-only configuration and `std` support disabled
- **THEN** the build MUST succeed
- **AND** Cargo resolution MUST NOT pull `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `anyhow`, `chrono`, `serde_json`, `aspen-disk`, `aspen-time`, filesystem persistence helpers, or other `std`-only runtime dependencies through that alloc-only surface

#### Scenario: Bare-default downstream consumer remains supported

- **GIVEN** the workspace smoke consumer `crates/aspen-core-no-std-smoke`
- **WHEN** it depends on `aspen-core` without overriding features
- **THEN** it MUST compile without enabling shell features
- **AND** it MUST be able to import foundational core types or traits from the alloc-only surface

#### Scenario: Alloc-only build rejects shell imports

- **GIVEN** a compile-fail fixture built against alloc-only `aspen-core`
- **WHEN** the fixture imports a `std`-only API such as transport or simulation helpers without enabling the documented shell path
- **THEN** compilation MUST fail
- **AND** the failure MUST identify that the requested API is outside the alloc-only surface

#### Scenario: Compile-fail verification is reviewable

- **GIVEN** compile-fail fixtures under `crates/aspen-core/tests/ui/` that exercise shell imports without `std`
- **WHEN** their negative verification is saved for review
- **THEN** the exact fixture paths, commands, and stderr assertions SHALL be recorded under `openspec/changes/no-std-aspen-core/evidence/`
- **AND** the fixture set SHALL cover at least `aspen_core::{AppRegistry, NetworkTransport, SimulationArtifact, ContentDiscovery, DirectoryLayer}` without the required shell features
- **AND** `openspec/changes/no-std-aspen-core/verification.md` SHALL identify which artifact proves each compile-fail expectation

#### Scenario: Std-dependent helpers require explicit opt-in

- **GIVEN** a consumer that needs runtime helpers such as transport integration, in-memory registries, or simulation persistence
- **WHEN** the consumer enables the documented `std` compatibility path
- **THEN** those helpers MUST become available only through that explicit opt-in boundary
- **AND** the alloc-only `aspen-core` surface MUST remain unchanged for consumers that do not opt in

#### Scenario: Std-gated shell APIs keep current public paths

- **GIVEN** a shell API currently exposed from `aspen_core::*`
- **WHEN** a consumer enables the required shell feature for this first cut
- **THEN** that API MUST remain available at its current public path
- **AND** the change MUST gate that path rather than rename it

#### Scenario: Module-family boundary matches documented inventory

- **GIVEN** the documented alloc-only families `circuit_breaker`, `cluster`, `constants` scalar and numeric exports, `crypto`, `error`, `hlc`, `kv`, alloc-safe `prelude`, `protocol`, `spec`, `storage`, `traits`, `types`, `vault`, `verified`, and alloc-safe optional `sql`
- **AND** the documented shell families `app_registry`, `context`, `simulation`, `transport`, `utils`, `test_support`, runtime-only prelude additions, and duration convenience exports
- **AND** the documented optional `std`-gated path families `layer` and `global-discovery`
- **WHEN** the boundary is reviewed
- **THEN** the alloc-only families SHALL remain available without `std`
- **AND** the shell families plus optional `std`-gated path families SHALL remain gated or shell-only as documented for this first cut

#### Scenario: Representative std consumers remain supported

- **GIVEN** the representative workspace consumers `crates/aspen-cluster` and `crates/aspen-cli`
- **WHEN** they opt into the documented `std` compatibility path
- **THEN** they MUST continue to compile against `aspen-core`
- **AND** the verification matrix MUST keep those consumer slices green

#### Scenario: Compile-slice verification is reviewable

- **GIVEN** the alloc-only crate, alloc-safe `sql`, smoke consumer, and representative std consumers
- **WHEN** their compile slices are verified for review
- **THEN** the exact commands and results SHALL be saved under `openspec/changes/no-std-aspen-core/evidence/`
- **AND** `openspec/changes/no-std-aspen-core/verification.md` SHALL identify which artifact proves each slice

## MODIFIED Requirements

### Requirement: Functional Core, Imperative Shell

The system SHALL separate pure business logic from runtime shells. `aspen-core` SHALL expose its functional core through alloc-safe modules that can compile without `std`, while I/O, filesystem access, process/environment inspection, network transport integration, async-runtime helpers, synchronization adapters, and other side-effecting facilities SHALL live behind explicit `std`-only features or companion shell crates. Pure functions SHALL receive time, randomness, and configuration as explicit parameters.

#### Scenario: Verified function purity

- **GIVEN** a function in a `verified/` module or other alloc-only core module
- **WHEN** it is invoked
- **THEN** it SHALL perform no I/O, no async operations, and no system calls
- **AND** it SHALL be deterministic for the same inputs

#### Scenario: Shell APIs do not leak into alloc-only core

- **GIVEN** a public API that requires filesystem access, environment inspection, process IDs, network transport, or runtime synchronization
- **WHEN** that API is exposed to callers
- **THEN** it SHALL be provided only through an explicit `std`-only boundary
- **AND** the alloc-only `aspen-core` surface SHALL NOT re-export it unconditionally

#### Scenario: Pure time-dependent logic uses no-std-safe inputs

- **GIVEN** a core algorithm that reasons about elapsed time, retry windows, or breaker state
- **WHEN** it is exposed from the alloc-only surface
- **THEN** it SHALL accept explicit numeric or no-std-safe time inputs
- **AND** it SHALL NOT require `std::time::Instant`, `tokio::time`, or other runtime-owned clock handles

#### Scenario: Pure logic accepts explicit randomness and configuration inputs

- **GIVEN** a core algorithm that depends on randomness or configuration
- **WHEN** it is exposed from the alloc-only surface
- **THEN** it SHALL receive randomness and configuration through explicit parameters or value objects
- **AND** it SHALL NOT read ambient environment, process-global state, or hidden runtime context

#### Scenario: Refactored pure logic keeps regression coverage

- **GIVEN** logic moved from a `std`-bound API into the alloc-only functional core
- **WHEN** that logic is refactored to use explicit primitive or value inputs
- **THEN** existing positive behavior coverage SHALL remain
- **AND** negative or boundary-case regression tests SHALL cover the converted pure entrypoints
