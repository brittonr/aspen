## Why

The Forge web UI can browse repos, commits, issues, and patches — but has no visibility into CI pipelines or cluster health. After pushing code to Forge, there's no way to see whether CI triggered, which stage is running, or why a job failed without dropping to the CLI. The RPC API already exposes `CiListRuns`, `CiGetStatus`, `CiGetJobLogs`, and `CiGetJobOutput`, but nothing in the web frontend consumes them. Adding CI dashboard pages and a cluster overview closes the loop on the self-hosted developer workflow: push → browse → watch CI → check cluster.

Separately, the web frontend is a standalone binary that requires manually passing `--ticket`. Registering it as a named service via `aspen-net` makes it discoverable cluster-wide and gives it a stable `forge.aspen` address that other services and users can reach through the SOCKS5 proxy or DNS resolver.

## What Changes

- **CI pipeline list page**: shows recent pipeline runs per repo, filterable by status
- **CI run detail page**: stages rendered as a pipeline diagram, each job with status badge, timing, and expand-to-view-logs
- **CI job log viewer**: streaming-style log output with ANSI color rendering, auto-scroll, chunked loading via `CiGetJobLogs`
- **CI badge on repo overview**: latest pipeline status shown on the repo overview page (pass/fail/running badge next to the branch)
- **Cluster overview page**: node list with health status, Raft role (leader/follower/learner), uptime, and connected peer count — uses `GetHealth` and `GetMetrics` RPCs
- **Service registration**: on startup, publish `forge-web` service to `aspen-net` registry so it's reachable at `forge.aspen` via SOCKS5/DNS; unpublish on shutdown
- **Auto-refresh for live pages**: CI run detail and cluster overview poll for updates (meta-refresh or minimal JS fetch, no websockets)

## Capabilities

### New Capabilities

- `ci-dashboard`: CI pipeline list, run detail, job log viewer, and repo-level CI status badge
- `cluster-overview`: Cluster health and node status page
- `service-registration`: Auto-register forge-web as a named service in aspen-net on startup

### Modified Capabilities

## Impact

- `crates/aspen-forge-web/src/routes.rs`: new route arms for `/ci`, `/{repo}/ci`, `/{repo}/ci/{run_id}`, `/{repo}/ci/{run_id}/{job_id}/logs`, `/cluster`
- `crates/aspen-forge-web/src/state.rs`: new methods wrapping CI and cluster RPCs (`list_runs`, `get_status`, `get_job_logs`, `get_health`)
- `crates/aspen-forge-web/src/templates.rs`: new maud templates for CI and cluster pages, CSS additions for pipeline visualization and log rendering
- `crates/aspen-forge-web/src/main.rs`: add `aspen-net` service publish on startup, unpublish on shutdown
- `crates/aspen-forge-web/Cargo.toml`: add `aspen-net` dependency (optional, behind feature flag)
- No changes to `aspen-client-api` — all needed RPCs already exist
