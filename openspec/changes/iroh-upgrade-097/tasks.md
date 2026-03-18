## 1. Version bumps and Cargo.toml updates

- [x] 1.1 Create worktree `iroh-097` from main
- [x] 1.2 Bump `iroh` from 0.95.1 to 0.97.0 in workspace Cargo.toml
- [x] 1.3 Bump `iroh-blobs` from 0.97 to 0.99.0 in workspace Cargo.toml
- [x] 1.4 Bump `iroh-docs` from 0.95 to 0.97.0 in workspace Cargo.toml
- [x] 1.5 Add `iroh-tickets` workspace dependency (tickets moved from iroh-base)
- [x] 1.6 Update `h3-iroh` git dependency — remove version-incompatibility comments from crate Cargo.tomls
- [x] 1.7 Run `cargo update` to regenerate Cargo.lock
- [x] 1.8 Update `crate-hashes.json` for nix builds (deferred to task 5.5)

## 2. Core API migration (compile fixes, dependency order)

- [x] 2.1 `aspen-core` — update iroh re-exports, key types, `EndpointAddr` changes
- [x] 2.2 `aspen-transport` — ALPN routing, `ServerConfig`/`TransportConfig` newtypes, ticket imports from `iroh-tickets`
- [x] 2.3 `aspen-cluster` — `Discovery` → `AddressLookup` rename, `Endpoint::add_node_addr` removal, bootstrap flow, gossip
- [x] 2.4 `aspen-dht-discovery` — `Discovery` → `AddressLookup` trait impl rename
- [x] 2.5 `aspen-raft-network` — connection handling, error type changes
- [x] 2.6 `aspen-client` — client connection, ticket parsing
- [x] 2.7 `aspen-rpc-core` / `aspen-rpc-handlers` — remove h3-iroh incompatibility comments, update connection types

## 3. Feature crate migration

- [x] 3.1 `aspen-blob` / `aspen-blob-handler` — iroh-blobs 0.99 API changes
- [x] 3.2 `aspen-docs` / `aspen-docs-handler` — iroh-docs 0.97 API changes
- [x] 3.3 `aspen-forge` — gossip types, connection handling
- [x] 3.4 `aspen-jobs` / `aspen-ci` — iroh endpoint usage in VM executor, worker comms
- [x] 3.5 `aspen-proxy` / `aspen-net` — connection types, endpoint usage
- [x] 3.6 `aspen-snix` / `aspen-snix-bridge` / `aspen-castore` — verify no iroh type leakage from vendored snix
- [x] 3.7 `aspen-tui` / `aspen-cli` — client-side iroh usage, ticket types
- [x] 3.8 Remaining crates — grep for any missed `use iroh::` imports, fix compilation

## 4. h3-iroh integration

- [x] 4.1 Add `h3-serving` feature flag to `aspen-nix-cache-gateway`
- [x] 4.2 Replace axum TCP listener with h3-iroh QUIC listener using axum router
- [x] 4.3 Define HTTP/3 ALPN constant in `aspen-transport`
- [ ] 4.4 Verify nix client can fetch from HTTP/3 endpoint (deferred — requires running cluster)

## 5. Build and test verification

- [x] 5.1 `cargo build` — full workspace compiles
- [x] 5.2 `cargo nextest run -P quick` — quick test suite passes (450/451 passed; 1 failure is pre-existing disk space issue)
- [x] 5.3 `nix build .#checks.x86_64-linux.build-node` — nix build with near-full features
- [x] 5.4 `nix build .#checks.x86_64-linux.clippy` — no new warnings
- [x] 5.5 Update `flake.nix` — crate-hashes, h3-iroh vendor overrides if needed
- [ ] 5.6 Run key NixOS VM tests: `kv-operations-test`, `multi-node-cluster`, `e2e-push-build-cache`
- [ ] 5.7 Run `nix-cache-gateway-test` with h3-iroh transport

## 6. Cleanup and merge

- [x] 6.1 Remove all `h3-iroh requires iroh 0.96+` comments from Cargo.tomls
- [x] 6.2 Remove stale `quinn::` direct imports if any remain
- [ ] 6.3 Merge worktree branch to main, push (after remaining verification)
