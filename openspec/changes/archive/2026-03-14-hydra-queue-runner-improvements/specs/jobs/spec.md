## MODIFIED Requirements

### Requirement: Job Submission and Scheduling

The system SHALL accept job submissions via the client API, store them in the Raft KV store, and schedule them to available workers based on job type, worker capabilities, and worker pressure metrics. Workers under CPU, memory, I/O pressure, or with insufficient disk space SHALL be excluded from scheduling.

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
