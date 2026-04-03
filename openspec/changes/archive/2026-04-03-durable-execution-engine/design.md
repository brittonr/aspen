## Context

aspen-jobs contains four independent subsystems that each address a piece of durable execution:

1. **WorkflowEventStore** — append-only event log in KV with HLC timestamps, schema versioning, upcasting, and snapshot support. 18 event types cover the full workflow lifecycle.
2. **WorkflowReplayEngine** — replays events and returns memoized activity results and side effects. Maintains position tracking and replay/live mode switching.
3. **DurableTimerManager + TimerService** — KV-persisted timers with zero-padded keys for time-ordered scans, 100ms polling, and per-workflow timer management.
4. **SagaExecutor** — forward execution and LIFO compensation with branch-backed steps (KV branch isolation for automatic rollback).

None of these are connected. `WorkflowManager` (used by `PipelineOrchestrator`) tracks state transitions via an inline `Vec<StateTransition>` history — no events recorded, no replay possible, no durable timers. The CI pipeline's durability comes entirely from job persistence + queue ack/nack, which is sufficient for fire-and-forget job dispatch but not for multi-step workflows that need to resume mid-execution after a crash.

## Goals / Non-Goals

**Goals:**

- Wire the four subsystems into a `DurableWorkflowExecutor` that records events during execution and replays from the event log on recovery
- Activity memoization: completed activities return cached results during replay instead of re-executing
- Durable sleep/timeout: workflow delays use KV-persisted timers that survive node restarts
- Saga compensation events recorded in the event store for audit trail
- Leader failover recovery: new leader replays active workflows from event log and resumes execution
- Multi-node NixOS VM tests proving correctness under leader kill, node restart, and timer firing across failover
- Opt-in: existing `WorkflowManager` callers (CI pipeline) can adopt incrementally

**Non-Goals:**

- Rewriting the event store, replay engine, or durable timer internals — they work, we're wiring them
- Child workflow support (event types exist but orchestration deferred)
- Continue-as-new (event type exists, wiring deferred)
- Signal handling (event type exists, wiring deferred)
- WASM/plugin-defined workflows — this is the Rust-level execution engine only
- Replacing the CI pipeline's existing durability model — the pipeline can opt in later

## Decisions

### 1. New `DurableWorkflowExecutor` rather than modifying `WorkflowManager`

`WorkflowManager` is a CAS-based state machine for simple step sequencing. It stores workflow state as a single KV entry and transitions atomically. Bolting event sourcing onto it would break its simple model and force all callers into the event-sourced path.

Instead, `DurableWorkflowExecutor` is a new type that composes `WorkflowEventStore`, `WorkflowReplayEngine`, `DurableTimerManager`, and `SagaExecutor`. It owns the full lifecycle: start → execute activities → handle timers → compensate on failure → complete. `WorkflowManager` remains as-is for simple use cases.

Alternative considered: subclassing via trait default methods. Rejected because the execution model is fundamentally different (event-driven replay vs CAS state transitions).

### 2. Event recording at the executor level, not inside each subsystem

Events are recorded by `DurableWorkflowExecutor` at each operation boundary (before/after activity execution, on timer schedule/fire, on compensation start/complete). The event store, replay engine, and timer manager remain unmodified — they don't know about each other.

This keeps the subsystems composable and testable in isolation. The executor is the single integration point.

Alternative considered: having `SagaExecutor` directly write to `WorkflowEventStore`. Rejected because it couples two independent subsystems and makes unit testing harder.

### 3. Replay-then-live execution model

On recovery, `DurableWorkflowExecutor`:

1. Loads the latest snapshot (if any) from KV
2. Replays events from snapshot point using `WorkflowReplayEngine`
3. During replay, activity calls return memoized results from the event cache
4. Once replay catches up to the last recorded event, switches to live mode
5. In live mode, activities execute normally and results are recorded as new events

This is the standard Temporal/Restate model. The replay engine already supports this (position tracking, `is_replaying()`, `get_activity_result()`).

