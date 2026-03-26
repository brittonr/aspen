## Why

Federation sync currently transfers ref entries (ref name → commit hash) but not the actual git objects those refs point to. After `federation fetch`, the local cluster knows *what* exists on the remote but has none of the content. The missing piece is iroh-blobs transfer of pack data so a federated repo can be cloned/read locally.

## What Changes

- Add a `"packfile"` object type to the federation sync protocol, where the server generates a git packfile for requested objects and transfers it via iroh-blobs (content-addressed, resumable, BLAKE3-verified)
- Add a local mirror repo creation step: after fetching refs + pack data, materialize a read-only mirror forge repo so `git clone aspen://mirror/...` works
- Extend `federation fetch` CLI to trigger pack transfer and mirror creation (not just ref persistence to `_fed:mirror:` KV keys)
- Add a `federation pull` CLI command for ongoing incremental sync (fetch only missing objects based on local vs remote ref diff)

## Capabilities

### New Capabilities

- `federation-pack-transfer`: Iroh-blobs based git packfile transfer between federated clusters during sync
- `federation-mirror-repo`: Local mirror repo creation from fetched federation data, making federated content readable via standard forge operations

### Modified Capabilities

- `federation-ref-fetch`: Extend to trigger pack transfer after ref discovery when content is requested

## Impact

- `crates/aspen-federation/src/sync/` — new pack transfer protocol messages and handler
- `crates/aspen-forge/src/resolver.rs` — serve packfile data via `FederationResourceResolver::sync_objects`
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — mirror repo creation after fetch
- `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs` — extended fetch + new pull command
- `crates/aspen-client-api/src/messages/federation.rs` — new RPC messages for pack transfer and pull
- Depends on iroh-blobs for content-addressed transfer (already a workspace dependency)
