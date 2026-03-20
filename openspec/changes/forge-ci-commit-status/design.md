## Context

Forge and CI communicate through a one-way channel: Forge gossip emits `RefUpdate` announcements, and CI's `TriggerService` subscribes to them via `CiTriggerHandler`. Pipeline results are persisted to CI's KV namespace (`_ci:runs:*`, `_ci:ref-status:*`) but never surface in Forge.

Forge's collaborative objects (COBs) already model patches with approvals and change requests, but have no concept of automated check results. The `CobOperation` enum has `Approve` and `RequestChanges` but nothing for CI. Merging a patch is unrestricted — no gating on external signals.

The hooks system (`aspen-hooks`) has three event types, all cluster-internal. No forge or CI events exist.

Key existing types:

- `PipelineStatus` (CI): `Initializing | CheckingOut | CheckoutFailed | Pending | Running | Success | Failed | Cancelled`
- `PipelineRun` (CI): has `context.repo_id`, `context.commit_hash`, `context.ref_name`, `status`, `id`
- `Announcement` (Forge gossip): `RefUpdate | CobChange | Seeding | Unseeding | RepoCreated`
- `Patch` (Forge COB): has `approvals`, `change_requests`, `state`, `head` commit

## Goals / Non-Goals

**Goals:**

- Close the CI→Forge feedback loop so pipeline results are visible in Forge
- Store commit statuses in the same KV store Forge uses, queryable by repo + commit + context
- Broadcast pipeline status changes over gossip for real-time reactivity
- Let patches expose aggregated CI status alongside human review status
- Enable branch protection rules that block merges without passing CI
- Add forge and CI event types to the hooks system

**Non-Goals:**

- Auto-merge (merge-when-pipeline-succeeds) — future work that builds on merge gating
- Per-line CI annotations (e.g., test failure mapped to code location)
- CI result caching or deduplication across identical commits
- Multi-cluster federation of commit statuses
- GitHub/GitLab compatibility shims for status APIs

## Decisions

### 1. Commit statuses live in Forge's KV namespace, not CI's

Statuses are keyed under `forge:status:{repo_hex}:{commit_hex}:{context}` rather than `_ci:*`. This keeps Forge self-contained — it can answer "what's the CI status of this commit?" without reaching into CI's namespace. CI writes to this namespace using the same `KeyValueStore` trait it already holds.

Alternative: Store in CI namespace, have Forge cross-read → rejected because it couples Forge queries to CI's storage layout and breaks if CI is disabled.

### 2. Trait-based `StatusReporter` in CI, not a direct Forge dependency

CI defines a `StatusReporter` trait:

```rust
#[async_trait]
pub trait StatusReporter: Send + Sync + 'static {
    async fn report_status(&self, status: CommitStatusReport) -> Result<()>;
}
```

The `ForgeStatusReporter` implementation writes `CommitStatus` entries to Forge KV and optionally publishes gossip. This avoids a hard `aspen-ci → aspen-forge` compile dependency in the trait definition (the trait lives in `aspen-ci`, the impl can live in a bridge crate or in `aspen-ci` behind the `forge` feature).

Alternative: Direct function call from CI orchestrator to Forge node → rejected because it couples CI's pipeline status sync to Forge's type system.

### 3. `CommitStatus` is a plain KV entry, not a COB

Commit statuses are simple KV entries (JSON-serialized), not collaborative objects. They don't need DAG-based conflict resolution — each `(repo, commit, context)` tuple has exactly one current status, and only CI writes it. Last-write-wins semantics through Raft are sufficient.

Alternative: Model as a COB → rejected because COB machinery (change DAGs, conflict resolution, signed operations) adds complexity for a write-once-per-transition value. Statuses don't have multiple concurrent authors.

### 4. New gossip variant `Announcement::PipelineStatus`

A new variant in the `Announcement` enum:

```rust
Announcement::PipelineStatus {
    repo_id: RepoId,
    commit: [u8; 32],
    ref_name: String,
    run_id: String,
    status: CommitCheckState,  // Pending, Success, Failure, Error
    context: String,           // e.g., "ci/pipeline"
}
```

This is additive — existing gossip handlers ignore unknown variants via the `_ => return` match arm in `CiTriggerHandler::on_announcement`. No protocol versioning needed.

Alternative: Poll KV for status changes → rejected because it adds latency and load for real-time use cases (TUI status display, auto-merge triggers).

### 5. Patch CI resolution via KV lookup, not COB operations

When resolving a Patch's display state, the system queries commit statuses from KV for the patch's `head` commit. This is a read-time enrichment, not a write to the COB.

We do NOT add a `CobOperation::CiStatusUpdate` variant. CI status is external to the COB DAG — it changes independently of patch operations and doesn't need causal ordering with human reviews.

Alternative: Add `CiStatusUpdate` as a COB operation → rejected because it pollutes the patch's change history with machine-generated noise (every pending→running→success transition would be a COB change). It also requires CI to hold a signing key with COB write permissions.

### 6. Branch protection stored as repo-level KV config

Protection rules live at `forge:protection:{repo_hex}:{ref_pattern}` as JSON. The `CobStore` checks protection rules when processing a `Merge` operation. Rules are set by repo administrators through the CLI/TUI.

This is enforced at the Forge layer, not at the git-remote-helper layer, because direct ref updates (non-patch merges) bypass COB operations. A future change could add pre-receive hook enforcement for direct pushes.

### 7. Hook events are string-typed, not enum variants

New hook event types use the existing `HookEventType` enum with new variants. The topic mapping function `topic_for_event_type` maps them to pubsub topics like `hooks.forge.ref_updated`, `hooks.ci.pipeline_completed`.

### 8. Status reporter invoked from `sync_run_status` and `track_run`

The orchestrator's existing status sync path (`sync_run_status` in `pipeline/status.rs`) already detects state transitions and persists to KV. The reporter hooks in at two points:

- `track_run`: reports initial `Pending` status when a pipeline is created
- `finalize_status_sync`: reports terminal status (Success/Failed/Cancelled) when pipeline completes

This avoids adding reporting to every intermediate state (Initializing, CheckingOut) — commit statuses only need Pending → Success/Failure/Error transitions, matching GitHub's model.

## Risks / Trade-offs

**[Risk] Gossip message size increase** → The `PipelineStatus` announcement adds ~200 bytes per status change. Gossip already handles `RefUpdate` at similar sizes. Bounded by CI's `max_total_runs` (existing Tiger Style limit).

**[Risk] KV write amplification** → Each pipeline transition writes both CI run data AND a commit status entry. Mitigated by only reporting on meaningful transitions (Pending, Success, Failure, Error), not every intermediate state.

**[Risk] Stale commit statuses** → If a pipeline is abandoned (node crash), the commit status stays `Pending` forever. Mitigated by CI's existing recovery system (`orchestrator/recovery.rs`) which detects orphaned runs and marks them failed — the reporter fires on that transition too.

**[Risk] Branch protection enforcement gap** → Protection is enforced at COB merge time, not at ref update time. A direct `refs.set()` call bypasses protection. Acceptable for now — patches are the primary merge path. Future work: pre-receive hooks on `RefStore::set`.

**[Trade-off] No COB history for CI status** → CI status changes aren't part of the patch's COB DAG, so they don't appear in the patch's change history. This is intentional — it keeps the DAG focused on human decisions. The trade-off is that historical CI results require querying the status KV namespace separately.

**[Trade-off] Single context per pipeline** → Each pipeline writes one commit status with context `"ci/pipeline"`. Multi-stage granularity (separate status per stage) is deferred. The status reflects the overall pipeline result.
