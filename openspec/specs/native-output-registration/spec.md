## ADDED Requirements

### Requirement: Build outputs registered at derivation output path

After a native bwrap build completes, the output SHALL be accessible at its derivation output path (`/nix/store/<hash>-<name>`) on the host filesystem, regardless of whether the nix store is mounted read-only.

#### Scenario: Output registered via nix daemon on read-only store

- **WHEN** LocalStoreBuildService completes a bwrap build with output at a sandbox temp path
- **AND** the nix store overlay is read-only (ProtectSystem=strict)
- **THEN** the output is registered in `/nix/store` at the derivation's expected output path via the nix daemon protocol
- **AND** the binary at the output path is executable

#### Scenario: Output already exists in store

- **WHEN** the derivation output path already exists in `/nix/store` (e.g. from a previous build or substituter)
- **THEN** the registration step is skipped
- **AND** no error is raised

#### Scenario: Daemon unavailable fallback

- **WHEN** the nix daemon is not running or the daemon protocol call fails
- **THEN** the build output is still available in PathInfoService (castore)
- **AND** a warning is logged indicating the local store registration failed
- **AND** the build is NOT marked as failed (castore is the primary storage)

### Requirement: Unit test for output registration

Output registration logic SHALL have a unit test that validates path construction and registration flow without requiring a running nix daemon or VM.

#### Scenario: Test output path computation

- **WHEN** a BuildResult contains an output at sandbox path `/tmp/builds/<uuid>/scratches/nix/store/<hash>-<name>`
- **THEN** the computed target path is `/nix/store/<hash>-<name>`
- **AND** the store basename extraction matches the derivation output

### Requirement: Integration test for bwrap build + registration

A cargo integration test (marked `#[ignore]`, requiring bubblewrap) SHALL build a trivial derivation via LocalStoreBuildService and verify the output exists at the expected `/nix/store` path.

#### Scenario: Trivial derivation output accessible

- **WHEN** a trivial derivation (`echo hello > $out`) is built via LocalStoreBuildService
- **THEN** the output file exists at the derivation's expected `/nix/store/<hash>-<name>` path
- **AND** the file contents match the expected output
