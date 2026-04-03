## Why

aspen-jobs has all the primitives for durable execution — an event store with 18 event types, a replay engine with activity memoization, durable timers, saga compensation, and workflow state machines — but none of them are wired together. The event store, replay engine, and durable timers are each tested in isolation but no code path records events during actual workflow or saga execution, no code path replays from the event log on crash recovery, and no code path uses durable timers for workflow delays. The CI pipeline gets durability from the simpler job persistence + queue ack/nack model, which works but can't offer replay-based recovery or memoized re-execution after a leader failover mid-workflow.

Connecting these primitives gives us a durable execution engine comparable to Temporal/Restate: workflows survive leader crashes, activities aren't re-executed on recovery (memoized results replayed), timers fire even if the scheduling node dies, and sagas record compensation events for auditable rollback. This is the foundation for long-running multi-step pipelines, deploy orchestration, and user-defined workflows that span minutes to hours.

## What Changes

- Wire `WorkflowEventStore` into `WorkflowManager` so every state transition, activity completion, and timer event is recorded as an append-only event
- Wire `WorkflowReplayEngine` into workflow startup so a recovering leader replays the event log, skips already-completed activities (memoized), and resumes from the last recorded state
- Wire `DurableTimerManager` into workflow execution so `sleep()` and `timeout()` operations use KV-persisted timers instead of in-process `tokio::time`
- Wire `SagaExecutor` compensation events into the event store for auditable rollback history
- Add a `DurableWorkflowExecutor` orchestration layer that ties all four subsystems together behind a single API
- Add NixOS VM integration tests exercising leader failover during active workflow execution, verifying activity memoization on replay, and confirming timer firing across node restarts

## Capabilities

### New Capabilities

- `durable-workflow-executor`: The orchestration layer that ties event store, replay engine, durable timers, and saga compensation into a unified durable execution API
- `durable-workflow-vm-tests`: Multi-node NixOS VM integration tests for durable workflow execution under leader failover, node restarts, and crash recovery

### Modified Capabilities

- `jobs`: Job system gains durable workflow execution mode where workflows record events and can be replayed after crashes
- `coordination`: Durable timers become a first-class coordination primitive wired into workflow execution

## Impact

- **Crates modified**: `aspen-jobs` (WorkflowManager, SagaExecutor, DurableTimerManager integration), `aspen-ci` (PipelineOrchestrator can opt into durable mode)
- **New files**: `crates/aspen-jobs/src/durable_executor.rs` (orchestration layer), `nix/tests/durable-workflow-failover.nix`, `nix/tests/durable-workflow-replay.nix`
- **APIs**: New `DurableWorkflowExecutor` public API; existing `WorkflowManager` gains optional event recording
- **Dependencies**: No new external deps — all primitives already exist in-crate
- **Risk**: The event store → replay path must handle schema evolution correctly (upcaster already exists but is untested under real workloads). Timer polling interval (100ms) may need tuning for large workflow counts.
