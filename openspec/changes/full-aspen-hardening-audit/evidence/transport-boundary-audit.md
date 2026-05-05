# Transport boundary audit

Generated: 2026-05-05T12:33:52Z

## Scope

Audited representative Iroh transport boundaries for ALPN routing, Raft auth negotiation, proxy/federation handshakes, blob/DAG/docs/snapshot-style streaming assumptions, rate limits, and metrics drift.

## Source handles

- `src/bin/aspen_node/setup/router.rs` — production `aspen-node` ALPN registration path.
- `src/node/mod.rs` — programmatic `NodeBuilder` router registration path.
- `crates/aspen-raft/src/connection_pool/pool_management.rs` — outgoing Raft ALPN selection and connection timeout.
- `crates/aspen-raft/src/membership_watcher.rs` — TrustedPeersRegistry membership synchronization for authenticated Raft.
- `crates/aspen-rpc-handlers/src/proxy.rs` — client-RPC proxy handoff and verified-token propagation.
- `crates/aspen-transport/src/lib.rs` — transport ALPN constants and protocol handler exports.

## Finding remediated in this slice

### `--enable-raft-auth` did not register authenticated Raft ALPN in the production binary router

The bootstrap/network factory already switches outgoing Raft connections to `RAFT_AUTH_ALPN` when `config.iroh.enable_raft_auth` is true. The production binary router in `src/bin/aspen_node/setup/router.rs`, however, registered only legacy `RAFT_ALPN` for incoming single-node and sharded Raft traffic. That created a configuration drift where nodes started with `--enable-raft-auth` would attempt authenticated outbound Raft connections but not accept authenticated inbound Raft connections on the same binary setup path.

Remediation:

- `setup_router` now imports and registers `RAFT_AUTH_ALPN` with `AuthenticatedRaftProtocolHandler` whenever `config.iroh.enable_raft_auth` is true.
- Registration is covered for both `NodeMode::Single` and primary-shard `NodeMode::Sharded`.
- Each authenticated handler is backed by a `TrustedPeersRegistry` seeded with the node's own Iroh public key and a membership watcher (`spawn_membership_watcher`) so accepted peer keys track Raft membership.
- Legacy `RAFT_ALPN` remains registered for backward compatibility, matching the existing programmatic `NodeBuilder` behavior.
- Added source-level regression `enable_raft_auth_registers_authenticated_alpn` to fail if the auth flag no longer gates authenticated ALPN registration or membership synchronization.

## Other audited transport assumptions

- Outgoing Raft ALPN selection already fails visibly through `create_connection_select_alpn`: auth-enabled pools select `RAFT_AUTH_ALPN`; legacy fallback logs an unauthenticated Raft warning.
- Federation ALPN registration remains feature/config gated and initializes explicit cluster identity/trust-manager context before handler registration.
- Proxy forwarding is not a raw unauthenticated ALPN: it is reached through client RPC dispatch and carries verified token context into proxy execution.
- Blob/DAG/docs/castore/cache ALPNs are opt-in by feature and resource availability; this slice did not assert content authorization semantics beyond registration and handler ownership.
- Snapshot transfer metrics/rate-limit posture still needs deeper follow-up if a later slice focuses on bulk-stream DoS/resumability rather than ALPN/auth drift.

## Verification

- `cargo test --bin aspen-node enable_raft_auth_registers_authenticated_alpn --features node-runtime-apps,blob,automerge,forge,federation -- --nocapture`
- `cargo check --bin aspen-node --features node-runtime-apps,blob,automerge,forge,federation`

## Residual risks / follow-up

- The full `--features full` binary check is blocked by pre-existing `aspen-wasm-plugin` API drift (`aspen_core::app_registry` / `aspen_core::ServiceExecutor`), not by this transport change.
- The authenticated Raft path intentionally keeps legacy `RAFT_ALPN` registered for compatibility. A future child change can introduce a stricter production mode that refuses legacy Raft ALPN once migration risk is acceptable.
- Bulk-transfer rate limiting for snapshot/blob/DAG streams remains a worthwhile deeper audit slice.
