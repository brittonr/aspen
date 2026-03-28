## ADDED Requirements

### Requirement: Materialize store paths from castore to local filesystem

The system SHALL materialize missing `/nix/store` paths from Aspen's castore services (PathInfoService, BlobService, DirectoryService) to the local filesystem without spawning subprocesses.

#### Scenario: All paths present in castore

- **WHEN** `materialize_store_paths` is called with paths that exist in PathInfoService and have complete content in BlobService/DirectoryService
- **THEN** each path SHALL be written to `/nix/store/<hash>-<name>` with correct file contents, directory structure, symlink targets, and Unix permissions

#### Scenario: Path missing from PathInfoService

- **WHEN** a requested store path has no entry in PathInfoService
- **THEN** the path SHALL be reported as unresolved in the `MaterializeReport` and the function SHALL NOT fail for other paths

#### Scenario: Path metadata exists but blob content missing

- **WHEN** PathInfoService has the path's metadata but BlobService lacks the blob data for a file node
- **THEN** the path SHALL be reported as a content error in the `MaterializeReport` and materialization SHALL continue for other paths

#### Scenario: Target path already exists on disk

- **WHEN** the target `/nix/store/<hash>-<name>` already exists on the local filesystem
- **THEN** the materializer SHALL skip it without error or overwrite

### Requirement: Walk castore Node tree for filesystem materialization

The system SHALL walk the castore `Node` tree directly (without NAR serialization) to produce filesystem output.

#### Scenario: File node materialization

- **WHEN** a `Node::File` is encountered during tree walking
- **THEN** the blob content SHALL be read from BlobService by its BLAKE3 digest and written to the target path with executable permission set if `executable` is true

#### Scenario: Directory node materialization

- **WHEN** a `Node::Directory` is encountered during tree walking
- **THEN** the directory SHALL be created and its children SHALL be resolved from DirectoryService by the directory's BLAKE3 digest, then each child SHALL be materialized recursively

#### Scenario: Symlink node materialization

- **WHEN** a `Node::Symlink` is encountered during tree walking
- **THEN** a symbolic link SHALL be created at the target path pointing to the symlink's target string

### Requirement: Resource bounds on materialization

The system SHALL enforce Tiger Style resource bounds during materialization.

#### Scenario: Path count limit

- **WHEN** more than 10,000 store paths are requested for materialization
- **THEN** the system SHALL return an error without attempting materialization

#### Scenario: Single blob size limit

- **WHEN** a file blob exceeds 256 MB in size
- **THEN** the system SHALL skip that file, log a warning, and continue with other paths

#### Scenario: Directory depth limit

- **WHEN** a directory tree exceeds 256 levels of nesting
- **THEN** the system SHALL stop recursion for that subtree and log a warning

### Requirement: Fallback to nix-store subprocess when castore unavailable

When castore services cannot resolve a store path, the system SHALL fall back to `nix-store --realise` subprocess if the `nix-cli-fallback` feature is enabled.

#### Scenario: Castore resolves all paths

- **WHEN** all missing paths are successfully materialized from castore
- **THEN** no subprocess SHALL be spawned

#### Scenario: Some paths unresolved, nix-cli-fallback enabled

- **WHEN** some paths cannot be resolved from castore AND the `nix-cli-fallback` feature is enabled
- **THEN** the unresolved paths SHALL be passed to `nix-store --realise` subprocess as a fallback

#### Scenario: Some paths unresolved, nix-cli-fallback disabled

- **WHEN** some paths cannot be resolved from castore AND the `nix-cli-fallback` feature is NOT enabled
- **THEN** the system SHALL return an error listing the unresolved paths

### Requirement: MaterializeReport provides detailed results

The `materialize_store_paths` function SHALL return a `MaterializeReport` with per-path results.

#### Scenario: Successful materialization report

- **WHEN** materialization completes
- **THEN** the report SHALL include: count of paths materialized from castore, count skipped (already present), count resolved via subprocess fallback, count of errors, and total elapsed time
