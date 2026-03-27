## Why

Federation is currently pull-only: cluster B connects to cluster A and fetches refs + objects. There's no way for cluster A to push changes to cluster B. This matters for workflows where the origin cluster wants to actively replicate to mirrors — CI triggers, real-time mirroring, or deployments where the source of truth pushes to downstream clusters rather than waiting for them to poll.

## What Changes

- New `PushObjects` federation request type: origin sends refs + git objects to a remote cluster
- Server-side handler that validates the push (trust, policy, content-addressed hashes), imports objects via the existing `GitObjectImporter`, and updates mirror refs
- Client-side `push_to_cluster` function in the sync client module
- CLI command `aspen-cli federation push --peer <key> --repo <id>` to trigger a push
- Node-side RPC handler for `FederationPush` client request

## Capabilities

### New Capabilities

- `federation-push`: Push refs and git objects from origin cluster to a remote cluster over the federation sync protocol

### Modified Capabilities

## Impact

- `crates/aspen-federation/src/sync/types.rs` — new request/response variants
- `crates/aspen-federation/src/sync/handler.rs` — server-side push handler
- `crates/aspen-federation/src/sync/client.rs` — client-side push function
- `crates/aspen-forge/src/resolver.rs` — import-side logic for receiving pushed objects
- `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs` — new `Push` subcommand
- `crates/aspen-client-api/` — new `FederationPush` RPC request/response
- `crates/aspen-rpc-handlers/` — handler wiring for the new RPC
