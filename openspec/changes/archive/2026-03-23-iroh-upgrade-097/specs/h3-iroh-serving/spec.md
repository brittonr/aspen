## h3-iroh-serving

HTTP/3 serving through iroh QUIC endpoints via ALPN routing.

### Requirements

1. The nix binary cache gateway MUST serve HTTP/3 responses through the iroh endpoint's QUIC transport, routed by ALPN protocol negotiation.
2. The h3-iroh integration MUST support axum's `Router` as the application layer — existing handler logic does not change.
3. The h3-iroh serving MUST be gated behind a `h3-serving` feature flag so it can be disabled independently.
4. The nix-cache-gateway MUST continue to work as a standalone binary connecting to the cluster via iroh client RPC.
5. Clients connecting via HTTP/3 MUST be able to fetch narinfo, nar files, and nix-cache-info from the gateway.
6. The gateway MUST NOT require a TLS certificate beyond iroh's built-in endpoint certificate (iroh QUIC connections are already authenticated).

### Acceptance Criteria

- `aspen-nix-cache-gateway` builds and serves HTTP/3 with `h3-serving` feature enabled
- Existing `nix-cache-gateway-test` NixOS VM test passes (or is adapted for HTTP/3 transport)
- Feature can be disabled (`--no-default-features`) without compilation errors
