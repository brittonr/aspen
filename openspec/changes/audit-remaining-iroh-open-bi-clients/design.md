## Context

The first timeout-hardening change intentionally focused on four entrypoints named in the proposal: `aspen-client`, CLI RPC/blob fetches, federation sync, and proxy forwarding. That rollout also documented a follow-up bucket for other direct `open_bi()` users so the initial change stayed reviewable.

A fresh repo-wide search still shows remaining direct stream opens in several categories.

Audit command used:

```text
rg -n '\.open_bi\(\)' crates src -g'*.rs'
```

## Inventory

### Request/response clients that match the same timeout pattern

These paths still look like connect or borrow connection -> `open_bi()` -> serialize/write -> `finish()` -> `read_to_end()`:

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

### Lower-level connection abstractions that need a dedicated review

These do not issue the whole request inline, but they still expose or internally perform direct `open_bi()` operations that may need their own timeout policy:

- `crates/aspen-castore/src/client.rs:470,481` (`IrohConnection::open_bi()` and cached reconnect path)

### Already bounded elsewhere or intentionally different protocol shape

These call sites should stay out of this follow-up unless a separate design says otherwise:

- Already bounded elsewhere:
  - `crates/aspen-deploy/src/coordinator/iroh_rpc.rs:205`
  - `crates/aspen-raft/src/connection_pool/peer_connection.rs:102`
  - `crates/aspen-raft/src/iroh_write_forwarder.rs:370` (`send_rpc` wraps the whole forwarding exchange in `tokio::time::timeout(FORWARD_TIMEOUT, ...)` even though `send_rpc_on_stream()` opens the stream directly.)
- Long-lived/session or streaming protocols:
  - `crates/aspen-client/src/watch.rs:417,464`
  - `crates/aspen-net/src/tunnel.rs:201`
  - `crates/aspen-automerge/src/sync_protocol.rs:540`
- Server-side or handler-side stream management:
  - `crates/aspen-dag/src/handler.rs:200`
- Test-only call sites:
  - `crates/aspen-cluster-handler/tests/deploy_rpc_integration.rs:155`
  - `crates/aspen-federation/tests/federation_wire_test.rs:222`
  - `crates/aspen-federation/tests/federation_auth_cluster_test.rs:211`
  - `crates/aspen-hooks/tests/ephemeral_pubsub_integration.rs:56,148,152`

## Goals / Non-Goals

**Goals**

- Reuse the same stage-specific timeout vocabulary from the first rollout: connection timeout, stream open timeout, request write timeout, response timeout.
- Convert the remaining one-request/one-response clients to the bounded post-connect exchange policy.
- Decide whether lower-level abstractions such as `aspen-castore::IrohConnection` should own their own stream-open budget or accept one from callers.

**Non-Goals**

- Reworking long-lived protocols to fit the one-request/one-response helper.
- Expanding the first change retroactively.

## Decisions

### D1: Split follow-up work by protocol shape

One-request/one-response clients can reuse the same helper structure from `harden-iroh-client-rpc-timeouts`. Long-lived session protocols need their own timeout model.

### D2: Prioritize user-facing entrypoints first

`git-remote-aspen`, TUI RPC paths, FUSE, hooks, and CLI hook forwarding are the most visible remaining clients and should land before deeper bridge/service helpers.

### D3: Treat connection abstractions separately from callers

`aspen-castore::IrohConnection` hides stream creation behind an abstraction. The follow-up should decide whether the abstraction itself owns timeout budgets or whether the surrounding IRPC layer should supply them.

## Risks / Trade-offs

- Some bridge clients move large payloads and may need dedicated read budgets instead of a short default RPC timeout.
- Lower-level abstractions may need API changes to propagate timeout budgets cleanly.
