## ADDED Requirements

### Requirement: Wire crates default to alloc-safe production surfaces

The shared wire crates `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol` MUST keep their production modules alloc-safe by default and confine std-only helpers to tests, dev-dependencies, or shell consumers.

ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces
#### Scenario: Client API production modules avoid std-only helpers
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-production-modules-avoid-std-only-helpers

- **GIVEN** `aspen-client-api` with its default production feature set
- **WHEN** the crate is built outside tests
- **THEN** it MUST compile with `alloc` and no unconditional `std` imports in production modules
- **AND** production message modules MUST use alloc-safe collections and postcard helpers instead of `std::collections` or `postcard::to_stdvec`
- **AND** no additional std-only feature opt-in SHALL be required for that default production build

#### Scenario: Client API postcard tests use alloc-safe serializers
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.client-api-postcard-tests-use-alloc-safe-serializers

- **GIVEN** `aspen-client-api` postcard regression tests
- **WHEN** the crate validates wire compatibility after the alloc-safe refactor
- **THEN** those tests MUST use alloc-safe postcard serializers rather than production std helpers
- **AND** the saved wire-compatibility artifacts SHALL make that serializer choice reviewable

#### Scenario: Protocol crates keep JSON helpers test-only
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.protocol-crates-keep-json-helpers-test-only

- **GIVEN** `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol`
- **WHEN** Cargo resolves their production dependencies
- **THEN** `serde_json` MUST remain a dev-dependency or `#[cfg(test)]`-only helper
- **AND** production modules MUST remain free of std-only JSON helpers

#### Scenario: Wire crates reject forbidden std helpers in production modules
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-crates-reject-forbidden-std-helpers

- **GIVEN** `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol` production modules
- **WHEN** deterministic negative verification inspects those sources for regressions
- **THEN** `std::collections` and `postcard::to_stdvec` MUST remain absent from production wire modules
- **AND** production `serde_json` helpers MUST remain absent outside tests
- **AND** `python3 scripts/check-foundation-wire-source-audits.py --mode wire` SHALL scan `crates/aspen-client-api/src/`, `crates/aspen-coordination-protocol/src/`, `crates/aspen-jobs-protocol/src/`, and `crates/aspen-forge-protocol/src/`, treat `tests/` trees and `#[cfg(test)]` code as test-only, and fail if those forbidden helpers reappear in production code

#### Scenario: Wire compatibility survives alloc-safe refactor
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-survives-alloc-safe-refactor

- **GIVEN** the append-only postcard discriminant contract for `ClientRpcRequest` and `ClientRpcResponse` plus existing runtime consumers
- **WHEN** the wire crates are refactored to alloc-safe production surfaces
- **THEN** postcard discriminants and default feature behavior MUST remain unchanged
- **AND** a deterministic baseline artifact for `ClientRpcRequest` and `ClientRpcResponse` postcard encodings SHALL remain byte-for-byte unchanged unless a future compatibility change explicitly updates that baseline
- **AND** that baseline SHALL include at least one default-production representative encoding for every enum variant of `ClientRpcRequest` and `ClientRpcResponse`, keyed by variant name so reviewers can detect reordering or renumbering deterministically
- **AND** the baseline harness SHALL derive canonical per-variant sample payloads from fixed rules only: stable variant-name-derived strings, fixed integer values, fixed byte sequences, deterministic single-entry collections, and recursively canonical nested values with no randomness, clock reads, or environment-dependent data
- **AND** the baseline harness SHALL deterministically enumerate the live `ClientRpcRequest` and `ClientRpcResponse` variant sets and fail if any current variant lacks a baseline entry
- **AND** representative runtime consumers `aspen-cluster`, `aspen-client`, `aspen-cli`, `aspen-rpc-handlers`, and root `aspen` via `--no-default-features --features node-runtime` MUST continue to compile

#### Scenario: Protocol dependency verification is reviewable
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.protocol-dependency-verification-is-reviewable

