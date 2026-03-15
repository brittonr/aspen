## ADDED Requirements

### Requirement: Expose nix-daemon Unix socket

The snix-bridge binary SHALL expose a Unix domain socket implementing the nix-daemon worker protocol, backed by Aspen's `BlobService`, `DirectoryService`, and `PathInfoService` implementations.

#### Scenario: Socket creation on startup

- **WHEN** `aspen-snix-bridge` starts with `--daemon-socket /tmp/aspen-nix-daemon.sock`
- **THEN** the binary SHALL create a Unix socket at that path and accept nix-daemon protocol connections

#### Scenario: Socket cleanup on shutdown

- **WHEN** `aspen-snix-bridge` shuts down
- **THEN** the binary SHALL remove the Unix socket file

### Requirement: Query path info via daemon protocol

The daemon SHALL respond to `QueryPathInfo` requests by looking up store paths in Aspen's `PathInfoService`.

#### Scenario: Path exists in store

- **WHEN** a nix client sends `QueryPathInfo` for `/nix/store/abc123-hello-2.12.1`
- **AND** the path exists in Aspen's PathInfoService
- **THEN** the daemon SHALL respond with `UnkeyedValidPathInfo` containing `narHash`, `narSize`, `references`, and `deriver` fields

#### Scenario: Path does not exist

- **WHEN** a nix client sends `QueryPathInfo` for a store path not in the PathInfoService
- **THEN** the daemon SHALL respond with `None`

### Requirement: Validate paths via daemon protocol

The daemon SHALL respond to `IsValidPath` requests by checking existence in Aspen's PathInfoService.

#### Scenario: Valid path check

- **WHEN** a nix client sends `IsValidPath` for `/nix/store/abc123-hello-2.12.1`
- **AND** the path exists in Aspen's PathInfoService
- **THEN** the daemon SHALL respond with `true`

#### Scenario: Invalid path check

- **WHEN** a nix client sends `IsValidPath` for a path not in the PathInfoService
- **THEN** the daemon SHALL respond with `false`

### Requirement: Add store paths via daemon protocol

The daemon SHALL accept `AddToStoreNar` requests to ingest NAR archives into Aspen's decomposed storage.

#### Scenario: Successful NAR ingestion

- **WHEN** a nix client sends `AddToStoreNar` with a valid NAR archive
- **THEN** the daemon SHALL ingest the NAR into BlobService and DirectoryService via `ingest_nar_and_hash`
- **AND** create a `PathInfo` entry in PathInfoService
- **AND** respond with success

#### Scenario: Corrupted NAR rejected

- **WHEN** a nix client sends `AddToStoreNar` with corrupt NAR data
- **THEN** the daemon SHALL respond with an error and NOT create any PathInfo entry

### Requirement: Query valid paths in batch

The daemon SHALL respond to `QueryValidPaths` requests by checking multiple store paths against the PathInfoService.

#### Scenario: Batch validation

- **WHEN** a nix client sends `QueryValidPaths` with 5 store paths where 3 exist
- **THEN** the daemon SHALL respond with exactly the 3 valid paths

### Requirement: Share services with gRPC bridge

The nix-daemon socket and existing gRPC socket SHALL share the same `BlobService`, `DirectoryService`, and `PathInfoService` instances to ensure consistency.

#### Scenario: Path added via daemon visible via gRPC

- **WHEN** a store path is added via the nix-daemon socket
- **THEN** the same path SHALL be queryable via the gRPC PathInfoService immediately
