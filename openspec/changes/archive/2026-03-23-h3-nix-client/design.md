## Context

The H3 binary cache path has three components, all implemented in Rust but not wired together in Nix:

1. **Server**: `aspen-nix-cache-gateway` with `h3-serving` feature — serves nar-bridge axum router over iroh QUIC using `iroh-h3-axum`
2. **Bridge**: `aspen-h3-proxy` — generic HTTP/1.1 TCP listener that forwards requests as H3 streams to an iroh endpoint
3. **Client**: stock `nix` — talks HTTP/1.1 to `--substituters` URLs

The data flow: `nix build` → HTTP/1.1 → `aspen-h3-proxy :8080` → H3 over iroh QUIC → `aspen-nix-cache-gateway (h3)` → Aspen cluster KV/blob.

The existing `nix-cache-gateway-test` VM test exercises only the TCP HTTP path. The `aspen-h3-proxy` crate isn't built in `flake.nix` at all.

## Goals / Non-Goals

**Goals:**

- Package `aspen-h3-proxy` and h3-enabled gateway in `flake.nix`
- VM test proving nix can fetch narinfo + NAR through the full H3 path
- Close iroh-upgrade-097 deferred tasks 4.4 and 5.7

**Non-Goals:**

- Teaching nix to speak H3 natively (impossible without upstream changes)
- Streaming NAR support in h3-proxy (current full-buffer approach works for cache sizes we care about)
- Multi-node cluster in the H3 test (single-node is sufficient to prove the transport)

## Decisions

### 1. Separate gateway build variant vs feature flag

Build a new `ci-aspen-nix-cache-gateway-h3` package from `ciCommonArgs` with `--features h3-serving`. Keeps the default gateway lean. The h3-proxy is a separate binary (`ci-aspen-h3-proxy`).

Alternative: single gateway binary with runtime `--h3` flag. Rejected because it pulls in `iroh`, `iroh-h3`, `iroh-h3-axum` deps unconditionally.

### 2. VM test structure

Single-machine test (like existing `nix-cache-gateway-test`). The gateway runs with `--h3`, h3-proxy runs pointing at the gateway's endpoint ID, nix queries through the proxy. No multi-machine needed since both processes run on the same VM and communicate via iroh relay/localhost.

### 3. H3 proxy endpoint discovery

The gateway prints its endpoint ID on startup. The test parses it from journal logs, then starts h3-proxy with `--endpoint-id <id> --alpn "iroh+h3"`. This matches how a real deployment would work.

## Risks / Trade-offs

- **[Risk]** iroh endpoint discovery between two processes on the same VM may need relay or direct addrs → The gateway logs include direct socket addresses; h3-proxy can use `--endpoint-id` which triggers relay-based discovery. If needed, parse the full `EndpointAddr` from logs.
- **[Risk]** h3-proxy connection pooling timeout vs test speed → Set short timeouts (5s connect, 10s request) in the VM test to fail fast.
