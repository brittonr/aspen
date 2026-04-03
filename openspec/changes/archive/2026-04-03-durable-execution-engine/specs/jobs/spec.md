## MODIFIED Requirements

### Requirement: Job Submission and Scheduling

The system SHALL accept job submissions via the client API, store them in the Raft KV store, and schedule them to available workers based on job type, worker capabilities, and worker pressure metrics. Workers under CPU, memory, I/O pressure, with insufficient disk space, or that have not yet achieved Raft log catchup SHALL be excluded from scheduling. Jobs that are part of a durable workflow SHALL have their completion recorded as `ActivityCompleted` events in the workflow's event store.

#### Scenario: Submit and execute a job

- GIVEN a running cluster with a shell worker registered
- WHEN a client submits a shell job with command `echo hello`
- THEN the job SHALL be scheduled to a shell worker
- AND the worker SHALL execute the command and return the output

#### Scenario: Job queuing

- GIVEN all workers are busy
- WHEN a new job is submitted
- THEN the job SHALL be queued
- AND it SHALL be dispatched when a worker becomes available

#### Scenario: Worker excluded by pressure

- GIVEN a worker with available job slots but `cpu_pressure_avg10` exceeding the configured threshold
- WHEN a job is submitted
- THEN the scheduler SHALL NOT dispatch to that worker
- AND the job SHALL be queued until a worker without pressure is available

#### Scenario: Worker excluded by disk space

- GIVEN a worker with available job slots but `disk_free_store_pct` below the configured threshold
- WHEN a job is submitted
- THEN the scheduler SHALL NOT dispatch to that worker

#### Scenario: Worker excluded by readiness

- GIVEN a worker that has registered but has `is_ready = false` (Raft log not caught up)
- WHEN a job is submitted
- THEN the scheduler SHALL NOT dispatch to that worker
- AND the job SHALL be queued until a ready worker is available

#### Scenario: Durable workflow job completion records event

- GIVEN a job that was submitted as part of a durable workflow execution
- WHEN the job completes successfully
- THEN the `DurableWorkflowExecutor` SHALL record an `ActivityCompleted` event with the job's result
- AND the workflow SHALL advance to the next step
