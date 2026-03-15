## ADDED Requirements

### Requirement: Serve nix-cache-info

The gateway SHALL respond to `GET /nix-cache-info` with a valid Nix binary cache info document containing `StoreDir`, `WantMassQuery`, and `Priority` fields. The HTTP layer SHALL be provided by `nar-bridge`'s axum router backed by Aspen's `BlobService`/`DirectoryService`/`PathInfoService`.

#### Scenario: Nix client discovers cache

- **WHEN** a client sends `GET /nix-cache-info`
- **THEN** the gateway SHALL respond with HTTP 200 and body containing `StoreDir: /nix/store`, `WantMassQuery: 1`, `Priority: 30`

### Requirement: Serve narinfo by store hash

The gateway SHALL respond to `GET /{hash}.narinfo` with a Nix narinfo document for cached store paths, or HTTP 404 for uncached paths. Narinfo rendering and signature generation SHALL be handled by `nar-bridge` using Aspen-backed services.

#### Scenario: Cache hit

- **WHEN** a client sends `GET /abc123.narinfo` and store hash `abc123` exists in the cache index
- **THEN** the gateway SHALL respond with HTTP 200, content-type `text/x-nix-narinfo`, and a body containing `StorePath`, `NarHash`, `NarSize`, `References`, and `Sig` fields

#### Scenario: Cache miss

- **WHEN** a client sends `GET /abc123.narinfo` and store hash `abc123` does not exist in the cache index
- **THEN** the gateway SHALL respond with HTTP 404

#### Scenario: Invalid hash format

- **WHEN** a client sends `GET /INVALID!.narinfo` with non-alphanumeric characters
- **THEN** the gateway SHALL respond with HTTP 400

### Requirement: Serve NAR archive by blob hash

The gateway SHALL respond to `GET /nar/{hash}.nar` with NAR data rendered from BlobService and DirectoryService via `nar-bridge`'s NAR rendering pipeline.

#### Scenario: NAR download

- **WHEN** a client sends `GET /nar/{blob_hash}.nar` and the blob exists
- **THEN** the gateway SHALL stream the NAR archive with content-type `application/x-nix-nar` and the correct content-length

#### Scenario: NAR not found

- **WHEN** a client sends `GET /nar/{blob_hash}.nar` and the blob does not exist
- **THEN** the gateway SHALL respond with HTTP 404

#### Scenario: NAR size bound

- **WHEN** a NAR archive exceeds 1 GB
- **THEN** the gateway SHALL refuse to serve it with HTTP 413

### Requirement: Connect to cluster via ticket

The gateway SHALL accept a cluster ticket on startup and connect to the aspen cluster as a regular client.

#### Scenario: Start with valid ticket

- **WHEN** the gateway starts with `--ticket <cluster-ticket>`
- **THEN** it SHALL connect to the cluster and begin serving HTTP on the configured port

#### Scenario: Start without ticket

- **WHEN** the gateway starts without a `--ticket` flag
- **THEN** it SHALL exit with an error message explaining the required flag

#### Scenario: Cluster unreachable

- **WHEN** the gateway cannot connect to any cluster node within 30 seconds
- **THEN** it SHALL exit with an error describing the connection failure

### Requirement: Bind to localhost only

The gateway SHALL bind its HTTP listener to `127.0.0.1` by default. It SHALL NOT bind to `0.0.0.0` unless explicitly configured.

#### Scenario: Default bind

- **WHEN** the gateway starts without a `--bind` flag
- **THEN** it SHALL listen on `127.0.0.1:8380`

#### Scenario: Custom port

- **WHEN** the gateway starts with `--port 9090`
- **THEN** it SHALL listen on `127.0.0.1:9090`

### Requirement: Concurrent request limit

The gateway SHALL limit concurrent requests to a configurable maximum (default: 100).

#### Scenario: Under limit

- **WHEN** fewer than 100 requests are in flight
- **THEN** new requests SHALL be processed normally

#### Scenario: At limit

- **WHEN** 100 requests are in flight and a new request arrives
- **THEN** the gateway SHALL respond with HTTP 503 Service Unavailable
