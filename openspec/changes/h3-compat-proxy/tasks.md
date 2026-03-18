## 1. Crate setup

- [ ] 1.1 Create `crates/aspen-h3-proxy/` with Cargo.toml (deps: iroh, iroh-h3, h3, hyper, tokio, clap, anyhow, tracing)
- [ ] 1.2 Add to workspace members in root Cargo.toml
- [ ] 1.3 Define `FORGE_WEB_ALPN` constant in `aspen-transport/src/constants.rs`

## 2. Library core

- [ ] 2.1 `src/lib.rs` — `H3Proxy` struct with `new(config)` and `async fn run(&self)` 
- [ ] 2.2 `src/connection.rs` — iroh endpoint setup, h3 client connection to target endpoint ID + ALPN, reconnect with exponential backoff
- [ ] 2.3 `src/bridge.rs` — per-request handler: receive hyper HTTP/1.1 request, open h3 stream, forward method/path/headers/body, stream h3 response back to hyper response
- [ ] 2.4 `src/config.rs` — `ProxyConfig` struct: bind addr, port, endpoint ID, ALPN, reconnect settings

## 3. Binary

- [ ] 3.1 `src/main.rs` — CLI with clap: `--endpoint-id`, `--alpn`, `--port`, `--bind`, `--timeout-secs`
- [ ] 3.2 Graceful shutdown on ctrl-c

## 4. Integration

- [ ] 4.1 Add `H3Proxy` re-export to `aspen-forge-web` so it can embed the proxy alongside the h3 server
- [ ] 4.2 Add `--tcp-port` flag to `aspen-forge-web` binary that starts an embedded `H3Proxy` pointing at its own endpoint

## 5. Verification

- [ ] 5.1 `cargo check -p aspen-h3-proxy` compiles
- [ ] 5.2 `cargo clippy -p aspen-h3-proxy -- --deny warnings` clean
- [ ] 5.3 Manual test: start forge-web, start h3-proxy pointing at it, curl localhost — HTML returned
- [ ] 5.4 Manual test: start nix-cache-gateway with `--h3`, start h3-proxy, `nix path-info --store http://localhost:<port>` works
