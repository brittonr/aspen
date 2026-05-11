# snix-build-default Specification

## Purpose

Defines the Snix Build Default capability requirements preserved by Aspen's archived OpenSpec records, including all CI-capable binaries compile with snix-build, snix build infrastructure propagated to all CI binaries, redundant binary variants removed.

## Requirements

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

### Requirement: Offline deterministic CI dogfood full-loop fixture [r[snix-build-default.ci-dogfood-full-loop-offline]]

Dogfood VM tests that exercise full CI pipeline Nix jobs MUST use deterministic flake inputs that can be resolved without public registry, DNS, substituter, or lock-file update access from inside the guest.

#### Scenario: Full-loop fixture avoids public registry resolution [r[snix-build-default.ci-dogfood-full-loop-offline.no-registry]]

- GIVEN `ci-dogfood-full-loop-test` has pushed its sample repository into Forge
- WHEN the CI `syntax-check`, `build-and-test`, or `unit-tests` job runs `nix build` in the checked-out repository
- THEN the flake input graph SHALL resolve from a store-resident, copied, or otherwise test-local source
- AND the job log SHALL NOT contain attempts to fetch `https://channels.nixos.org/flake-registry.json`
- AND the job SHALL NOT require updating a missing `flake.lock` through public `nixpkgs` registry lookup

#### Scenario: Fixture input failure is classified separately from CI orchestration failure [r[snix-build-default.ci-dogfood-full-loop-offline.failure-classification]]

- GIVEN a full-loop dogfood VM run fails before any staged job can build the sample artifact
- WHEN the failure log contains registry, DNS, lock-update, or external input resolution errors
- THEN the result SHALL be reported as a fixture determinism failure rather than as accepted evidence about stage dependency ordering
- AND the remediation SHALL keep the pipeline proof narrow instead of broadening support claims for full self-hosting acceptance

### Requirement: CI dogfood full-loop stage proof remains intact [r[snix-build-default.ci-dogfood-full-loop-stage-proof]]

The deterministic fixture fix MUST preserve the full-loop acceptance intent: Forge push triggers a CI run; jobs are assigned to the expected stages; dependency ordering prevents later stages from running before earlier stages succeed; and the built artifact is retrieved and executed by the test.

#### Scenario: Three-stage pipeline succeeds with local inputs [r[snix-build-default.ci-dogfood-full-loop-stage-proof.success]]

- GIVEN the full-loop fixture uses only deterministic local/store inputs
- WHEN `ci-dogfood-full-loop-test` runs to completion
- THEN `format-check` and `syntax-check` SHALL complete successfully in the `check` stage
- AND `build-and-test` SHALL complete successfully only after the `check` stage succeeds
- AND `unit-tests` SHALL complete successfully only after the `build` stage succeeds
- AND the test SHALL verify the CI-built artifact or equivalent stage output without relying on external network resolution

#### Scenario: Feature-complete CI node is required [r[snix-build-default.ci-dogfood-full-loop-stage-proof.features]]

- GIVEN a VM test submits `type = 'nix` CI jobs through `aspen-node`
- WHEN the test's node package is selected in `flake.nix`
- THEN the package feature set SHALL include CI job execution support and the expected Nix build fallback/native path features (`ci`, `shell-worker`, `snix`, `snix-build`, and `nix-cli-fallback` where subprocess fallback is part of the acceptance path)
