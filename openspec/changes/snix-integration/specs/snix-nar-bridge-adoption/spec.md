## ADDED Requirements

### Requirement: Serve binary cache via nar-bridge

The nix cache gateway SHALL use `nar-bridge`'s axum router backed by Aspen's `BlobService`, `DirectoryService`, and `PathInfoService` instead of the hand-rolled hyper HTTP server.

#### Scenario: GET nix-cache-info

- **WHEN** a client sends `GET /nix-cache-info`
- **THEN** the gateway SHALL respond with HTTP 200 and cache metadata including `StoreDir`, `WantMassQuery`, and `Priority`

#### Scenario: GET narinfo for cached path

- **WHEN** a client sends `GET /{hash}.narinfo` for a path present in PathInfoService
- **THEN** the gateway SHALL respond with HTTP 200 and a valid narinfo document with `StorePath`, `NarHash`, `NarSize`, `References`, and `Sig` fields

#### Scenario: GET narinfo for uncached path

- **WHEN** a client sends `GET /{hash}.narinfo` for a path not in PathInfoService
- **THEN** the gateway SHALL respond with HTTP 404

#### Scenario: GET NAR archive

- **WHEN** a client sends `GET /nar/{hash}.nar`
- **THEN** the gateway SHALL render the NAR from BlobService/DirectoryService and stream it to the client

#### Scenario: Range request on NAR

- **WHEN** a client sends `GET /nar/{hash}.nar` with a `Range` header
- **THEN** the gateway SHALL respond with HTTP 206 and the requested byte range

### Requirement: Support NAR upload via PUT

The gateway SHALL accept `PUT` requests to upload narinfo and NAR archives, enabling bidirectional cache synchronization.

#### Scenario: Upload NAR

- **WHEN** a client sends `PUT /nar/{hash}.nar` with NAR data
- **THEN** the gateway SHALL ingest the NAR into BlobService/DirectoryService and cache the root node for subsequent narinfo upload

#### Scenario: Upload narinfo

- **WHEN** a client sends `PUT /{hash}.narinfo` with narinfo data
- **AND** the NAR for that path was previously uploaded
- **THEN** the gateway SHALL create a PathInfo entry linking the narinfo metadata to the stored root node

### Requirement: Signing key integration

The gateway SHALL sign narinfo responses using Aspen's cluster signing key, consistent with the existing `aspen-nix-cache-gateway` behavior.

#### Scenario: Narinfo includes valid signature

- **WHEN** a narinfo is served
- **THEN** it SHALL include a `Sig` field signed with the cluster's Ed25519 signing key
- **AND** the signature SHALL be verifiable with the cluster's public key

### Requirement: Remove hand-rolled HTTP server code

The `aspen-nix-cache-gateway` crate SHALL remove its custom hyper-based HTTP server implementation and delegate to `nar-bridge`'s router.

#### Scenario: Gateway binary uses nar-bridge

- **WHEN** `aspen-nix-cache-gateway` starts
- **THEN** it SHALL construct `nar_bridge::AppState` with Aspen-backed services and serve the nar-bridge router
