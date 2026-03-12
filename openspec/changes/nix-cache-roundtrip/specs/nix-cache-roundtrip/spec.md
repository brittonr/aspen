## ADDED Requirements

### Requirement: Cache round-trip in serial dogfood VM

The serial dogfood VM SHALL run the nix cache gateway alongside aspen-node, and CI builds SHALL both publish to and consume from the cluster's binary cache.

#### Scenario: Gateway starts with cluster

- **WHEN** the serial dogfood VM boots and the aspen-node cluster is initialized
- **THEN** the nix cache gateway SHALL be running on `127.0.0.1:8380`
- **AND** the cache signing key SHALL exist in KV at `_system:nix-cache:signing-key`
- **AND** `GET /nix-cache-info` SHALL return HTTP 200 with `StoreDir: /nix/store`

#### Scenario: First build populates cache

- **WHEN** a CI pipeline triggers a `nix build` of a flake (e.g., cowsay)
- **THEN** the NixBuildWorker SHALL upload output store paths as NAR blobs to iroh-blobs
- **AND** the NixBuildWorker SHALL register each store path in the KvCacheIndex
- **AND** `GET /{store_hash}.narinfo` on the gateway SHALL return HTTP 200 with a signed narinfo

#### Scenario: Second build hits cache

- **WHEN** a second CI pipeline builds a flake whose output store paths already exist in the cache
- **THEN** `nix build` SHALL fetch the cached paths from the gateway substituter instead of rebuilding or downloading from cache.nixos.org
- **AND** the gateway access log SHALL show HTTP 200 responses for `.narinfo` and `/nar/` requests

#### Scenario: Cache miss falls through

- **WHEN** `nix build` requests a store path not in the cluster cache
- **THEN** the gateway SHALL return HTTP 404 for the `.narinfo` request
- **AND** `nix build` SHALL fall through to cache.nixos.org without error

### Requirement: Signing key provisioned before first CI build

The aspen-node SHALL ensure the nix cache signing keypair exists in KV before any CI worker starts processing jobs.

#### Scenario: Key created on node startup

- **WHEN** the node starts with `nix-cache-gateway` feature enabled and no signing key exists in KV
- **THEN** the node SHALL call `ensure_signing_key` to generate and store the keypair
- **AND** the public key SHALL be available for NixBuildWorkerConfig before worker registration

#### Scenario: Key already exists

- **WHEN** the node starts and the signing key already exists in KV
- **THEN** the node SHALL load the existing key without regenerating

### Requirement: Gateway URL configured for CI workers

The NixBuildWorkerConfig SHALL receive the gateway URL so the substituter can be injected into `nix build` commands.

#### Scenario: Gateway URL from config

- **WHEN** `config.nix_cache.gateway_url` is set (e.g., `http://127.0.0.1:8380`)
- **THEN** NixBuildWorkerConfig SHALL have `gateway_url: Some(url)`
- **AND** `substituter_args()` SHALL return `["--extra-substituters", "{url}"]`

#### Scenario: Gateway URL not configured

- **WHEN** `config.nix_cache.gateway_url` is None
- **THEN** NixBuildWorkerConfig SHALL have `gateway_url: None`
- **AND** the cache proxy path (`can_use_cache_proxy()`) may still be used if iroh_endpoint is available
