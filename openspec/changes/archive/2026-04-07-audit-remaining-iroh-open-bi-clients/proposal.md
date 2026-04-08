## Why

`harden-iroh-client-rpc-timeouts` closed the timeout gap for the shared application client, CLI RPC/blob fetches, federation sync, and proxy forwarding. A follow-up `open_bi()` audit still shows several out-of-scope request/response clients that open QUIC bidirectional streams directly without the same bounded post-connect exchange policy.

The remaining user-facing and bridge-style clients include:

- `src/bin/git-remote-aspen/main.rs:286`
- `crates/aspen-tui/src/iroh_client/rpc.rs:32`
- `crates/aspen-tui/src/iroh_client/multi_node.rs:158`
- `crates/aspen-fuse/src/client.rs:244,250`
- `crates/aspen-hooks/src/client.rs:295`
- `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs:603`
- `crates/aspen-snix/src/rpc_blob_service.rs:116`
- `crates/aspen-snix/src/rpc_directory_service.rs:112`
- `crates/aspen-snix/src/rpc_pathinfo_service.rs:110`
- `crates/aspen-blob/src/replication/adapters.rs:287`
- `crates/aspen-castore/src/client.rs:470,481` (`IrohConnection::open_bi()` cache path)

Audit evidence came from:

```text
rg -n '\.open_bi\(\)' crates src -g'*.rs'
```

Under QUIC flow-control stalls or partially responsive peers, these paths can still block indefinitely in `open_bi()`, request writes, or `finish()` even when connect or final response read is bounded.

## What Changes

- Inventory the remaining direct `open_bi()` call sites after the first timeout-hardening rollout.
- Bucket them into request/response clients, lower-level connection abstractions, and protocol shapes that need a separate timeout design.
- Apply the bounded post-connect exchange policy to the request/response clients that fit the same one-request/one-response model.
- Leave long-lived streaming/session protocols explicitly out of scope unless they get their own timeout design.

## Scope

- **In scope**: direct QUIC request/response clients and bridge helpers that still perform `open_bi()` + write + finish + read without full stage coverage.
- **Out of scope**: server-side handlers, long-lived streaming/session protocols, and test-only call sites.
