## Why

`connect_to_cluster()` receives peer capabilities in the handshake response but discards them, returning only `(Connection, SignedClusterIdentity)`. `connect_to_cluster_full()` was added to return a `ConnectResult` with capabilities, but hardcodes `["forge", "streaming-sync"]` instead of threading the actual response values. All 10 call sites pass `None` for the credential parameter, so auth tokens never flow through federation connections even though the handler side already verifies them.

## What Changes

- Refactor `connect_to_cluster()` to return `ConnectResult` (connection + identity + capabilities from the actual handshake response)
- Remove the now-redundant `connect_to_cluster_full()` wrapper
- Update all call sites in `aspen-forge-handler` and `aspen-federation` orchestrator to use `ConnectResult` and destructure as needed
- Thread stored credentials through federation connections: orchestrator and forge handlers look up the credential for the target cluster from KV (`_sys:fed:token:received:<key>`) and pass it in the handshake
- Use `ConnectResult::has_capability()` to gate capability-dependent operations (e.g., skip streaming-sync if peer doesn't advertise it)

## Capabilities

### New Capabilities

- `federation-capability-negotiation`: Peer capability exchange and feature-gating based on actual handshake response

### Modified Capabilities

- `federation-auth-enforcement`: Sync client sends stored credential in handshake (the spec already requires this but it's not wired up)

## Impact

- `crates/aspen-federation/src/sync/client.rs` — `connect_to_cluster` signature change, `connect_to_cluster_full` removed
- `crates/aspen-federation/src/sync/mod.rs` — re-exports updated
- `crates/aspen-federation/src/sync/orchestrator.rs` — 2 call sites updated, credential lookup added
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — 5 call sites updated
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — 3 call sites updated
- All existing tests for handshake/federation continue to pass (wire format unchanged)
