## ADDED Requirements

### Requirement: All CI-capable binaries compile with snix-build

Every aspen-node binary variant that includes CI features (`ci` feature flag) SHALL also compile with `snix,snix-build` features enabled.

#### Scenario: full-aspen-node-plugins includes snix-build

- **WHEN** `bins.full-aspen-node-plugins` is built via `nix build`
- **THEN** its `cargoExtraArgs` SHALL include `snix,snix-build` in the feature list
- **AND** the binary SHALL have `NativeBuildService` compiled in

#### Scenario: ci-aspen-node-plugins includes snix-build

- **WHEN** `bins.ci-aspen-node-plugins` is built via `nix build`
- **THEN** its `cargoExtraArgs` SHALL include `snix,snix-build` in the feature list

#### Scenario: dogfood-local aspenNode includes snix-build

- **WHEN** the `aspenNode` binary (u2n default) is built
- **THEN** its feature list SHALL include `snix,snix-build`

### Requirement: Snix build infrastructure propagated to all CI binaries

All CI-capable binary definitions SHALL include the build environment needed for snix compilation.

#### Scenario: PROTO_ROOT set for protobuf codegen

- **WHEN** a CI-capable binary is built
- **THEN** the `PROTO_ROOT` environment variable SHALL point to the snix source tree
- **AND** snix-castore's `build.rs` SHALL find its proto files

#### Scenario: SNIX_BUILD_SANDBOX_SHELL set

- **WHEN** a CI-capable binary is built
- **THEN** the `SNIX_BUILD_SANDBOX_SHELL` environment variable SHALL point to busybox-sandbox-shell

#### Scenario: Vendor directory includes real snix source

- **WHEN** a CI-capable binary is built via crane
- **THEN** the cargo vendor directory SHALL contain the real snix crate source (not stubs)
- **AND** `fullSrcWithSnix` SHALL be used as the source tree

### Requirement: Redundant binary variants removed

Binary variants that exist only as intermediate snix feature combinations SHALL be removed.

#### Scenario: No snix-without-snix-build variant

- **WHEN** flake.nix binary definitions are enumerated
- **THEN** there SHALL be no binary with `snix` feature but without `snix-build` feature (when it also has `ci`)

#### Scenario: full-aspen-node-plugins-snix-build is an alias

- **WHEN** `bins.full-aspen-node-plugins-snix-build` is referenced
- **THEN** it SHALL resolve to the same derivation as `bins.full-aspen-node-plugins`

### Requirement: Native build path exercised by default in dogfood tests

All NixOS VM tests that perform nix builds SHALL use a binary with snix-build enabled, exercising the native `BuildService` path before falling back to subprocess.

#### Scenario: Dogfood test uses native build

- **WHEN** `ci-dogfood-test` executes a nix build job
- **THEN** the executor SHALL attempt native build via `NativeBuildService` first
- **AND** the executor log SHALL contain evidence of the native build attempt

#### Scenario: Subprocess fallback still works

- **WHEN** the native build path fails (e.g., unsupported builtin, missing bwrap)
- **THEN** the executor SHALL fall back to `nix build` subprocess
- **AND** the build SHALL still succeed
