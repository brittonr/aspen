## 1. Expose AspenClient internals

- [x] 1.1 Add `endpoint()`, `first_peer_addr()` methods to CLI's `AspenClient` and remove `#[allow(dead_code)]` from `cluster_id()`

## 2. CLI watch-based follow

- [x] 2.1 Add `aspen-client` dependency to `aspen-cli` Cargo.toml (for `WatchSession` type) if not already present
- [x] 2.2 Rewrite `ci_logs` follow path: historical catch-up via `CiGetJobLogs`, then `WatchSession::connect` + `subscribe` on log prefix, parse Set events as `CiLogChunk` JSON, print content, stop on `__complete__` marker key
- [x] 2.3 Add fallback: if `WatchSession::connect` fails, log at debug level and fall back to existing polling loop
- [x] 2.4 Handle mid-stream watch disconnect: catch `EndOfStream` event, resume polling from `last_chunk_index`

## 3. TUI watch-based streaming

- [x] 3.1 Add watch background task: when `open_ci_log_viewer` sets `is_streaming = true`, spawn a `tokio::spawn` task that holds a `WatchSubscription` and sends parsed `CiLogLine` entries via `mpsc` channel
- [x] 3.2 Drain watch channel in TUI event loop tick, append lines to `log_stream.lines`, respect auto-scroll and bounded buffer
- [x] 3.3 Clean up watch task on `close_ci_log_viewer` (drop the channel sender to signal task exit)

## 4. Testing

- [x] 4.1 Unit test: verify `CiLogChunk` can be deserialized from a `WatchEvent::Set` value payload
- [x] 4.2 Integration test: write chunks via `CiLogWriter` to a `DeterministicKeyValueStore`, subscribe via watch prefix, verify chunks arrive in order
