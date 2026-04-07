## Why

`harden-iroh-client-rpc-timeouts` closed the timeout gap for the shared application client, CLI RPC/blob fetches, federation sync, and proxy forwarding. A follow-up `open_bi()` audit still shows several out-of-scope request/response clients that open QUIC bidirectional streams directly without the same bounded post-connect exchange policy.

The remaining user-facing and bridge-style clients include:

- `src/bin/git-remote-aspen/main.rs`
- `crates/aspen-tui/src/iroh_client/rpc.rs`
- `crates/aspen-tui/src/iroh_client/multi_node.rs`
- `crates/aspen-fuse/src/client.rs`
- `crates/aspen-hooks/src/client.rs`
- `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs`
- `crates/aspen-snix/src/rpc_{blob,directory,pathinfo}_service.rs`
- `crates/aspen-blob/src/replication/adapters.rs`
- `crates/aspen-castore/src/client.rs` (`IrohConnection::open_bi()` cache path)

Under QUIC flow-control stalls or partially responsive peers, these paths can still block indefinitely in `open_bi()`, request writes, or `finish()` even when connect or final response read is bounded.

## What Changes

- Inventory the remaining direct `open_bi()` call sites after the first timeout-hardening rollout.
- Bucket them into request/response clients, lower-level connection abstractions, and protocol shapes that need a separate timeout design.
- Apply the bounded post-connect exchange policy to the request/response clients that fit the same one-request/one-response model.
- Leave long-lived streaming/session protocols explicitly out of scope unless they get their own timeout design.

## Scope

- **In scope**: direct QUIC request/response clients and bridge helpers that still perform `open_bi()` + write + finish + read without full stage coverage.
- **Out of scope**: server-side handlers, long-lived streaming/session protocols, and test-only call sites.
