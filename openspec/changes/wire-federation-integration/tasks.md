## 1. RouterBuilder federation method

- [x] 1.1 Add `federation()` method to `RouterBuilder` in `crates/aspen-cluster/src/router_builder.rs` that accepts a `FederationProtocolHandler` and registers at `FEDERATION_ALPN`, feature-gated behind `cfg(feature = "federation")`
- [x] 1.2 Add `aspen-federation` dependency to `aspen-cluster/Cargo.toml` with optional `federation` feature

## 2. Federation bootstrap module

- [x] 2.1 Create `crates/aspen-cluster/src/bootstrap/node/federation_init.rs` with `setup_federation()` function that reads config and returns `Option<FederationProtocolHandler>`
- [x] 2.2 Build `ClusterIdentity` from `config.federation.cluster_secret_key` hex string, log error and return `None` on invalid key
- [x] 2.3 Build `TrustManager` pre-populated with `config.federation.trusted_clusters` public keys at `TrustLevel::Trusted`
- [x] 2.4 Build `FederationProtocolContext` with endpoint, HLC, identity, trust manager, and resource resolver
- [x] 2.5 Wire `setup_federation()` into `bootstrap_node()` in `crates/aspen-cluster/src/bootstrap/node/mod.rs`, pass result through to caller

## 3. Node router registration

- [x] 3.1 In `src/node/mod.rs` `spawn_router_with_blobs()`, call `builder.federation(handler)` when bootstrap returned a federation handler
- [x] 3.2 Store federation context (identity, trust manager) on `NodeHandle` for access by CLI/RPC handlers

## 4. Forge resource resolver

- [x] 4.1 Create `ForgeResourceResolver` struct implementing `FederationResourceResolver` trait, located in `crates/aspen-forge/src/resolver.rs` (or `crates/aspen-forge-handler/`)
- [x] 4.2 Implement `resource_exists()` — scan KV for federation settings matching the `FederatedId`
- [x] 4.3 Implement `get_resource_state()` — read ref heads from forge repo KV keys, return as `FederationResourceState`
- [x] 4.4 Implement `sync_objects()` — return git objects from blob storage for requested hashes
- [x] 4.5 Respect `FederationSettings` per resource: return `FederationDisabled` error when mode is `Disabled`
- [ ] 4.6 Wire `ForgeResourceResolver` into the `FederationProtocolContext` during bootstrap when forge feature is enabled

## 5. Config plumbing

- [x] 5.1 Verify `crates/aspen-cluster/src/config/federation.rs` has `cluster_secret_key`, `cluster_name`, `trusted_clusters` fields (may already exist)
- [ ] 5.2 Add `federation.cluster_secret_key` and `federation.cluster_name` to NixOS module options in `nix/modules/aspen-node.nix` (as `secretKey` and `clusterName` under `services.aspen.node`)
- [ ] 5.3 Ensure environment variable `ASPEN_CLUSTER_KEY` maps to the config field

## 6. Tests

- [ ] 6.1 Unit test: `RouterBuilder::federation()` registers handler at correct ALPN
- [x] 6.2 Unit test: `setup_federation()` returns `None` when no key configured
- [x] 6.3 Unit test: `setup_federation()` returns handler when valid key provided
- [x] 6.4 Unit test: `setup_federation()` returns `None` and logs error on invalid hex key
- [x] 6.5 Unit test: `ForgeResourceResolver::get_resource_state()` returns ref heads from KV
- [x] 6.6 Unit test: `ForgeResourceResolver` returns `NotFound` for unknown `FederatedId`
- [x] 6.7 Unit test: `ForgeResourceResolver` returns `FederationDisabled` when mode is disabled
- [ ] 6.8 Expand `nix/tests/federation.nix` — cluster A creates repo + federates, cluster B performs federation handshake + `GetResourceState`, assert ref heads match
