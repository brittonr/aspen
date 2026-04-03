## ADDED Requirements

### Requirement: Durable workflow start and completion

The system SHALL provide a `DurableWorkflowExecutor` that starts workflows, records all state transitions as append-only events in the `WorkflowEventStore`, and persists workflow state in the KV store. A workflow that completes all steps SHALL have a `WorkflowCompleted` event recorded.

#### Scenario: Start and complete a simple workflow

- **WHEN** a 2-step workflow is submitted to the `DurableWorkflowExecutor`
- **THEN** the executor SHALL record a `WorkflowStarted` event
- **AND** upon completion of each step, the executor SHALL record an `ActivityCompleted` event with the step's result
- **AND** upon completion of all steps, the executor SHALL record a `WorkflowCompleted` event
- **AND** the total event count SHALL equal the number of steps plus 2 (started + per-step + completed)

#### Scenario: Workflow failure records error event

- **WHEN** a workflow step fails with a non-retryable error
- **THEN** the executor SHALL record an `ActivityFailed` event
- **AND** the executor SHALL record a `WorkflowFailed` event with the error message

### Requirement: Activity memoization during replay

The system SHALL memoize completed activity results in the event log. During replay after crash recovery, activities that have an `ActivityCompleted` event SHALL return their cached result without re-executing the activity function.

#### Scenario: Activity not re-executed on replay

- **GIVEN** a workflow with 3 activities where activities 1 and 2 have completed and their `ActivityCompleted` events are in the event store
- **WHEN** the executor replays the workflow after a crash
- **THEN** activities 1 and 2 SHALL return their memoized results from the event cache
- **AND** each activity's execution function SHALL NOT be called during replay
- **AND** activity 3 SHALL execute normally in live mode

#### Scenario: Side effects are deterministic during replay

- **GIVEN** a workflow that recorded a `SideEffectRecorded` event with a UUID value during live execution
- **WHEN** the workflow is replayed after a crash
- **THEN** the side effect SHALL return the same UUID value from the event cache
- **AND** the side effect SHALL NOT generate a new UUID

### Requirement: Durable sleep via persisted timers

The system SHALL provide a `sleep(duration)` operation that persists the delay as a `DurableTimer` in the KV store and records `TimerScheduled` and `TimerFired` events. The sleep SHALL survive node restarts.

#### Scenario: Durable sleep completes normally

- **WHEN** a workflow calls `sleep(5 seconds)`
- **THEN** the executor SHALL schedule a `DurableTimer` in the KV store
- **AND** the executor SHALL record a `TimerScheduled` event
- **AND** after 5 seconds, the `TimerService` SHALL fire the timer
- **AND** the executor SHALL record a `TimerFired` event
- **AND** execution SHALL resume after the sleep

#### Scenario: Durable sleep survives node restart

- **GIVEN** a workflow called `sleep(10 seconds)` and the `TimerScheduled` event was recorded
- **WHEN** the node restarts before the timer fires
- **THEN** the recovering executor SHALL find the `TimerScheduled` event during replay
- **AND** the `TimerService` on the new leader SHALL detect the ready timer via KV scan
- **AND** the timer SHALL fire and execution SHALL resume

#### Scenario: Durable sleep skipped during replay if already fired

- **GIVEN** a workflow where the `TimerFired` event is already in the event store
- **WHEN** the executor replays the workflow
- **THEN** the sleep operation SHALL return immediately without scheduling a new timer

### Requirement: Crash recovery via event replay

The system SHALL recover active workflows after leader failover by replaying the event log from the last snapshot. The new leader SHALL detect active workflows, replay their events, reconstruct state, and resume execution from the point of interruption.

#### Scenario: Leader failover mid-workflow

- **GIVEN** a 3-node cluster running a 3-step workflow where step 1 has completed
- **WHEN** the leader node is killed
- **THEN** a new leader SHALL be elected
- **AND** the new leader SHALL discover the active workflow via KV scan
- **AND** the new leader SHALL replay events and find `ActivityCompleted` for step 1
- **AND** step 1 SHALL NOT be re-executed
- **AND** steps 2 and 3 SHALL execute on the new leader
- **AND** the workflow SHALL complete successfully

#### Scenario: Recovery from snapshot

- **GIVEN** a workflow with 1,200 events and a snapshot at event 1,000
- **WHEN** the executor recovers the workflow
- **THEN** it SHALL load the snapshot and replay only events 1,001 through 1,200
- **AND** the resulting state SHALL be identical to replaying all 1,200 events

### Requirement: Saga compensation with event recording

The system SHALL record saga compensation operations as events in the `WorkflowEventStore`. When a saga step fails, compensation SHALL run in LIFO order and each compensation step SHALL produce `CompensationStarted` and `CompensationCompleted` (or `CompensationFailed`) events.

#### Scenario: Saga compensation with events

- **GIVEN** a saga with 3 steps where steps 1 and 2 have completed
- **WHEN** step 3 fails
- **THEN** compensation SHALL run for steps 2 and 1 (LIFO order)
- **AND** the event store SHALL contain `CompensationStarted` and `CompensationCompleted` events for each compensated step
- **AND** the total compensation events SHALL equal 4 (2 started + 2 completed)

#### Scenario: Saga compensation replayed on recovery

- **GIVEN** a saga where step 3 failed and compensation for step 2 completed but the node crashed before compensating step 1
- **WHEN** the executor recovers the workflow
- **THEN** it SHALL replay events and find compensation for step 2 already done
- **AND** it SHALL execute compensation for step 1 only
- **AND** step 2 compensation SHALL NOT be re-executed

### Requirement: Automatic snapshotting

The system SHALL take automatic snapshots of workflow state when the event count since the last snapshot exceeds the configured threshold (default: 1,000 events). Snapshots SHALL be stored in the KV store and SHALL include pending activity states, pending timer states, the compensation stack, and the side effect sequence counter.

#### Scenario: Snapshot taken at threshold

- **GIVEN** a workflow with `MAX_EVENTS_BEFORE_SNAPSHOT` set to 1,000
- **WHEN** the 1,001st event is recorded
- **THEN** the executor SHALL take a snapshot at event 1,000
- **AND** the snapshot SHALL be stored at `__wf_snapshots::{workflow_id}::{snapshot_id}`

#### Scenario: Snapshot contains complete state

- **WHEN** a snapshot is taken for a workflow with 2 pending activities and 1 pending timer
- **THEN** the snapshot `pending_activities` map SHALL contain 2 entries
- **AND** the snapshot `pending_timers` map SHALL contain 1 entry
- **AND** the snapshot `side_effect_seq` SHALL equal the current side effect counter

### Requirement: Workflow discovery for recovery

The system SHALL provide a mechanism for the new leader to discover all active (non-terminal) workflows after election. Discovery SHALL scan the KV store for workflow event prefixes and determine which workflows need recovery.

#### Scenario: Discover active workflows after failover

- **GIVEN** 3 workflows: one completed, one failed, one in-progress at step 2
- **WHEN** a new leader scans for active workflows
- **THEN** it SHALL identify only the in-progress workflow for recovery
- **AND** the completed and failed workflows SHALL NOT be recovered

#### Scenario: No duplicate recovery

- **GIVEN** a workflow that was already recovered and is running on the current leader
- **WHEN** the leader scans for active workflows again
- **THEN** it SHALL detect the workflow is already running and skip recovery
