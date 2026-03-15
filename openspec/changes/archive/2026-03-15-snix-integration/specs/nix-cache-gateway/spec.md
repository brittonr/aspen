## MODIFIED Requirements

### Requirement: Serve nix-cache-info

The gateway SHALL respond to `GET /nix-cache-info` with a valid Nix binary cache info document containing `StoreDir`, `WantMassQuery`, and `Priority` fields. The HTTP layer SHALL be provided by `nar-bridge`'s axum router instead of the custom hyper implementation.

#### Scenario: Nix client discovers cache

- **WHEN** a client sends `GET /nix-cache-info`
- **THEN** the gateway SHALL respond with HTTP 200 and body containing `StoreDir: /nix/store`, `WantMassQuery: 1`, `Priority: 30`

### Requirement: Serve narinfo by store hash

The gateway SHALL respond to `GET /{hash}.narinfo` with a Nix narinfo document for cached store paths, or HTTP 404 for uncached paths. Narinfo rendering and signature generation SHALL be handled by `nar-bridge` using Aspen-backed services.

#### Scenario: Cache hit

- **WHEN** a client sends `GET /abc123.narinfo` and store hash `abc123` exists in the PathInfoService
- **THEN** the gateway SHALL respond with HTTP 200, content-type `text/x-nix-narinfo`, and a body containing `StorePath`, `NarHash`, `NarSize`, `References`, and `Sig` fields

#### Scenario: Cache miss

- **WHEN** a client sends `GET /abc123.narinfo` and store hash `abc123` does not exist in the PathInfoService
- **THEN** the gateway SHALL respond with HTTP 404

#### Scenario: Invalid hash format

- **WHEN** a client sends `GET /INVALID!.narinfo` with non-alphanumeric characters
- **THEN** the gateway SHALL respond with HTTP 400

### Requirement: Serve NAR archives

The gateway SHALL respond to `GET /nar/{hash}.nar` with NAR data rendered from BlobService and DirectoryService via `nar-bridge`'s NAR rendering pipeline.

#### Scenario: NAR download

- **WHEN** a client sends `GET /nar/{hash}.nar` for a known blob hash
- **THEN** the gateway SHALL stream the rendered NAR archive

#### Scenario: NAR not found

- **WHEN** a client sends `GET /nar/{hash}.nar` for an unknown hash
- **THEN** the gateway SHALL respond with HTTP 404
