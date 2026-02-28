# Jobs Specification

## Purpose

General-purpose distributed job execution framework. Jobs are submitted to the cluster, scheduled to workers based on capabilities, executed with isolation, and their results stored durably. Specialized workers handle shell commands, SQL queries, blob operations, maintenance, and replication tasks.

## Requirements

### Requirement: Job Submission and Scheduling

The system SHALL accept job submissions via the client API, store them in the Raft KV store, and schedule them to available workers based on job type and worker capabilities.

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

### Requirement: Worker Types

The system SHALL support multiple specialized worker types, each handling a specific category of jobs.

#### Scenario: Shell worker

- GIVEN a shell worker is registered
- WHEN a shell job is submitted
- THEN the shell worker SHALL execute the command in a subprocess

#### Scenario: SQL worker

- GIVEN a SQL worker is registered with DataFusion enabled
- WHEN a SQL job is submitted
- THEN the SQL worker SHALL execute the query against the KV-backed DataFusion engine

#### Scenario: Blob worker

- GIVEN a blob worker is registered
- WHEN a blob transfer job is submitted
- THEN the blob worker SHALL handle iroh-blob operations (fetch, replicate, garbage collect)

#### Scenario: Maintenance worker

- GIVEN a maintenance worker is registered
- WHEN a maintenance job is scheduled (e.g., compaction, cleanup)
- THEN the maintenance worker SHALL perform the operation

#### Scenario: Replication worker

- GIVEN a replication worker is registered
- WHEN a replication job is submitted
- THEN the replication worker SHALL synchronize data between nodes or clusters

### Requirement: Job Status and Results

The system SHALL track job status transitions (pending → running → completed/failed) and store results durably in the KV store.

#### Scenario: Status tracking

- GIVEN a submitted job
- WHEN a client queries its status
- THEN the current status SHALL be returned (pending, running, completed, or failed)

#### Scenario: Result retrieval

- GIVEN a completed job
- WHEN a client retrieves the result
- THEN the output, exit code, and execution metadata SHALL be returned

#### Scenario: Failed job

- GIVEN a job that exits with a non-zero code
- WHEN the failure is recorded
- THEN the job status SHALL be `failed`
- AND the error output SHALL be preserved