### 4. Leader-only execution with Raft-based leader detection

Durable workflows execute on the Raft leader only. On leader failover:

1. New leader detects active workflows via KV scan (`__wf_events::` prefix)
2. For each active workflow, creates a `DurableWorkflowExecutor` and calls `recover()`
3. `recover()` replays the event log and resumes execution

Follower nodes don't execute workflows. If a follower receives a workflow start request, it forwards to the leader (same pattern as write forwarding).

### 5. Durable sleep via timer + event recording

`DurableWorkflowExecutor::sleep(duration)`:

- In live mode: schedules a `DurableTimer`, records `TimerScheduled` event, then awaits a notification from `TimerService`
- In replay mode: checks for `TimerFired` event in the event cache. If found, returns immediately (the sleep already happened). If not found, the timer is still pending — reschedules it and awaits.

This survives crashes: the timer is in KV, the scheduled event is in KV. On recovery, the replay sees the `TimerScheduled` event and either finds `TimerFired` (skip) or reschedules the timer.

### 6. Saga compensation wired through event recording only

`SagaExecutor` already handles compensation logic correctly. The executor wraps each saga operation with event recording:

- Step completion → `ActivityCompleted` event
- Step failure → `ActivityFailed` event
- Compensation start → `CompensationStarted` event
- Compensation complete → `CompensationCompleted` event

On replay, the saga state is reconstructed from events rather than re-executing compensation logic. The `SagaExecutor` itself is not modified.

### 7. VM tests: 3-node cluster with leader kill mid-workflow

Two NixOS VM tests:

**durable-workflow-failover.nix**: 3-node cluster. Submit a 3-step workflow where step 2 includes a durable sleep (5s). Kill the leader after step 1 completes but before the timer fires. Verify: new leader replays events, step 1 is not re-executed (memoized), timer fires on schedule, step 3 completes, workflow reaches terminal state.

**durable-workflow-replay.nix**: 3-node cluster. Submit a workflow with 3 activities. Kill the leader after activity 2 completes. Verify: new leader replays, activities 1 and 2 return memoized results (check via activity execution count — should be 1 each, not 2), activity 3 executes fresh, workflow completes. Also verify saga compensation: submit a workflow where step 3 fails, verify compensation events are recorded and compensation runs in LIFO order.

## Risks / Trade-offs

**[Risk] Event log growth for long-running workflows** → Snapshot support already exists (threshold at 1,000 events). The executor calls `should_snapshot()` after each event and takes automatic snapshots. MAX_EVENTS_PER_WORKFLOW (50,000) provides a hard ceiling.

**[Risk] Timer polling under high workflow count** → 100ms poll with TIMER_SCAN_BATCH_SIZE=1,000 may become a bottleneck with thousands of concurrent timers. Mitigation: the scan is prefix-bounded and keys are time-ordered, so only ready timers are deserialized. If needed later, partition timers by time bucket.

**[Risk] Replay correctness with non-deterministic operations** → Side effects (UUID generation, current time) must go through `SideEffectRecorded` events. The replay engine already supports this via `get_side_effect()` / `record_side_effect()`. The executor must route all non-deterministic calls through these methods. Missing a side-effect recording would produce a different result on replay — this is the classic Temporal footgun. Mitigation: the executor API makes side effects explicit (no hidden `Utc::now()` calls).

**[Risk] Schema evolution during upgrade** → The upcaster pattern handles this: events written by version N are upcasted to version N+1 on read. Currently only v1 exists. New event types are additive (new enum variants) — old events remain valid. The risk is removing or renaming event fields. Mitigation: never remove fields, only add new ones with `#[serde(default)]`.

**[Risk] Dual-write between event store and workflow state** → The executor must write events and update workflow state atomically. Since both are in the same Raft KV store, they go through the same consensus log. But they're separate KV writes, not a single transaction. Mitigation: event is the source of truth. If the state write fails after the event write, recovery replays events and reconstructs state. State is a cache, not the authority.
