## Why

CI log streaming uses 1-second polling via `CiGetJobLogs` RPC. During long builds (8+ minutes for the full dogfood pipeline), the CLI and TUI repeatedly hit the server with scan requests that return empty results. Meanwhile, there's already a push-based `WatchSession` infrastructure (`LOG_SUBSCRIBER_ALPN`) that streams KV changes in real-time — the log writer even references it in its architecture diagram. Connecting these two pieces eliminates the polling overhead and gives sub-second log latency.

## What Changes

- CLI `ci logs --follow` switches from polling to `WatchSession`-based KV prefix watch on `_ci:logs:{run_id}:{job_id}:`, falling back to polling if watch fails
- TUI log viewer uses the same watch-based approach for live streaming when `is_streaming = true`
- CLI `AspenClient` exposes its iroh `Endpoint` and bootstrap addresses so callers can establish `WatchSession` connections
- Historical log catch-up: fetch existing chunks via `CiGetJobLogs` first, then switch to watch for new chunks at the correct start index

## Capabilities

### New Capabilities

- `ci-log-watch`: Real-time CI log streaming via `WatchSession` KV prefix subscription, replacing polling in CLI and TUI

### Modified Capabilities

## Impact

- `crates/aspen-cli/src/bin/aspen-cli/client.rs`: Expose endpoint/cluster_id for WatchSession construction
- `crates/aspen-cli/src/bin/aspen-cli/commands/ci.rs`: Rewrite `ci_logs` follow mode to use WatchSession
- `crates/aspen-tui/src/app/ci_ops.rs`: Wire WatchSession into log viewer refresh loop
- `crates/aspen-client/src/watch.rs`: Already complete — no changes needed
- No new dependencies (aspen-client already has WatchSession, CLI already depends on aspen-client)
- No wire format changes (uses existing LOG_SUBSCRIBER_ALPN protocol)
