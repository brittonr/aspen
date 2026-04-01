## ADDED Requirements

### Requirement: NAR serialization from castore nodes

The system SHALL serialize store paths to NAR format using snix-store's `SimpleRenderer` backed by the cluster's BlobService and DirectoryService, replacing the `nix nar dump-path` subprocess.

#### Scenario: Serialize a file store path to NAR

- **WHEN** a store path with a `Node::File` root exists in PathInfoService
- **THEN** the system produces a byte-identical NAR archive to what `nix nar dump-path` would produce, with correct NAR hash

#### Scenario: Serialize a directory store path to NAR

- **WHEN** a store path with a `Node::Directory` root exists in PathInfoService
- **THEN** the system recursively renders all children from DirectoryService and BlobService into a valid NAR archive

#### Scenario: Missing blob during serialization

- **WHEN** a castore node references a blob digest not present in BlobService
- **THEN** the system returns an error identifying the missing digest and store path

### Requirement: PathInfo lookup replaces nix path-info

The system SHALL query PathInfoService for store path metadata (NAR hash, NAR size, references, deriver) instead of running `nix path-info --json`.

#### Scenario: Path metadata retrieval

- **WHEN** `check_store_path_size` is called for a store path present in PathInfoService
- **THEN** the system returns the NAR size from PathInfoService without spawning any subprocess

#### Scenario: Path not in PathInfoService

- **WHEN** the store path is not in PathInfoService
- **THEN** the system returns `None` (same behavior as current `nix path-info` failure path)

### Requirement: Upload store paths via castore

The system SHALL upload build output store paths to PathInfoService + BlobService + DirectoryService using castore ingestion, replacing the `nix nar dump-path | blob_store.put()` pipeline.

#### Scenario: Upload single output path

- **WHEN** a native build produces an output path on disk
- **THEN** the system ingests the path tree into BlobService/DirectoryService, computes the NAR hash via SimpleRenderer, and registers the PathInfo with references and signatures

#### Scenario: Upload with signing

- **WHEN** a cache signing key is configured
- **THEN** the uploaded PathInfo includes a valid signature that the nix-cache-gateway can serve in narinfo responses
