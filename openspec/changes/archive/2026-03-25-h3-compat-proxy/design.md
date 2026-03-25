## Context

Aspen's forge web frontend and nix cache gateway serve HTTP/3 over iroh QUIC, routed by ALPN protocol negotiation. Iroh endpoints are identified by public keys, not IP addresses. Standard HTTP clients (browsers, curl, nix) speak HTTP/1.1 or HTTP/2 over TCP and have no way to connect to an iroh endpoint.

The `aspen-ci-executor-shell` already has a similar pattern — its `cache_proxy.rs` bridges localhost HTTP to the cluster's h3 nix cache. But it's tightly coupled to the CI executor and only handles nix cache requests. A general-purpose proxy is needed.

## Goals / Non-Goals

**Goals:**

- TCP HTTP/1.1 listener on localhost that forwards to any iroh h3 endpoint
- Works with any ALPN — forge web, nix cache, future h3 services
- Standalone binary for development/local use
- Library API for embedding in `aspen-node` or other binaries
- Passes through request method, path, headers, and body faithfully
- Streams response body back without buffering entire response

**Non-Goals:**

- WebSocket bridging (different protocol, separate concern)
- TLS termination on the TCP side (localhost only, no certs needed)
- Authentication or access control (the iroh endpoint handles that)
- Load balancing across multiple iroh endpoints
- HTTP/2 on the client side (HTTP/1.1 is sufficient for browsers and nix)

## Decisions

**1. hyper for the TCP side, raw h3 for the iroh side**

The TCP listener uses hyper's HTTP/1.1 server. Each incoming request is forwarded as an h3 request to the iroh endpoint via `iroh-h3` + `h3::client`. Response streams back. No axum — hyper is sufficient for a transparent proxy.

**2. One iroh connection per proxy instance, multiplexed streams**

The proxy connects to the iroh endpoint once at startup and multiplexes HTTP/3 streams over that connection. Each TCP request becomes one h3 stream. QUIC handles the multiplexing natively.

**3. ALPN specified at startup, not per-request**

The proxy targets a single ALPN (e.g., `aspen/forge-web/1`). This keeps the proxy stateless — it doesn't need to inspect requests to decide routing. Run multiple proxy instances for multiple ALPNs on different ports.

**4. Library + binary split**

The proxy logic lives in `src/lib.rs` as `H3Proxy` with a `run(config)` async function. The binary in `src/main.rs` is a thin CLI wrapper. This lets `aspen-node` embed the proxy to serve TCP alongside its iroh endpoint without a separate process.

**5. Connection retry with backoff**

If the iroh connection drops, the proxy reconnects with exponential backoff. TCP clients see 502 Bad Gateway during reconnection. The proxy doesn't buffer or retry individual requests — it's transparent.

## Risks / Trade-offs

**Risk: Head-of-line blocking on single QUIC connection.** One slow h3 stream could theoretically affect others. Mitigation: QUIC streams are independent — this is a non-issue at the protocol level. Only the connection-level flow control window matters, and the defaults are generous.

**Risk: iroh endpoint unavailability.** If the target endpoint is offline, all proxied requests fail. Mitigation: 502 responses with descriptive error. The proxy stays running and retries the connection.

**Trade-off: No HTTP/2.** Browsers prefer HTTP/2 for multiplexing. HTTP/1.1 with keep-alive is fine for the forge web UI (server-rendered pages, no heavy concurrent asset loading). If needed later, hyper supports HTTP/2 trivially.

**Trade-off: Localhost only by default.** The proxy binds to 127.0.0.1. Exposing it on 0.0.0.0 is opt-in via CLI flag. This prevents accidental exposure of iroh endpoints to the network without authentication.
