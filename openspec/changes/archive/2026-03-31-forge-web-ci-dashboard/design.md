## Context

The Forge web UI (`aspen-forge-web`, ~2,900 lines) serves HTML over HTTP/3 via iroh QUIC with a TCP proxy for browser access. It uses maud for server-side HTML generation, raw h3 request handling (no axum), and `AspenClient` for cluster RPC. All existing pages (repos, commits, issues, patches, code search) follow the same pattern: route handler calls `AppState` method → `AppState` sends `ClientRpcRequest` → matches `ClientRpcResponse` → passes data to maud template.

The CI system already exposes a complete RPC surface: `CiListRuns`, `CiGetStatus` (with stages/jobs), `CiGetJobLogs` (chunked), `CiGetJobOutput`, `CiTriggerPipeline`, `CiWatchRepo`. Cluster health is available via `GetHealth` and `GetMetrics`. The `aspen-net` service registry has `NetPublish`/`NetLookup`/`NetUnpublish` RPCs.

## Goals / Non-Goals

**Goals:**

- CI pipeline visibility in the browser: list runs, inspect stages/jobs, read logs
- Cluster health at a glance: node roles, health, peer count
- Live updates on CI run and cluster pages via polling (no persistent connections)
- Service mesh registration so `forge.aspen` resolves without manual ticket passing
- Consistent with existing dark-theme maud templates and route dispatch pattern

**Non-Goals:**

- WebSocket or SSE for real-time streaming (h3 handler doesn't support this; polling is sufficient)
- CI configuration editing (`.aspen/ci.ncl` is edited via git push)
- Log search or filtering (plain text rendering only for now)
- Authentication/authorization on CI pages (inherits the same Nostr login as existing pages)
- Metrics graphing or time-series dashboards (just current snapshot)

## Decisions

### 1. Server-side rendering with polling, not client-side JS

All CI and cluster pages are server-rendered maud templates like existing pages. For live updates, the CI run detail page and cluster overview include a `<meta http-equiv="refresh" content="5">` tag when the pipeline is still running or cluster state might change. This avoids adding a JS framework or fetch-based polling loop.

Alternative considered: minimal `fetch()` + DOM replacement. Rejected because maud templates are simpler, the existing codebase has zero JS-driven page updates (only the repo filter and Nostr login), and meta-refresh gives acceptable UX for watching a build.

### 2. ANSI color rendering in logs via server-side conversion

Job logs often contain ANSI escape codes (from cargo, nix, etc.). Convert ANSI to HTML `<span>` elements with inline color styles server-side using the `ansi-to-html` crate. This avoids shipping a JS ANSI parser and keeps the template pattern consistent.

Alternative considered: strip ANSI and show plain text. Rejected because colored build output is significantly more readable — red errors, green success, yellow warnings.

### 3. Pipeline visualization as a flat stage→job table, not a DAG

Render CI stages as sequential sections, each containing a table of jobs. Jobs within a stage run in parallel but are displayed as a flat list. This matches how `.aspen/ci.ncl` defines pipelines (ordered stages with named jobs).

Alternative considered: graphical DAG with SVG/CSS arrows. Rejected — overkill for the stage-based model, adds rendering complexity, and existing CSS is pure functional (no layout JS).

### 4. Chunked log loading with "load more" link

Job logs are fetched via `CiGetJobLogs` which returns chunks (up to 100 per request). Initial page load fetches the first 100 chunks. A "load more" link at the bottom triggers a new page load with `?start=100` query parameter. No infinite scroll.

### 5. Service registration via optional `net` feature flag

The `aspen-net` dependency is behind a `net` feature flag on `aspen-forge-web`. When enabled, the binary calls `NetPublish` at startup and `NetUnpublish` on graceful shutdown. When disabled, the binary works exactly as before (standalone with `--ticket`). The service name is `forge-web` with protocol `http`.

### 6. CI status badge on repo overview

The repo overview page makes a best-effort `CiListRuns` call (limit 1, repo filtered) to get the latest pipeline status. If CI isn't available or returns no runs, the badge is simply omitted. No error — just absent.

### 7. Route structure

```
/ci                              → all recent pipeline runs (cross-repo)
/{repo_id}/ci                    → pipeline runs for a specific repo
/{repo_id}/ci/{run_id}           → run detail with stages and jobs
/{repo_id}/ci/{run_id}/{job_id}  → job log viewer
/cluster                         → cluster overview (nodes, health, roles)
```

These nest under the existing route dispatch in `routes.rs` without conflicting with forge routes (repo IDs are BLAKE3 hashes, never "ci" or "cluster").

## Risks / Trade-offs

- **Polling creates load under many concurrent viewers** → Mitigated: 5-second refresh interval is low, and each refresh is a single RPC call. CI run detail stops auto-refreshing once the run reaches a terminal state (succeeded/failed/cancelled).

- **ANSI-to-HTML adds a new dependency** → Small, well-maintained crate. Alternative is writing our own parser, which is more code for no benefit.

- **Meta-refresh causes full page reload** → Acceptable for a build status page. The flash is brief and users expect it (GitHub Actions does the same). Could upgrade to fetch-based partial replacement later without API changes.

- **`aspen-net` registration fails if net feature isn't on the cluster node** → Registration is fire-and-forget with a warning log. The web UI remains fully functional without service mesh — it just won't be discoverable at `forge.aspen`.

- **Job log chunks may be very large** → `CiGetJobLogs` already caps at 100 chunks per request. Template truncates individual chunk display at 1MB. Total log page capped at ~10MB rendered HTML.