- **GIVEN** the alloc-safe production contract for `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol`
- **WHEN** dependency and compile rails are saved for review
- **THEN** the exact commands and results for `cargo check -p aspen-client-api`, `cargo check -p aspen-client-api --no-default-features`, `cargo check -p aspen-client-api --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-coordination-protocol`, `cargo check -p aspen-coordination-protocol --no-default-features`, `cargo check -p aspen-coordination-protocol --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-jobs-protocol`, `cargo check -p aspen-jobs-protocol --no-default-features`, `cargo check -p aspen-jobs-protocol --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-forge-protocol`, `cargo check -p aspen-forge-protocol --no-default-features`, `cargo check -p aspen-forge-protocol --no-default-features --target wasm32-unknown-unknown`, paired `cargo tree -p <crate> -e normal`, paired `cargo tree -p <crate> -e features`, `python3 scripts/check-foundation-wire-deps.py --mode wire`, and `python3 scripts/check-foundation-wire-source-audits.py --mode wire` SHALL be saved under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** the saved `python3 scripts/check-foundation-wire-deps.py --mode wire` artifact SHALL fail unless `aspen-client-api`, `aspen-coordination-protocol`, `aspen-jobs-protocol`, and `aspen-forge-protocol` exclude production `serde_json`, `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `redb`, `libc`, `chrono`, `tracing`, `anyhow`, `n0-future`, `aspen-disk`, `aspen-time`, `aspen-layer`, and `rand`
- **AND** the saved `python3 scripts/check-foundation-wire-source-audits.py --mode wire` artifact SHALL prove production modules no longer import `std::collections`, `postcard::to_stdvec`, or production `serde_json` helpers outside tests
- **AND** the saved `--no-default-features --target wasm32-unknown-unknown` artifacts SHALL be treated as the deterministic alloc-only / `no_std` proof that these wire crates do not require unconditional `std` imports in production modules
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each protocol-dependency claim

#### Scenario: Wire compatibility verification is reviewable
ID: architecture.modularity.alloc-safe-wire-crates-default-to-alloc-safe-production-surfaces.wire-compatibility-verification-is-reviewable

- **GIVEN** the wire-compatibility contract for postcard discriminants, default feature behavior, and representative runtime consumer builds
- **WHEN** those checks are saved for review
- **THEN** the exact commands and results for `cargo test -p aspen-client-api`, `cargo test -p aspen-client-api client_rpc_postcard_baseline -- --nocapture`, plus representative consumer compile checks for `aspen-cluster`, `aspen-client`, `aspen-cli`, `aspen-rpc-handlers`, and root `aspen` via `--no-default-features --features node-runtime` SHALL be saved under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** a deterministic comparison artifact at `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/client-rpc-postcard-baseline.json` SHALL record at least one variant-keyed default-production postcard encoding for every `ClientRpcRequest` and `ClientRpcResponse` enum variant
- **AND** the saved `client_rpc_postcard_baseline` test output SHALL show both that the baseline covers every live `ClientRpcRequest` / `ClientRpcResponse` variant and that the current wire behavior matches that baseline artifact
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each wire-compatibility claim

## MODIFIED Requirements

### Requirement: Acyclic no-std core boundary

Aspen MUST maintain an acyclic dependency boundary where the alloc-only `aspen-core` surface depends only on alloc-safe leaf crates, and `std`-bound runtime shells depend on that core rather than the reverse.

For this requirement:

- **alloc-safe** means either (a) an approved alloc-only direct prerequisite listed in this spec and configured with alloc-only features, or (b) a transitive dependency reachable only through those approved prerequisites and verified to avoid filesystem, process, thread, network, async-runtime, or storage-engine requirements.
- **leaf crate** means an alloc-safe dependency that provides shared types, codecs, or deterministic helpers and does not depend on Aspen shell crates or app bundles.
- **direct prerequisite** means a dependency selected directly by `crates/aspen-core/Cargo.toml` for the alloc-only build, not an arbitrary transitive dependency of another enabled crate.

ID: architecture.modularity.acyclic-no-std-core-boundary
#### Scenario: Runtime shells depend outward on core
ID: architecture.modularity.acyclic-no-std-core-boundary.runtime-shells-depend-outward-on-core

- **GIVEN** a shell facility such as transport, registry state, simulation persistence, Redb table definitions, or runtime convenience implementations
- **WHEN** its module or crate is composed
- **THEN** it MUST depend on the alloc-only core contracts and types
- **AND** the alloc-only core surface MUST NOT depend back on that shell facility

#### Scenario: Acyclic boundary proof is reviewable
ID: architecture.modularity.acyclic-no-std-core-boundary.acyclic-boundary-proof-is-reviewable

- **GIVEN** the alloc-only core and `std` shell boundary
- **WHEN** that boundary is verified for review
- **THEN** the exact commands and results for `python3 scripts/check-aspen-core-no-std-surface.py` and `python3 scripts/check-aspen-core-no-std-boundary.py` SHALL be saved under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** saved inventory, export-map, and source-audit artifacts such as `surface-inventory.md`, `export-map.md`, and `source-audit.txt` under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/` SHALL show that alloc-only modules and exports do not depend on shell-only modules or crates
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifacts prove the boundary remains acyclic

