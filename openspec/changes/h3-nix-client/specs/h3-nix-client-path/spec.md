## ADDED Requirements

### Requirement: Nix client fetches narinfo through H3 proxy

A Nix client SHALL be able to query `.narinfo` files through the `aspen-h3-proxy` → `aspen-nix-cache-gateway (--h3)` path. The proxy translates HTTP/1.1 requests to HTTP/3 streams over iroh QUIC.

#### Scenario: narinfo cache hit via H3

- **WHEN** a store path exists in the Aspen cluster and the nix client sends `GET /{hash}.narinfo` to the h3-proxy
- **THEN** the proxy SHALL forward the request over H3 to the gateway, and return the narinfo document with HTTP 200 and content-type `text/x-nix-narinfo`

#### Scenario: narinfo cache miss via H3

- **WHEN** a store path does not exist and the nix client sends `GET /{hash}.narinfo` to the h3-proxy
- **THEN** the proxy SHALL return HTTP 404

### Requirement: Nix client fetches nix-cache-info through H3 proxy

The `/nix-cache-info` endpoint SHALL be accessible through the H3 proxy path with the same content as the direct TCP path.

#### Scenario: cache info via H3

- **WHEN** a nix client sends `GET /nix-cache-info` to the h3-proxy
- **THEN** the response SHALL contain `StoreDir: /nix/store`, `WantMassQuery: 1`, and `Priority: 30`

### Requirement: H3 proxy binary is packaged in Nix

The `aspen-h3-proxy` binary SHALL be buildable via `flake.nix` as a Nix package for use in VM tests and deployments.

#### Scenario: Nix build produces h3-proxy binary

- **WHEN** the flake is built with the h3-proxy package target
- **THEN** the output SHALL contain a `bin/aspen-h3-proxy` executable

### Requirement: H3-enabled gateway is packaged in Nix

The `aspen-nix-cache-gateway` binary SHALL be buildable with the `h3-serving` feature via `flake.nix`.

#### Scenario: Nix build produces h3-enabled gateway

- **WHEN** the flake is built with the h3 gateway package target
- **THEN** the binary SHALL accept the `--h3` flag and serve over iroh QUIC
