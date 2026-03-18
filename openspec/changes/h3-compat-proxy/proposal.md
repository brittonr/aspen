## Why

Aspen serves HTTP/3 over iroh QUIC (forge web frontend, nix binary cache gateway) but browsers and standard HTTP clients can't connect to iroh endpoints. An iroh endpoint is identified by a public key, not an IP:port. Without a bridge, h3-served content is only reachable by other iroh peers. This blocks browser access to the forge web UI and prevents standard `nix` clients from using the h3 cache gateway.

## What Changes

- New lightweight proxy binary (`aspen-h3-proxy`) that listens on a local TCP port and forwards HTTP requests to an iroh endpoint's h3 ALPN over QUIC
- The proxy translates between HTTP/1.1 (TCP) on the client side and HTTP/3 (iroh QUIC) on the server side
- Supports any h3-serving ALPN — works for both `aspen/forge-web/1` and `iroh+h3` (nix cache)
- Single static binary, no cluster ticket needed — only the target endpoint ID and ALPN
- Can also be embedded as a library for in-process bridging (e.g., `aspen-node` serving TCP alongside its iroh endpoint)

## Capabilities

### New Capabilities
- `h3-compat-proxy`: Local TCP-to-iroh-h3 proxy that makes iroh-served HTTP/3 content accessible to browsers and standard HTTP clients

### Modified Capabilities

## Impact

- New crate: `crates/aspen-h3-proxy/`
- New binary: `aspen-h3-proxy`
- Dependencies: `iroh`, `iroh-h3`, `h3`, `hyper` (for TCP HTTP/1.1 server side)
- Consumers: forge web frontend (browser access), nix cache gateway (standard nix client access), any future h3-served content
- No changes to existing crates — purely additive