#### Scenario: Pure consumers avoid runtime shells
ID: architecture.modularity.acyclic-no-std-core-boundary.pure-consumers-avoid-runtime-shells

- **GIVEN** a workspace crate or external consumer that needs only Aspen core contracts
- **WHEN** it depends on the alloc-only `aspen-core` surface
- **THEN** Cargo resolution MUST succeed without enabling Aspen runtime shell features
- **AND** the consumer MUST NOT need to import shell modules just to reach foundational types and traits

#### Scenario: Leaf crates keep alloc-safe production surfaces
ID: architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-keep-alloc-safe-production-surfaces

- **GIVEN** alloc-only leaf crates on the core path such as `aspen-storage-types` and `aspen-traits`
- **WHEN** they are built for production use
- **THEN** their production modules MUST NOT require `redb`, `libc`, `std::sync::Arc`, or other shell/runtime crates
- **AND** any Redb-specific or std-specific conveniences MUST live in `aspen-core-shell` or test-only modules instead while preserving equivalent shell-facing behavior when those conveniences remain supported
- **AND** no separate shell-side trait shim is required for this cut because `alloc::sync::Arc` preserves the supported `aspen-traits` convenience behavior

#### Scenario: Leaf crates default to alloc-safe builds
ID: architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-default-to-alloc-safe-builds

- **GIVEN** `aspen-storage-types` and `aspen-traits`
- **WHEN** each crate is built with its default production configuration outside tests
- **THEN** it MUST compile with an alloc-safe production surface
- **AND** no std-only feature opt-in SHALL be required for that default build

#### Scenario: Leaf crates reject forbidden runtime helpers in production modules
ID: architecture.modularity.acyclic-no-std-core-boundary.leaf-crates-reject-forbidden-runtime-helpers

- **GIVEN** `aspen-storage-types` and `aspen-traits` production modules
- **WHEN** deterministic negative verification inspects those sources for regressions
- **THEN** `redb::TableDefinition`, `TableDefinition::new`, `redb::Database`, `redb::ReadTransaction`, and `redb::WriteTransaction` MUST remain absent from `aspen-storage-types`
- **AND** `std::sync::Arc` MUST remain absent from `aspen-traits` production modules
- **AND** `python3 scripts/check-foundation-wire-source-audits.py --mode leaf` SHALL scan `crates/aspen-storage-types/src/` and `crates/aspen-traits/src/`, treat `tests/` trees and `#[cfg(test)]` code as test-only, and fail if those forbidden helpers reappear outside test-only code

#### Scenario: Leaf-crate verification is reviewable
ID: architecture.modularity.acyclic-no-std-core-boundary.leaf-crate-verification-is-reviewable

- **GIVEN** the alloc-safe leaf-crate contract for `aspen-storage-types` and `aspen-traits`
- **WHEN** crate-level compile and dependency rails are saved for review
- **THEN** the exact commands and results for `cargo check -p aspen-storage-types`, `cargo check -p aspen-storage-types --no-default-features`, `cargo check -p aspen-storage-types --no-default-features --target wasm32-unknown-unknown`, `cargo check -p aspen-traits`, `cargo check -p aspen-traits --no-default-features`, `cargo check -p aspen-traits --no-default-features --target wasm32-unknown-unknown`, `cargo tree -p aspen-storage-types -e normal`, `cargo tree -p aspen-storage-types -e features`, `cargo tree -p aspen-traits -e normal`, `cargo tree -p aspen-traits -e features`, `cargo tree -p aspen-traits -e features -i aspen-cluster-types`, `python3 scripts/check-foundation-wire-deps.py --mode leaf`, and `python3 scripts/check-foundation-wire-source-audits.py --mode leaf` SHALL be saved under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** the saved `python3 scripts/check-foundation-wire-deps.py --mode leaf` artifact SHALL fail unless `aspen-storage-types` excludes `redb` / `libc` from its production graph and the `aspen-traits` → `aspen-cluster-types` path keeps alloc-only feature settings, does not re-enable default features, and does not leak `iroh` or `iroh-base` through that path
- **AND** the saved `python3 scripts/check-foundation-wire-source-audits.py --mode leaf` artifact SHALL prove `aspen-storage-types` production modules no longer expose `redb::TableDefinition`, `TableDefinition::new`, `redb::Database`, `redb::ReadTransaction`, or `redb::WriteTransaction` and `aspen-traits` production modules no longer import `std::sync::Arc`
- **AND** the saved `--no-default-features --target wasm32-unknown-unknown` artifacts SHALL be treated as the deterministic alloc-only / `no_std` proof that these leaf crates do not require unconditional `std` imports in production modules
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each leaf-crate claim

