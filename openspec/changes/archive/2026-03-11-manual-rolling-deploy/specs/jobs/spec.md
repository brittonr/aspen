## MODIFIED Requirements

### Requirement: Worker Types

The system SHALL support multiple specialized worker types, each handling a specific category of jobs. Deploy jobs SHALL be handled by the deployment coordinator on the Raft leader, not by regular worker nodes.

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

#### Scenario: Deploy job routing

- GIVEN a deploy job is submitted to the job queue
- WHEN a worker polls for jobs
- THEN the deploy job SHALL NOT be dispatched to regular workers
- AND the deployment coordinator on the Raft leader SHALL handle it directly
