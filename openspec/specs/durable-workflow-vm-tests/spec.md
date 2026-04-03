## ADDED Requirements

### Requirement: Leader failover during active workflow

A NixOS VM integration test SHALL verify that a durable workflow survives leader failover and completes on the new leader with correct activity memoization.

#### Scenario: 3-node cluster workflow survives leader kill

- **GIVEN** a 3-node Aspen cluster with a durable workflow executor running a 3-step workflow
- **AND** step 1 is an activity that writes a unique marker to KV
- **AND** step 2 is a durable sleep of 5 seconds
- **AND** step 3 is an activity that writes a completion marker to KV
- **WHEN** the leader is killed after step 1 completes but before the durable sleep fires
- **THEN** a new leader SHALL be elected within the Raft election timeout
- **AND** the new leader SHALL recover the workflow from the event log
- **AND** step 1 SHALL NOT re-execute (the marker write count in KV SHALL be 1, not 2)
- **AND** the durable timer SHALL fire on the new leader
- **AND** step 3 SHALL execute and write its completion marker
- **AND** the workflow SHALL reach terminal state `Completed`

#### Scenario: Workflow state consistent across all nodes

- **GIVEN** the workflow from the leader failover scenario has completed
- **WHEN** any node in the cluster is queried for the workflow's completion marker
- **THEN** the marker SHALL be present with the correct value
- **AND** the event log SHALL be readable from any node via linearizable read

### Requirement: Activity memoization verification under crash

A NixOS VM integration test SHALL verify that activities are not re-executed during replay by tracking execution counts.

#### Scenario: Execution count proves memoization

- **GIVEN** a 3-node cluster running a workflow with 3 activities
- **AND** each activity increments an atomic counter in KV keyed by activity ID
- **WHEN** the leader is killed after activities 1 and 2 complete (counters = 1 each)
- **AND** the new leader recovers and completes the workflow
- **THEN** the counter for activity 1 SHALL be 1 (not re-executed)
- **AND** the counter for activity 2 SHALL be 1 (not re-executed)
- **AND** the counter for activity 3 SHALL be 1 (executed once on new leader)

### Requirement: Durable timer firing across node restart

A NixOS VM integration test SHALL verify that a durable timer scheduled on one node fires correctly after that node restarts.

#### Scenario: Timer fires after scheduling node restarts

- **GIVEN** a 3-node cluster where the leader schedules a durable timer with a 10-second delay
- **AND** the `TimerScheduled` event is recorded in the event store
- **WHEN** the leader node is restarted (not killed — clean restart via systemctl)
- **AND** a new leader is elected
- **THEN** the `TimerService` on the new leader SHALL detect the timer via KV scan
- **AND** the timer SHALL fire within 2 seconds of its scheduled time (accounting for election + recovery)
- **AND** a `TimerFired` event SHALL be recorded in the event store

### Requirement: Saga compensation under failover

A NixOS VM integration test SHALL verify that saga compensation runs correctly when a leader failover occurs mid-compensation.

#### Scenario: Compensation completes on new leader

- **GIVEN** a 3-node cluster running a 3-step saga
- **AND** steps 1 and 2 have completed and step 3 has failed
- **AND** compensation for step 2 has completed
- **WHEN** the leader is killed before compensation for step 1 runs
- **THEN** the new leader SHALL recover the saga from the event log
- **AND** the new leader SHALL find `CompensationCompleted` for step 2 in the events
- **AND** the new leader SHALL execute compensation for step 1 only
- **AND** a `CompensationCompleted` event for step 1 SHALL be recorded
- **AND** the saga SHALL reach terminal state `CompensationCompleted`

### Requirement: Multiple concurrent workflows survive failover

A NixOS VM integration test SHALL verify that multiple concurrent durable workflows all recover correctly after leader failover.

#### Scenario: 5 concurrent workflows recover after leader kill

- **GIVEN** a 3-node cluster with 5 concurrent durable workflows at various stages of progress
- **WHEN** the leader is killed
- **THEN** the new leader SHALL discover and recover all 5 workflows
- **AND** each workflow SHALL resume from its last recorded event
- **AND** all 5 workflows SHALL eventually reach a terminal state (completed or failed)
- **AND** no activity SHALL execute more than once across the failover boundary
