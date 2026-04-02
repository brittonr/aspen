## ADDED Requirements

### Requirement: Local store ingestion replaces placeholder nodes

When `resolve_single_input` finds a store path on the local filesystem but not in PathInfoService, it SHALL ingest the path into BlobService/DirectoryService and return a Node with a real B3Digest and accurate size. The system SHALL NOT create nodes with zeroed digests or zero sizes for paths that exist on disk.

#### Scenario: File exists locally but not in PathInfoService

- **WHEN** a store path points to a regular file on disk that has no PathInfoService entry
- **THEN** the resolver reads the file, computes its B3 digest via BlobService, and returns a `Node::File` with the correct digest, size, and executable bit

#### Scenario: Directory exists locally but not in PathInfoService

- **WHEN** a store path points to a directory on disk that has no PathInfoService entry
- **THEN** the resolver walks the directory tree, ingests all children into BlobService and DirectoryService, and returns a `Node::Directory` with the correct digest and size

#### Scenario: Symlink exists locally but not in PathInfoService

- **WHEN** a store path points to a symlink on disk that has no PathInfoService entry
- **THEN** the resolver returns a `Node::Symlink` with the correct target

### Requirement: Pre-build input verification

After materialization and before sandbox execution, the build pipeline SHALL verify that every required store path exists on the local filesystem. If any paths are missing, the build SHALL fail with a structured error listing all missing paths.

#### Scenario: All inputs present

- **WHEN** all required store paths (input derivation outputs, input sources, builder) exist on disk
- **THEN** the verification passes and the build proceeds to sandbox execution

#### Scenario: Some inputs missing after materialization

- **WHEN** materialization completes but one or more required store paths are still missing from disk
- **THEN** the build fails with an error that lists every missing path and their source (input derivation, input source, or builder)

#### Scenario: Resource bound on verification

- **WHEN** the number of paths to verify exceeds `MAX_BUILD_INPUTS`
- **THEN** the verification rejects the request with a limit-exceeded error

### Requirement: Input derivation resolution reports missing derivations

When `collect_input_store_paths` cannot read an input derivation from disk, it SHALL record the missing path and include it in a structured report. The system SHALL log missing input derivations at `warn` level, not `debug`.

#### Scenario: Input derivation file missing from disk

- **WHEN** `collect_input_store_paths` encounters an input derivation path that does not exist on disk
- **THEN** the function logs at `warn` level and includes the path in the returned unresolved list

#### Scenario: Input derivation file exists but fails to parse

- **WHEN** `collect_input_store_paths` reads a .drv file that fails ATerm parsing
- **THEN** the function logs at `warn` level with the parse error and includes the path in the returned unresolved list
