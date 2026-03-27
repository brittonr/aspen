## Why

Federation sync operations (pull, push, bidi-sync) currently rely on the TrustManager's binary Trusted/Public/Blocked model. The auth crate has a full UCAN-inspired capability token system, and the handshake protocol already accepts an optional `Credential`, but the sync client never sends one. Push operations check `has_credential` as a boolean flag without verifying _what_ the credential authorizes. There's no way to grant a remote cluster pull-only access, push access to specific repos, or time-bounded sync permissions. The client-side `connect_to_cluster` always sends `credential: None`.

## What Changes

- Sync client sends credentials during handshake when available
- Federation-specific `Capability` variants for pull/push/sync scoped to repo prefixes
- Push handler checks credential capabilities, not just presence
- CLI commands to grant, inspect, and use federation tokens for sync
- Token storage in KV so credentials persist across restarts and get presented automatically on reconnect

## Capabilities

### New Capabilities

- `federation-auth-enforcement`: Wire-level enforcement of capability tokens during federation sync — client sends credentials, handler checks specific pull/push/sync authorization per repo

### Modified Capabilities

- `federation-credential`: Credential is now sent by the sync client during handshake (was always `None`)
- `federation-push`: Push handler checks credential capabilities match the target repo, not just `has_credential`
- `federation-token-lifecycle`: Tokens stored in KV and auto-loaded for outbound connections

## Impact

- `crates/aspen-auth/src/capability.rs`: New `FederationPull`, `FederationPush` capability variants and matching `Operation` variants
- `crates/aspen-federation/src/sync/client.rs`: `connect_to_cluster` accepts optional `Credential` and sends it in handshake
- `crates/aspen-federation/src/sync/handler.rs`: `handle_push_objects` and `check_resource_access` verify credential capabilities
- `crates/aspen-federation/src/sync/orchestrator.rs`: Loads stored credentials for outbound connections
- `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs`: New `federation grant` / `federation token inspect` subcommands
- `crates/aspen-client-api/src/messages/federation.rs`: Already has `FederationGrant` — needs handler wiring
- `crates/aspen-federation/tests/`: Auth enforcement integration tests
