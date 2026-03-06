## ADDED Requirements

### Requirement: Nix builds publish to distributed cache

After a CI Nix build completes successfully, the system SHALL upload output store paths to the cluster's distributed Nix binary cache via the snix backend. Published paths SHALL be immediately available via the cache gateway's HTTP substituter interface.

#### Scenario: Successful build publishes store paths

- **WHEN** a `NixBuildWorker` completes a Nix flake build with output store paths
- **THEN** each output store path SHALL be NAR-archived and uploaded to the snix `BlobService`
- **AND** a `PathInfo` entry SHALL be registered in the snix `PathInfoService`
- **AND** the store path SHALL be retrievable via `GET /<hash>.narinfo` on the cache gateway

#### Scenario: Build with multiple outputs

- **WHEN** a build produces outputs `out`, `dev`, and `doc`
- **THEN** all three store paths SHALL be published to the cache
- **AND** each SHALL have a separate `PathInfo` entry with correct references

#### Scenario: Publish failure does not fail the build

- **WHEN** NAR upload or PathInfo registration fails (e.g., network error, leader unavailable)
- **THEN** the build job SHALL still be marked as successful
- **AND** the publish failure SHALL be logged as a warning
- **AND** the failed paths SHALL be retried on next build (idempotent)

#### Scenario: Duplicate publish is idempotent

- **WHEN** the same store path is published twice (e.g., retry after partial failure)
- **THEN** the second publish SHALL succeed without error
- **AND** the `PathInfo` entry SHALL be unchanged

### Requirement: Cache publish is configurable

The system SHALL allow CI pipeline configs to control cache publishing behavior.

#### Scenario: Cache publish enabled (default)

- **WHEN** a Nix build job does not specify `publish_to_cache`
- **THEN** cache publishing SHALL be enabled by default

#### Scenario: Cache publish disabled

- **WHEN** a Nix build job specifies `publish_to_cache = false`
- **THEN** no store paths SHALL be published to the cache after the build

#### Scenario: Selective output publishing

- **WHEN** a Nix build job specifies `cache_outputs = ["out"]`
- **THEN** only the `out` output SHALL be published
- **AND** `dev` and `doc` outputs SHALL NOT be published
