## ADDED Requirements

### Requirement: Automatic deployment resume on leader election

When a node becomes the Raft leader, it SHALL call DeploymentCoordinator::check_and_resume() to detect and finalize in-progress deployments left by a previous leader that upgraded itself and restarted.

#### Scenario: In-progress deployment found

- **WHEN** a node becomes leader
- **AND** `_sys:deploy:current` contains a record with status Deploying
- **THEN** the new leader SHALL resume the deployment from persisted node states
- **AND** nodes already marked Healthy SHALL be skipped
- **AND** nodes in Pending state SHALL be upgraded normally
- **AND** nodes in Draining/Upgrading/Restarting SHALL be health-polled to determine current state

#### Scenario: No in-progress deployment

- **WHEN** a node becomes leader
- **AND** `_sys:deploy:current` does not exist or has a terminal status
- **THEN** check_and_resume() SHALL return None
- **AND** the leader SHALL proceed with normal operation

#### Scenario: Completed deployment found

- **WHEN** a node becomes leader
- **AND** `_sys:deploy:current` has status Completed
- **THEN** check_and_resume() SHALL return None (nothing to resume)

### Requirement: Resume runs in background task

The check_and_resume() call SHALL run in a spawned task so it does not block leader initialization or the first Raft heartbeat.

#### Scenario: Leader initialization not blocked

- **WHEN** a node transitions to leader state
- **THEN** the leader SHALL begin accepting Raft operations immediately
- **AND** the deployment resume task SHALL run concurrently

### Requirement: Idempotent resume under split-brain

If two nodes briefly both believe they are leader during an election, concurrent check_and_resume() calls SHALL not corrupt deployment state. The CAS writes in the coordinator SHALL prevent double-execution.

#### Scenario: Concurrent resume attempts

- **WHEN** two nodes call check_and_resume() concurrently for the same deployment
- **THEN** one SHALL succeed and the other SHALL fail with a CAS conflict
- **AND** the deployment SHALL complete exactly once
