## Why

Multiple client-side QUIC RPC paths still bound only `connect()` or the final `read_to_end()` call, but leave `open_bi()`, `write_all()`, and `finish()` unbounded. The repo already fixed this exact failure mode once in deploy RPC status polling after QUIC flow-control stalls left requests stuck in `Draining` even though the deploy had progressed.

A fresh audit found the same pattern in general-purpose clients:

- `crates/aspen-client/src/client.rs:248-274`
- `crates/aspen-cli/src/bin/aspen-cli/client.rs:335-430`
- `crates/aspen-federation/src/sync/client.rs:57-375`
- `crates/aspen-rpc-handlers/src/proxy.rs:199-221`

Under load or partial peer failure, these code paths can hang indefinitely while opening streams or flushing request bodies. That turns transient transport backpressure into user-visible hangs and can wedge retry loops that only budget connect/read time.

## What We Know

- `aspen-client` wraps `endpoint.connect()` and `recv.read_to_end()` in `timeout(...)`, but `connection.open_bi().await`, `send.write_all(...).await`, and `send.finish()` are unbounded.
- `aspen-cli` has the same gap in both normal RPCs and blob fetch paths; `send_get_blob()` also reads up to 256 MiB with no timeout.
- `aspen-federation` sync clients open fresh streams for handshake, list, state, sync, and push operations without any outer timeout.
- `aspen-rpc-handlers::proxy` only bounds connect and the final read.
- The repo already has a closely related incident recorded in `.agent/napkin.md` on 2026-03-29: deploy RPC stalled because `open_bi()`, `write_all()`, and `finish()` had no timeout.

## What Changes

Introduce a shared timeout pattern for client-side QUIC RPCs:

- Wrap the full request/response exchange in a single timeout budget, or at minimum bound `open_bi()`, `write_all()`, `finish()`, and `read_to_end()` separately.
- Apply the same policy to `aspen-client`, CLI RPC helpers, federation sync clients, and proxy forwarding.
- Add regression tests that simulate a peer accepting a connection but never opening or draining the bidirectional stream.
- Audit remaining `open_bi()` call sites for the same gap.

## Scope

- **In scope**: Iroh/QUIC request-response paths that currently leave stream setup or request writes unbounded.
- **Out of scope**: Server-side handler business logic, retry policy tuning unrelated to timeout coverage.

## Evidence

```text
crates/aspen-client/src/client.rs:257  connection.open_bi().await
crates/aspen-client/src/client.rs:268  send.write_all(&request_bytes).await
crates/aspen-client/src/client.rs:270  send.finish()
crates/aspen-client/src/client.rs:274  timeout(... recv.read_to_end(...))

crates/aspen-cli/src/bin/aspen-cli/client.rs:341  connection.open_bi().await
crates/aspen-cli/src/bin/aspen-cli/client.rs:430  recv.read_to_end(256 * 1024 * 1024).await

crates/aspen-federation/src/sync/client.rs:70   connection.open_bi().await
crates/aspen-rpc-handlers/src/proxy.rs:209      connection.open_bi().await
```
