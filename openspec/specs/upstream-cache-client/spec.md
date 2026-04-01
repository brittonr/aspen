## ADDED Requirements

### Requirement: Query upstream binary cache narinfo

The system SHALL query upstream Nix binary caches (e.g., `cache.nixos.org`) via HTTP to retrieve narinfo metadata for store paths not present in the cluster's PathInfoService.

#### Scenario: Narinfo found in upstream cache

- **WHEN** PathInfoService has no entry for a store path AND the upstream cache URL is configured
- **THEN** the system fetches `https://<cache>/<hash>.narinfo`, parses the NAR hash, NAR size, references, deriver, and signatures, and returns a structured `NarInfo` result

#### Scenario: Narinfo not found upstream

- **WHEN** the upstream cache returns HTTP 404 for a store path hash
- **THEN** the system returns a `NotFound` result without error

#### Scenario: Upstream cache unreachable

- **WHEN** the HTTP request to the upstream cache times out or returns a server error
- **THEN** the system returns an error after the configured timeout (default 30s) without panicking

### Requirement: Fetch NAR archive from upstream cache

The system SHALL download NAR archives from upstream binary caches and ingest them into the cluster's BlobService and DirectoryService.

#### Scenario: Successful NAR fetch and ingest

- **WHEN** narinfo is resolved and the NAR URL is known
- **THEN** the system downloads the NAR (following compression indicated by narinfo), decompresses it, ingests it via snix-castore's NAR ingestion, and registers the resulting PathInfo in PathInfoService

#### Scenario: NAR exceeds size limit

- **WHEN** the NAR size reported in narinfo exceeds `MAX_NAR_FETCH_SIZE` (2 GB)
- **THEN** the system skips the fetch and returns an error indicating the path is too large

#### Scenario: NAR hash mismatch

- **WHEN** the downloaded NAR content does not match the hash declared in narinfo
- **THEN** the system discards the content and returns a verification error

### Requirement: Populate closure from upstream cache

The system SHALL recursively resolve the transitive closure of a store path by fetching narinfo for each reference, walking the reference graph breadth-first until all paths are either present in PathInfoService or fetched from upstream.

#### Scenario: Full closure resolution

- **WHEN** a derivation requires input paths not in PathInfoService
- **THEN** the system fetches narinfo for each missing path, discovers its references, and recursively fetches until the full closure is populated

#### Scenario: Closure depth limit

- **WHEN** the transitive closure exceeds `MAX_CLOSURE_PATHS` (50,000)
- **THEN** the system stops fetching and returns an error listing the count of resolved and unresolved paths

#### Scenario: Partial resolution with fallback

- **WHEN** some paths in the closure cannot be resolved from any configured upstream cache AND `nix-cli-fallback` feature is enabled
- **THEN** the system returns the list of unresolved paths so the caller can fall back to `nix-store --realise`

### Requirement: Cache configuration

The system SHALL accept a list of upstream cache URLs and their trusted public keys via `NixBuildWorkerConfig`.

#### Scenario: Default configuration

- **WHEN** no upstream caches are explicitly configured
- **THEN** the system uses `https://cache.nixos.org` with the well-known public key `cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY=`

#### Scenario: Custom upstream caches

- **WHEN** `upstream_caches` is set in config with URLs and keys
- **THEN** the system queries caches in order, using the first successful response
