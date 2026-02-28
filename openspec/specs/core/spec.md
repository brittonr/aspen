# Core Specification

## Purpose

Foundational types, traits, and resource bounds for the Aspen distributed orchestration layer. All other subsystems depend on these contracts.

## Requirements

### Requirement: Distributed Key-Value Store

The system SHALL expose a `KeyValueStore` trait providing linearizable read, write, delete, and scan operations over a distributed key-value namespace.

#### Scenario: Write and read back

- GIVEN a running Aspen cluster with a leader
- WHEN a client writes key `"foo"` with value `"bar"`
- THEN a subsequent read of `"foo"` SHALL return `"bar"`

#### Scenario: Delete key

- GIVEN a key `"foo"` exists in the store
- WHEN a client deletes `"foo"`
- THEN a subsequent read of `"foo"` SHALL return `None`

#### Scenario: Prefix scan

- GIVEN keys `"app/a"`, `"app/b"`, `"other/c"` exist
- WHEN a client scans with prefix `"app/"`
- THEN the result SHALL contain `"app/a"` and `"app/b"` but not `"other/c"`
- AND the result SHALL contain at most `MAX_SCAN_RESULTS` (10,000) entries

### Requirement: Cluster Controller

The system SHALL expose a `ClusterController` trait for managing Raft cluster membership: initializing a cluster, adding learners, changing membership, and querying metrics.

#### Scenario: Initialize single-node cluster

- GIVEN a fresh Aspen node with no cluster state
- WHEN the node initializes a cluster with itself as the sole member
- THEN the node SHALL become leader
- AND the cluster SHALL accept read/write operations

#### Scenario: Add learner node

- GIVEN a running single-node cluster
- WHEN a second node is added as a learner
- THEN the learner SHALL replicate the leader's log
- AND the learner SHALL NOT participate in elections until promoted to voter

#### Scenario: Change membership

- GIVEN a cluster with nodes {1, 2, 3}
- WHEN membership is changed to {1, 2, 3, 4, 5}
- THEN the new configuration SHALL take effect after Raft consensus
- AND the cluster SHALL continue operating during the transition

### Requirement: Resource Bounds (Tiger Style)

The system SHALL enforce fixed upper limits on all operations to prevent resource exhaustion. No operation SHALL be unbounded.

#### Scenario: Batch size limit

- GIVEN a client submitting a batch write
- WHEN the batch exceeds `MAX_BATCH_SIZE` (1,000 entries)
- THEN the operation SHALL be rejected with an error

#### Scenario: Key and value size limits

- GIVEN a client writing a key-value pair
- WHEN the key exceeds 1 KB or the value exceeds 1 MB
- THEN the operation SHALL be rejected with an error

#### Scenario: Scan result limit

- GIVEN a prefix scan matching more than 10,000 keys
- WHEN the scan executes
- THEN at most `MAX_SCAN_RESULTS` (10,000) entries SHALL be returned

### Requirement: Hybrid Logical Clock

The system SHALL use a Hybrid Logical Clock (HLC) combining physical wall-clock time with a logical counter for causally ordering events across nodes.

#### Scenario: Monotonic timestamps

- GIVEN two consecutive events on the same node
- WHEN HLC timestamps are generated for each
- THEN the second timestamp SHALL be strictly greater than the first

#### Scenario: Causal ordering across nodes

- GIVEN node A sends a message with HLC timestamp T to node B
- WHEN node B receives the message and generates its next timestamp
- THEN node B's timestamp SHALL be greater than T

### Requirement: Error Handling

The system SHALL use `snafu` for structured error handling in library crates and `anyhow` for application-level errors. Production code SHALL NOT use `.unwrap()`, `.expect()`, `panic!()`, `todo!()`, or `unimplemented!()`.

#### Scenario: Actionable errors

- GIVEN an operation fails due to a network timeout
- WHEN the error propagates to the caller
- THEN the error SHALL include context about what operation failed and why
- AND the error SHALL be structured with a distinct variant (not a string)

### Requirement: Functional Core, Imperative Shell

The system SHALL separate pure business logic (in `src/verified/` modules) from I/O and async operations (in shell modules). Pure functions SHALL receive time, randomness, and configuration as explicit parameters.

#### Scenario: Verified function purity

- GIVEN a function in a `verified/` module
- WHEN it is invoked
- THEN it SHALL perform no I/O, no async operations, and no system calls
- AND it SHALL be deterministic for the same inputs
