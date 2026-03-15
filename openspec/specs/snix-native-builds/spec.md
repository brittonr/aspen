## ADDED Requirements

### Requirement: Execute builds via BuildService trait

The CI nix executor SHALL support executing builds through snix-build's `BuildService` trait as an alternative to shelling out to `nix build`.

#### Scenario: Successful in-process build

- **WHEN** a CI job specifies a nix build with `snix-build` feature enabled
- **AND** the flake evaluates to a valid derivation
- **THEN** the executor SHALL convert the derivation to a `BuildRequest` and execute it via `BuildService::do_build`
- **AND** return the output paths from the `BuildResult`

#### Scenario: Build failure

- **WHEN** a `BuildService::do_build` returns an error
- **THEN** the executor SHALL report the failure with the build log and record it in the failure cache

### Requirement: Bubblewrap sandbox on Linux

On Linux systems, the executor SHALL use bubblewrap (`bwrap`) namespace isolation for build sandboxing when the `snix-build` feature is enabled.

#### Scenario: Sandboxed build with network disabled

- **WHEN** a build request has `network_access = false` (default)
- **THEN** the sandbox SHALL unshare the network namespace
- **AND** the build process SHALL NOT have network access

#### Scenario: Sandboxed build with network enabled

- **WHEN** a build request has `network_access = true` (fixed-output derivation)
- **THEN** the sandbox SHALL allow network access

### Requirement: OCI container sandbox fallback

When bubblewrap is not available, the executor SHALL support OCI container-based sandboxing as an alternative.

#### Scenario: OCI fallback when bwrap missing

- **WHEN** `bwrap` is not found on PATH
- **AND** an OCI runtime is available
- **THEN** the executor SHALL create an OCI spec and execute the build in a container

### Requirement: Subprocess fallback

The executor SHALL retain the `nix build` subprocess execution path behind a `nix-cli-fallback` feature flag.

#### Scenario: Fallback to nix CLI

- **WHEN** the `nix-cli-fallback` feature is enabled
- **AND** the `snix-build` feature is disabled or the build service fails to initialize
- **THEN** the executor SHALL fall back to spawning `nix build` as a subprocess

### Requirement: Build inputs from Aspen store

The `BuildService` SHALL resolve build inputs from Aspen's `BlobService` and `DirectoryService`, not from the local filesystem `/nix/store`.

#### Scenario: Input resolution from cluster store

- **WHEN** a `BuildRequest` references input paths
- **THEN** the executor SHALL resolve them via Aspen's castore services
- **AND** mount them into the sandbox at the appropriate paths

### Requirement: Build outputs uploaded to Aspen store

After a successful build, the executor SHALL upload output paths to Aspen's decomposed storage using the existing SNIX upload pipeline.

#### Scenario: Output upload after build

- **WHEN** a build completes with output paths
- **AND** `publish_to_cache` is true
- **THEN** the executor SHALL create NAR archives of the outputs and ingest them into BlobService/DirectoryService/PathInfoService
