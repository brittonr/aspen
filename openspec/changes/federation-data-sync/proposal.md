## Why

The federation QUIC handshake works between clusters. But no data flows — `handle_federate_repository` is a stub, the sync handler doesn't call `get_resource_state` per resource, and there's no code to store synced refs locally. Alice can create a repo, but Bob can't see it after syncing.

## What Changes

- Wire `handle_federate_repository` to persist `FederationSettings` to KV via `ForgeNode::set_federation_settings`, constructing a `FederatedId` from the cluster identity and repo ID.
- Extend `handle_federation_sync_peer` to call `get_remote_resource_state` for each resource returned by `list_remote_resources`, and return ref heads in the sync result.
- Store synced remote ref state in local KV under `_fed:<origin_short>:<repo_short>:refs/<name>` so the data survives the RPC call.
- Update the NixOS federation test: Alice creates a repo → federates it → Bob syncs → Bob verifies remote refs are returned.

## Capabilities

### New Capabilities

- `federation-repo-federating`: Persist federation settings for a repo via the RPC, enabling it to appear in remote resource listings.
- `federation-ref-sync`: Pull remote ref heads during federation sync and store them locally in KV.

### Modified Capabilities

## Impact

- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — wire `handle_federate_repository` and extend `handle_federation_sync_peer`
- `crates/aspen-client-api/src/messages/federation.rs` — add ref data to `SyncPeerResourceInfo`
- `nix/tests/federation.nix` — full create → federate → sync → verify flow
