## 1. Client RPC plumbing

- [ ] 1.1 Add `FederationSyncPeer { peer_node_id: String }` variant to `FederationRequest` in `crates/aspen-client-api/src/messages/federation.rs`
- [ ] 1.2 Add `FederationSyncPeerResponse` with `remote_cluster_name`, `remote_cluster_key`, `trusted`, `resources` (list of resource type + ref count) to response types
- [ ] 1.3 Handle `FederationSyncPeer` in the forge federation handler (`crates/aspen-forge-handler/src/handler/handlers/federation.rs`): parse node ID, call `connect_to_cluster` + `list_remote_resources` + `get_remote_resource_state`, return response

## 2. CLI command

- [ ] 2.1 Add `Sync(SyncArgs)` variant to the federation subcommand enum in `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs`
- [ ] 2.2 Implement `SyncArgs` with `--peer <node-id>` argument
- [ ] 2.3 Implement the sync command handler: send `FederationSyncPeer` RPC, display remote cluster name, trust status, and ref heads

## 3. NixOS VM end-to-end test

- [ ] 3.1 In `nix/tests/federation.nix`, extract iroh node public key from alice's cluster ticket
- [ ] 3.2 From bob's VM, run `aspen-cli federation sync --peer <alice-node-id>` and capture output
- [ ] 3.3 Assert the sync response contains alice's cluster name and the repo created earlier
- [ ] 3.4 Verify ref heads match: the ref alice created should appear in bob's sync output

## 4. Tests

- [ ] 4.1 Unit test: `FederationSyncPeer` request/response serialization roundtrip
- [ ] 4.2 Integration test: verify the RPC handler returns error for unreachable peer
