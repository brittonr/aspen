## MODIFIED Requirements

### Requirement: Nix builds publish to distributed cache

After a CI Nix build completes successfully, the system SHALL upload output store paths to the cluster's distributed Nix binary cache. The upload SHALL use the blob store for NAR data and KvCacheIndex for metadata. Published paths SHALL be immediately available via the cache gateway's HTTP substituter interface.

#### Scenario: Successful build publishes store paths

- **WHEN** a `NixBuildWorker` completes a Nix flake build with output store paths
- **THEN** each output store path SHALL be NAR-archived via `nix nar dump-path`
- **AND** the NAR SHALL be uploaded to iroh-blobs via `blob_store.add_bytes()`
- **AND** a CacheEntry SHALL be registered in KvCacheIndex with store_path, store_hash, blob_hash, nar_size, nar_hash, and references
- **AND** the store path SHALL be retrievable via `GET /{store_hash}.narinfo` on the cache gateway

#### Scenario: Build with multiple outputs

- **WHEN** a build produces outputs `out`, `dev`, and `doc`
- **THEN** all three store paths SHALL be published to the cache
- **AND** each SHALL have a separate CacheEntry with correct references

#### Scenario: Publish failure does not fail the build

- **WHEN** NAR upload or CacheEntry registration fails (e.g., network error, leader unavailable)
- **THEN** the build job SHALL still be marked as successful
- **AND** the publish failure SHALL be logged as a warning

#### Scenario: Duplicate publish is idempotent

- **WHEN** the same store path is published twice (e.g., retry after partial failure)
- **THEN** the second publish SHALL succeed without error
- **AND** the CacheEntry SHALL be overwritten with identical data
