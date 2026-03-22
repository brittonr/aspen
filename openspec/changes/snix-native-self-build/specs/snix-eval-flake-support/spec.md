## ADDED Requirements

### Requirement: Generate flake.lock when missing

When snix-eval encounters a flake directory without `flake.lock`, it SHALL run `nix flake lock` to generate it before attempting evaluation.

#### Scenario: CI checkout without flake.lock

- **WHEN** a CI pipeline checks out a repository containing `flake.nix` but no `flake.lock`
- **AND** the native build path attempts in-process flake eval
- **THEN** `nix flake lock` is invoked to generate `flake.lock`
- **AND** the eval proceeds with the generated lock file
- **AND** the lock generation time is logged

#### Scenario: flake.lock already exists

- **WHEN** a flake directory already contains `flake.lock`
- **THEN** no lock generation subprocess is invoked
- **AND** eval proceeds directly

#### Scenario: Lock generation fails

- **WHEN** `nix flake lock` fails (no network, invalid flake.nix, etc.)
- **THEN** the eval falls through to the `nix eval` subprocess fallback
- **AND** a warning is logged with the failure reason

### Requirement: Unit test for flake.lock detection

Flake lock detection and generation logic SHALL have unit tests that run without nix or network access.

#### Scenario: Test lock file detection

- **WHEN** a temp directory contains `flake.nix` but no `flake.lock`
- **THEN** `needs_flake_lock()` returns true

#### Scenario: Test lock file present

- **WHEN** a temp directory contains both `flake.nix` and `flake.lock`
- **THEN** `needs_flake_lock()` returns false

### Requirement: Integration test for eval with lock generation

A cargo integration test SHALL verify that a flake without `flake.lock` can be evaluated after lock generation, producing a valid derivation path.

#### Scenario: Eval trivial flake after lock generation

- **WHEN** a trivial flake (no nixpkgs, `derivation { name="test"; system="x86_64-linux"; builder="/bin/sh"; args=["-c" "echo test > $out"]; }`) is evaluated
- **AND** `flake.lock` is generated from `inputs = {}`
- **THEN** `resolve_drv_path` returns a valid `/nix/store/*.drv` path

### Requirement: Self-build VM test uses native pipeline

The `ci-dogfood-self-build-test` SHALL use `full-aspen-node-plugins-snix-build` and verify that at least the `build-and-test` job completes via the native bwrap path (not `nix build` subprocess).

#### Scenario: Self-build with native pipeline

- **WHEN** the self-build VM test runs with snix-build enabled
- **THEN** the `build-and-test` CI job builds via LocalStoreBuildService (bwrap)
- **AND** the output binary is runnable and produces correct output
- **AND** the build output is stored in PathInfoService

#### Scenario: Cargo-check job fallback acceptable

- **WHEN** the `cargo-check` job's eval times out (first nixpkgs fetch in VM)
- **THEN** it falls back to `nix build` subprocess
- **AND** the test still passes (at most 1 fallback allowed)
