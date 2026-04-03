## ADDED Requirements

### Requirement: Job dependency DAG execution test

The system SHALL include a VM test that submits a diamond-shaped dependency DAG (A → B, A → C, B+C → D), verifies jobs execute in topological order, and verifies that failure of an upstream job prevents downstream execution.

#### Scenario: Diamond DAG completes in order

- **WHEN** jobs A, B, C, D are submitted where B and C depend on A, and D depends on B and C
- **THEN** A completes before B and C start, B and C complete before D starts, and D completes successfully

#### Scenario: Upstream failure blocks downstream

- **WHEN** job B is configured to fail, and D depends on B and C
- **THEN** D SHALL NOT execute, and the dependency tracker reports D as blocked by B's failure

### Requirement: Distributed worker pool routing test

The system SHALL include a multi-node VM test that starts a 2-node cluster, submits jobs, and verifies jobs are routed to and executed by workers on both nodes.

#### Scenario: Jobs distribute across nodes

- **WHEN** 10 jobs are submitted to a 2-node cluster with workers on both nodes
- **THEN** both nodes execute at least 1 job, and all 10 jobs complete successfully

#### Scenario: Worker on failed node triggers re-routing

- **WHEN** a job is assigned to node 2 and node 2's worker is stopped
- **THEN** the job is re-routed to node 1 and completes successfully

### Requirement: Cron and delayed scheduling test

The system SHALL include a VM test that registers a cron-scheduled job and a delayed job, advances time, and verifies both fire at the correct times.

#### Scenario: Delayed job fires after duration

- **WHEN** a job is scheduled with a 5-second delay and time is advanced by 5 seconds
- **THEN** the job transitions from Scheduled to Pending within 1 second of the target time

#### Scenario: Cron job fires on schedule

- **WHEN** a job is registered with a cron expression `*/10 * * * * *` (every 10 seconds) and time advances 25 seconds
- **THEN** the scheduler creates at least 2 job instances

### Requirement: Dead letter queue test

The system SHALL include a VM test that submits a job configured to always fail, exhausts its retry budget, and verifies it lands in the DLQ with correct metadata.

#### Scenario: Failed job moves to DLQ after retries exhausted

- **WHEN** a job is submitted with max_retries=2 and the worker always returns failure
- **THEN** the job is retried exactly 2 times, then moved to the DLQ with failure reason and attempt count

#### Scenario: DLQ job can be inspected and retried

- **WHEN** a job is in the DLQ and a retry is requested
- **THEN** the job moves back to Pending status and is eligible for execution

### Requirement: Standalone saga executor test

The system SHALL include a VM test that builds a multi-step saga, triggers a failure partway through, and verifies compensation runs in LIFO order with correct event recording.

#### Scenario: Saga compensates on mid-step failure

- **WHEN** a 4-step saga is executed and step 3 fails
- **THEN** compensation runs for steps 2, 1 (LIFO order), step 3's compensation does NOT run, and all compensation events are recorded in the event store

#### Scenario: Saga completes without compensation on success

- **WHEN** a 4-step saga is executed and all steps succeed
- **THEN** no compensation functions are called, and the saga state is Completed

### Requirement: Workflow engine multi-step test

The system SHALL include a VM test that defines a workflow with conditional transitions, executes it, and verifies the correct path is taken based on step outputs.

#### Scenario: Workflow follows success path

- **WHEN** a workflow has steps A → B (on success) or A → C (on failure), and step A succeeds
- **THEN** step B executes and step C does not

#### Scenario: Workflow follows failure path

- **WHEN** step A fails
- **THEN** step C executes and step B does not

### Requirement: Job affinity routing test

The system SHALL include a VM test that submits jobs with affinity constraints and verifies they are routed to workers that match the constraints.

#### Scenario: Affinity-tagged job routes to matching worker

- **WHEN** worker W1 has tag `gpu=true` and worker W2 has tag `gpu=false`, and a job requires `gpu=true`
- **THEN** the job executes on W1

#### Scenario: No matching worker leaves job pending

- **WHEN** a job requires `region=eu` and no workers have that tag
- **THEN** the job remains in Pending state until a matching worker registers

### Requirement: Job replay deterministic execution test

The system SHALL include a VM test that records a job execution, replays it, and verifies the replay produces identical output.

#### Scenario: Replay matches original execution

- **WHEN** a job is executed with recording enabled, producing output O and event trace T
- **THEN** replaying the recorded execution produces the same output O and event trace T

#### Scenario: Replay detects divergence

- **WHEN** a replay is run with a modified worker that produces different output
- **THEN** the replay system reports a divergence with the original recorded output
