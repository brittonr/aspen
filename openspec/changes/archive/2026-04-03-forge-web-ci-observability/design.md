## Context

The forge web frontend (`crates/aspen-forge-web`) serves HTML over iroh-h3 using maud templates. All data comes from `AspenClient` RPC calls — the web server has no database of its own. CI state lives in the cluster's KV store and is exposed through `ClientRpcRequest` variants (`CiGetStatus`, `CiListRuns`, `CiGetJobLogs`, etc.).

The current CI pages were built as part of the `ci-dashboard` spec. They cover the read path — listing runs, viewing stages/jobs, streaming logs. What's missing is the operational layer: actions (cancel, retrigger), richer data display (errors, durations, artifacts, commit badges), and visual progress indication.

All the data needed for these features already flows through the API. The client-api crate has `CiCancelRun`, `CiTriggerPipeline`, `CiListArtifacts`, `CiGetArtifact`, `CiGetJobOutput`, and `CiGetRefStatus`. The forge KV stores `CommitStatus` entries written by `ForgeStatusReporter`. None of this is wired to the web UI yet.

## Goals / Non-Goals

**Goals:**

- Surface all available CI data in the web UI — no information left trapped in the API
- Add cancel and re-trigger actions with POST forms (no JS required)
- Keep the server-rendered, no-JS-required approach (meta-refresh for live updates)
- Fit naturally into the existing dark GitHub-style design

**Non-Goals:**

- WebSocket or SSE streaming for real-time log tailing (current meta-refresh is adequate)
- Authentication/authorization on cancel/retrigger (login system exists but gating actions is a separate change)
- Pagination beyond the existing 50-run limit
- CI configuration editing from the web UI

## Decisions

### 1. POST forms for cancel and retrigger

Cancel and retrigger will use plain HTML `<form method="POST">` submissions, same pattern as issue close/reopen and patch merge. The routes:

- `POST /{repo_id}/ci/{run_id}/cancel` → calls `CiCancelRun`, redirects back to run detail
- `POST /{repo_id}/ci/{run_id}/retrigger` → calls `CiTriggerPipeline` with same repo_id and ref_name, redirects to new run detail

No confirmation dialog — cancel is reversible in the sense that you can retrigger. The button only appears on non-terminal runs (cancel) or terminal runs (retrigger).

**Alternative considered:** JavaScript `fetch()` with optimistic UI. Rejected because the entire forge-web is server-rendered with no JS dependencies beyond the optional Nostr login.

### 2. Stage timeline as a horizontal bar

The pipeline detail page gets a horizontal progress bar above the stage sections:

```
┌──────────────┬──────────────┬──────────────┐
│  ■ checkout  │  ■ build     │  □ deploy    │
│  ✓ 12s       │  ● 2m 34s   │  ○ pending   │
└──────────────┴──────────────┴──────────────┘
```

Each segment is a `<div>` with `flex-grow: 1` inside a flexbox row. Color comes from the stage status CSS class. Width is equal for all stages (not proportional to duration) since stages can have wildly different durations and proportional width would make short stages invisible.

**Alternative considered:** Vertical timeline with connecting lines. Rejected because horizontal matches the left-to-right reading of a pipeline and takes less vertical space.

### 3. Commit status badges via KV scan

The commit detail page already fetches commit info. Adding CI status means one extra KV scan at `forge:status:{repo_hex}:{commit_hex}:` prefix. The `AppState` gets a new `get_commit_statuses(repo_id, commit_hash)` method that scans this prefix and deserializes `CommitStatus` entries.

Badges render next to the commit hash in the detail header, same visual style as the repo overview CI badge.

### 4. Artifacts on job log page

Below the log viewer, an "Artifacts" section lists artifacts from `CiListArtifacts`. Each row shows name, size (human-readable), and content type. Download links use `CiGetArtifact` to get a blob ticket, then construct a download URL. Since the forge-web serves over h3, and blob tickets are for iroh-blobs, the download link will point to `/{repo_id}/ci/{run_id}/{job_id}/artifact/{blob_hash}` which proxies via `CiGetArtifact` → blob ticket → blob fetch → stream to client.

For the first pass, artifacts will show metadata only (name, size, type) without inline download — the blob-to-HTTP proxy is a larger piece of work. Download can be added when the h3 proxy supports streaming blob responses.

### 5. Log enhancements: line numbers and full output

Line numbers: wrap each log chunk line in a `<tr>` with a line number `<td>`, same as the existing code viewer. A running counter across chunks.

Full output toggle: a link "View full output" at the top of the log viewer that appends `?full=1` to the URL. The handler checks for this param and calls `CiGetJobOutput` instead of `CiGetJobLogs`, returning the complete stdout/stderr. Capped at 1MB display to avoid browser memory issues.

### 6. Duration in pipeline list

`CiRunInfo` currently has only `created_at_ms`. The pipeline detail (`CiGetStatusResponse`) has both `created_at_ms` and `completed_at_ms`. For the list page, we compute duration client-side: running pipelines show elapsed since `created_at_ms` (via meta-refresh), completed ones need `completed_at_ms`.

Two options:

- **(Chosen)** Add `completed_at_ms` to `CiRunInfo` in `aspen-client-api`. This is a backwards-compatible field addition.
- Fetch full status for each run in the list. Rejected — N+1 RPC calls.

### 7. Error rendering

`CiGetStatusResponse.error` and `CiJobInfo.error` are already in the API types. The pipeline detail template will render errors in a red callout box below the status badge. Job errors render inline in the job row.

`PipelineRun.error_message` (checkout failures, etc.) maps to `CiGetStatusResponse.error`.

## Risks / Trade-offs

- **[CiRunInfo field addition]** Adding `completed_at_ms` to `CiRunInfo` requires a change in `aspen-client-api` and the CI handler that builds the response. Low risk — it's an additive `Option<u64>` field with `#[serde(default)]`. → Mitigation: add with default None so old serialized data still deserializes.

- **[Artifact download deferred]** Artifacts will show metadata but not download links initially. The h3-to-blob proxy isn't in place. → Mitigation: show "download via CLI: `aspen-cli ci artifact <hash>`" as interim.

- **[Cancel without confirmation]** No "are you sure?" dialog since we have no JS. → Mitigation: the cancel button is styled as destructive (red outline), and retrigger is always available to undo.

- **[KV scan for commit statuses]** Each commit detail page does an extra KV scan. → Mitigation: the scan prefix is narrow (`forge:status:{repo}:{commit}:`) and returns at most a handful of entries per commit. No performance concern.
