## Context

Aspen already diagnosed one QUIC stall class in deploy RPCs: `connect()` and `read_to_end()` were bounded, but `open_bi()`, `write_all()`, and `finish()` were not. Under flow-control pressure the request stayed stuck in the middle of the exchange and higher-level polling logic reported a bogus deploy timeout.

A fresh audit found the same pattern in the common client paths used outside deploys:

- `aspen-client` request/response RPCs
- `aspen-cli` cached RPC connections and blob fetches
- federation sync client helpers
- proxy forwarding to remote clusters

These paths do not all share the same retry or connection-cache behavior, so the fix needs a consistent timeout policy without papering over transport errors or returning poisoned cached connections to reuse.

## Goals / Non-Goals

**Goals:**

- Ensure no client-initiated QUIC RPC stage can block forever after a connection has been obtained.
- Reuse a consistent timeout pattern across `aspen-client`, CLI, federation sync, and proxy forwarding.
- Preserve existing error context so callers can distinguish connect timeout, stream-open timeout, request-write timeout, and response timeout.
- Ensure timed-out cached connections are discarded instead of returned to the pool.

**Non-Goals:**

- Changing server-side handler semantics.
- Retuning every timeout constant in the repo.
- Solving large-transfer throughput issues beyond making them bounded and diagnosable.

## Decisions

### D1: Bound the full post-connect exchange, not just the final read

Once a connection exists, the request still has three more blocking stages before the response body is read: `open_bi()`, `write_all()`, and `finish()`. The fix should treat these as part of one request/response exchange rather than as unbounded setup.

**Decision:** add a shared helper pattern that runs the full post-connect exchange under an explicit timeout budget and returns stage-specific context on failure.

This keeps the rule simple: after connect succeeds, the remainder of the exchange is always bounded.

### D2: Keep connection acquisition semantics separate from stream-exchange semantics

Some call sites create a fresh connection per request. Others, such as the CLI, cache connections and only open new streams per request.

**Decision:** keep the existing connection acquisition path and timeout policy, then apply the new bounded exchange helper after a connection has been acquired.

That avoids mixing connection-establishment retries with per-request stream behavior and lets cached-connection users discard only the specific connection that timed out.

### D3: Treat mid-exchange timeout as connection corruption for cache purposes

A connection that timed out while opening a stream or flushing a request is not trustworthy enough to return to the cache.

**Decision:** any timeout or I/O failure after a cached connection has been borrowed causes that connection to be closed and discarded.

This matches the existing CLI behavior on some stream errors and extends it to timeout paths.

### D4: Add regression tests with non-responsive peers

Normal unit tests rarely exercise QUIC flow-control stalls because happy-path peers quickly open streams and drain request bodies.

**Decision:** add targeted tests that simulate peers which:

- accept the connection but never open the bidirectional stream,
- open the stream but never drain request bytes,
- or never produce a response body.

Those tests should assert that the client returns within the configured timeout budget and surfaces the correct error context.

### D5: Record remaining direct `open_bi()` users that are not part of this rollout

A repo-wide `open_bi()` audit still shows other direct stream-open call sites after updating `aspen-client`, CLI RPC/blob fetches, federation sync helpers, and proxy forwarding.

**Decision:** document the remaining call sites in three buckets:

- **Already bounded elsewhere:** deploy coordinator RPCs and the raft connection pool already wrap stream opens in timeouts.
- **Different protocol shape:** long-lived streaming/session paths such as `aspen-client/src/watch.rs`, `aspen-net/src/tunnel.rs`, `aspen-automerge/src/sync_protocol.rs`, and server-side handlers do not use one-request/one-response exchange semantics, so they need a separate timeout review.
- **Follow-up request/response clients:** `aspen-tui/src/iroh_client/{rpc,multi_node}.rs`, `aspen-fuse/src/client.rs`, `aspen-hooks/src/client.rs`, and `crates/aspen-cli/src/bin/aspen-cli/commands/hooks.rs` still open streams directly and should be handled in a separate audit change rather than expanding this one further.

This keeps the current change focused on the four client entrypoints named in the proposal while preserving a concrete follow-up list.

## Risks / Trade-offs

- **[Risk]** Reusing one timeout budget for the full exchange may shorten effective time available to read large responses. -> Mitigation: keep the connect budget separate and allow blob/streaming helpers to use a dedicated read budget when needed.
- **[Risk]** More timeout wrapping can make error stacks noisy. -> Mitigation: standardize error messages per stage and avoid nested generic `context("timeout")` wrappers.
- **[Risk]** Discarding timed-out cached connections may increase reconnect churn under temporary load. -> Mitigation: this is preferable to reusing a wedged connection and matches existing safety-first transport handling.