### Requirement: Feature bundles are explicit and bounded

The build feature graph SHALL distinguish alloc-only core surfaces, `std`-only shell features, and higher-level convenience bundles so enabling foundational Aspen contracts does not silently pull runtime dependencies.

ID: architecture.modularity.feature-bundles-are-explicit-and-bounded
#### Scenario: Alloc-only core excludes runtime shells
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.alloc-only-core-excludes-runtime-shells

- **GIVEN** a contributor or downstream crate enables only the alloc-only `aspen-core` surface
- **WHEN** Cargo resolves the feature graph
- **THEN** the direct prerequisite set SHALL be limited to `aspen-constants`, `aspen-hlc`, `aspen-cluster-types`, `aspen-kv-types`, `aspen-traits`, `aspen-storage-types`, `async-trait`, `serde`, `bincode`, `base64`, `hex`, `snafu`, and `thiserror`
- **AND** those dependencies SHALL be configured with alloc-only feature settings, using `default-features = false` wherever a default feature set exists and enabling only features needed by the alloc-only core surface
- **AND** `std`-only shell facilities such as transport, simulation, Redb table definitions, or runtime registry helpers SHALL remain disabled

#### Scenario: Std compatibility is an explicit opt-in
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.std-compatibility-is-an-explicit-opt-in

- **GIVEN** a build profile requires shell helpers on top of the alloc-only core
- **WHEN** the profile is expressed through the alloc-only package `aspen-core` and the companion shell package `aspen-core-shell`
- **THEN** `aspen-core` SHALL keep an empty default feature set and alloc-safe `sql` as its only additional surface for this seam
- **AND** shell helpers SHALL be opted into by depending on `aspen-core-shell`, whose documented feature families for this seam are the default shell base plus `layer`, `global-discovery`, and optional shell `sql`
- **AND** documentation at `docs/no-std-core.md` and compile-slice tests SHALL identify what each package/feature path enables

#### Scenario: Dependency boundary is checked deterministically
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.dependency-boundary-is-checked-deterministically

- **GIVEN** the alloc-only `aspen-core` feature set
- **WHEN** its dependency graph is checked by the documented dependency-boundary verification commands
- **THEN** the direct prerequisite assertion from `cargo tree -p aspen-core --no-default-features -e normal --depth 1` SHALL match the alloc-only prerequisite set named above
- **AND** the saved full-graph artifact from `cargo tree -p aspen-core --no-default-features -e normal` SHALL show that every transitive dependency is reachable only through those approved prerequisites using alloc-safe feature settings
- **AND** the saved feature-resolution artifact from `cargo tree -p aspen-core --no-default-features -e features` SHALL make transitive default-feature leaks reviewable
- **AND** every transitive dependency SHALL appear in the approved alloc-safe transitive allowlist for this change at `scripts/aspen-core-no-std-transitives.txt` or cause verification to fail
- **AND** the full graph SHALL exclude `aspen-layer`, `aspen-time`, `aspen-disk`, `anyhow`, `n0-future`, `tokio`, `tokio-util`, `iroh`, `iroh-base`, `iroh-blobs`, `rand`, `tracing`, `chrono`, `serde_json`, `redb`, and `libc`
- **AND** the saved verification artifact SHALL identify the exact commands and results used for review under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves the dependency-boundary claim

#### Scenario: Feature-topology verification is reviewable
ID: architecture.modularity.feature-bundles-are-explicit-and-bounded.feature-topology-verification-is-reviewable

- **GIVEN** the public package/feature contract for alloc-only `aspen-core` and companion `aspen-core-shell`
- **WHEN** that contract is verified for review
- **THEN** the exact commands and results for the required `aspen-core` compile slices (`default`, `--no-default-features`, and alloc-safe `sql`) plus the required `aspen-core-shell` compile slices (default shell base, `layer`, `global-discovery`, and optional shell `sql`) SHALL be saved under `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/evidence/`
- **AND** `openspec/changes/archive/2026-04-25-extend-no-std-foundation-and-wire/verification.md` SHALL identify which artifact proves each feature topology claim
