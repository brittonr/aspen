## ADDED Requirements

### Requirement: Gateway serves over HTTP/3 via iroh QUIC

The gateway SHALL serve the nar-bridge axum router over HTTP/3 when started with the `--h3` flag and the `h3-serving` feature is enabled. The H3 endpoint SHALL use ALPN `iroh+h3` and be reachable by any iroh peer.

#### Scenario: H3 gateway startup

- **WHEN** the gateway is started with `--h3 --ticket <ticket>`
- **THEN** it SHALL bind an iroh endpoint, log its endpoint ID, and accept H3 connections on the `iroh+h3` ALPN

#### Scenario: H3 nix-cache-info

- **WHEN** an H3 client sends `GET /nix-cache-info` to the gateway's iroh endpoint
- **THEN** the gateway SHALL respond with the same content as the TCP path (StoreDir, WantMassQuery, Priority)
