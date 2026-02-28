# Consensus Specification

## Purpose

Raft-based distributed consensus providing linearizable operations, leader election, log replication, and state machine management. Built on vendored openraft v0.10.0 with redb storage.

## Requirements

### Requirement: Raft Leader Election

The system SHALL elect a single leader among cluster members using the Raft protocol. Only the leader SHALL accept write operations.

#### Scenario: Leader election on startup

- GIVEN a cluster of 3 nodes with no current leader
- WHEN election timeout expires on one node
- THEN that node SHALL request votes from peers
- AND if it receives a majority, it SHALL become leader

#### Scenario: Leader failure triggers re-election

- GIVEN a 3-node cluster with node 1 as leader
- WHEN node 1 becomes unreachable
- THEN the remaining nodes SHALL elect a new leader within the election timeout
- AND the new leader SHALL serve all pending and new requests

#### Scenario: Forwarding writes to leader

- GIVEN a client connects to a follower node
- WHEN the client submits a write operation
- THEN the follower SHALL forward the request to the current leader

### Requirement: Log Replication

The system SHALL replicate Raft log entries from the leader to all followers. A write is committed once a majority of nodes have persisted it.

#### Scenario: Majority commit

- GIVEN a 5-node cluster with a leader
- WHEN the leader receives a write request
- THEN the write SHALL be committed after 3 nodes (majority) acknowledge
- AND the committed entry SHALL be applied to the state machine

#### Scenario: Follower catches up

- GIVEN a follower that was temporarily disconnected
- WHEN the follower reconnects
- THEN the leader SHALL replicate all missed entries to the follower
- AND the follower's state SHALL converge with the leader's

### Requirement: Unified Storage (redb)

The system SHALL store both the Raft log and the state machine in a single redb database, using single-fsync writes for durability with ~2-3ms write latency.

#### Scenario: Durable write

- GIVEN a write operation is committed by Raft
- WHEN the node crashes immediately after the fsync
- THEN on recovery, the committed write SHALL be present in the database

#### Scenario: Snapshot and compaction

- GIVEN the Raft log exceeds the compaction threshold
- WHEN a snapshot is triggered
- THEN the state machine snapshot SHALL be persisted to redb
- AND old log entries SHALL be compactable

### Requirement: Batched Writes

The system SHALL batch multiple concurrent write requests into a single Raft proposal to improve throughput under load.

#### Scenario: Batch coalescing

- GIVEN 50 concurrent write requests arrive within the batch window
- WHEN the batch is submitted to Raft
- THEN all 50 writes SHALL be proposed as a single log entry
- AND each caller SHALL receive their individual result

#### Scenario: Batch size limit

- GIVEN incoming writes that would exceed `MAX_BATCH_SIZE` (1,000)
- WHEN the batch limit is reached
- THEN the current batch SHALL be submitted immediately
- AND subsequent writes SHALL start a new batch

### Requirement: Key Expiration (TTL)

The system SHALL support time-to-live (TTL) on key-value entries. Expired keys SHALL be treated as deleted on read and cleaned up by background compaction.

#### Scenario: Key expires after TTL

- GIVEN a key written with a TTL of 5 seconds
- WHEN 5 seconds have elapsed
- THEN a read of that key SHALL return `None`

#### Scenario: TTL refresh on update

- GIVEN a key with TTL of 10 seconds, 7 seconds have elapsed
- WHEN the key is updated with a new TTL of 10 seconds
- THEN the expiration deadline SHALL reset to 10 seconds from now
