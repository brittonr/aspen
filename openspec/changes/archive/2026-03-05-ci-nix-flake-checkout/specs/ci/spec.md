## MODIFIED Requirements

### Requirement: SNIX cache upload from CI builds

The NixBuildWorker SHALL use SNIX services (BlobService, DirectoryService, PathInfoService) for cache uploads when available, in addition to the legacy blob store path.

#### Scenario: SNIX services wired in node binary

- **WHEN** the node binary starts with `snix` feature enabled and `config.snix.is_enabled` is true
- **THEN** `NixBuildWorkerConfig` SHALL receive non-None SNIX service references
- **AND** `upload_store_paths_snix()` SHALL be invoked for store path uploads

#### Scenario: SNIX services unavailable

- **WHEN** the node binary starts without `snix` feature or with SNIX disabled
- **THEN** `NixBuildWorkerConfig` SHALL have `snix_*_service: None`
- **AND** the legacy `upload_store_paths()` path SHALL be used as fallback

## ADDED Requirements

### Requirement: Pipeline config SHALL support publish_to_cache

The Nickel CI config schema and `JobConfig` struct SHALL include a `publish_to_cache` boolean field that controls whether build outputs are uploaded to the Nix binary cache.

#### Scenario: Default publish_to_cache

- **WHEN** a job config does not specify `publish_to_cache`
- **THEN** it SHALL default to `true`

#### Scenario: Disable cache publishing

- **WHEN** a job config sets `publish_to_cache = false`
- **THEN** the NixBuildWorker SHALL skip store path uploads for that job
