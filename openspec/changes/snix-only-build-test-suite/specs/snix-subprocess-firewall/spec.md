## ADDED Requirements

### Requirement: Test harness blocks nix CLI subprocess spawning

The `test_support` module SHALL provide a mechanism that makes it impossible for any `nix`, `nix-store`, or `nix-instantiate` subprocess to execute during a test. Any attempt to spawn these binaries SHALL cause an immediate, loud test failure rather than silent fallback.

#### Scenario: Unit test helper returns sanitised environment

- **WHEN** a test calls `test_support::no_nix_env()`
- **THEN** the returned `CommandEnvGuard` provides a `PATH` value containing only safe binaries (`cp`, `sh`, `echo`, `env`, `cat`, `mkdir`, `rm`, `chmod`, `ls`) and no `nix*` binaries

#### Scenario: FakeBuildService available for pipeline tests

- **WHEN** a test constructs a `FakeBuildService` with a canned `BuildResult`
- **THEN** the service implements `snix_build::buildservice::BuildService` and returns the configured result from `do_build()` without spawning any subprocess

#### Scenario: TestSnixStack wires up in-memory services

- **WHEN** a test constructs a `TestSnixStack`
- **THEN** it provides `Arc<dyn BlobService>`, `Arc<dyn DirectoryService>`, `Arc<dyn PathInfoService>`, and a `NativeBuildService` backed by `FakeBuildService` — all in-memory, no disk or network I/O

### Requirement: Subprocess firewall is test-only

The subprocess firewall module SHALL be gated behind `#[cfg(test)]`. It SHALL NOT appear in production binary compilation. It SHALL NOT add any runtime overhead or dependencies to non-test builds.

#### Scenario: Production build excludes test_support

- **WHEN** `cargo build --release -p aspen-ci-executor-nix --features snix-build` is run
- **THEN** the `test_support` module is not compiled and no test-only dependencies are linked
