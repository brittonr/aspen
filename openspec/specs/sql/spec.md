# SQL Specification

## Purpose

SQL query engine over the distributed KV store using Apache DataFusion. Enables querying Aspen's key-value data with standard SQL syntax. Optional, gated behind the `sql` feature flag.

## Requirements

### Requirement: SQL Query Execution

The system SHALL execute SQL queries against data stored in the Raft KV store using DataFusion. Queries SHALL read from a virtual table backed by KV scan operations.

#### Scenario: SELECT query

- GIVEN keys `"users/1" = {"name": "Alice"}` and `"users/2" = {"name": "Bob"}` in the KV store
- WHEN a SQL query `SELECT * FROM kv WHERE key LIKE 'users/%'` executes
- THEN the result SHALL contain both entries

#### Scenario: Aggregation query

- GIVEN 1,000 metric entries in the KV store
- WHEN a SQL query `SELECT COUNT(*), AVG(value) FROM metrics` executes
- THEN the result SHALL reflect the correct count and average

### Requirement: SQL as a Job

The system SHALL support submitting SQL queries as jobs via the job execution framework, allowing asynchronous query execution with result storage.

#### Scenario: Async SQL job

- GIVEN a SQL query submitted as a job
- WHEN the SQL worker processes it
- THEN the query results SHALL be stored as a blob
- AND the job result SHALL contain a reference to the blob
