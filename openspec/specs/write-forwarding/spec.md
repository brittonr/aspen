# write-forwarding Specification

## Purpose

Defines the Write Forwarding capability requirements preserved by Aspen's archived OpenSpec records.

## Requirements

### Requirement: Write forwarding rule 1

Write forwarding MUST forward writes to the indicated leader when `RaftNode::write()` receives `ForwardToLeader` and a `WriteForwarder` is set.

#### Scenario: Rule 1 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST forward writes to the indicated leader when `RaftNode::write()` receives `ForwardToLeader` and a `WriteForwarder` is set.

### Requirement: Write forwarding rule 2

Write forwarding MUST use existing iroh QUIC connections through the connection pool or direct connect path for forwarding.

#### Scenario: Rule 2 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST use existing iroh QUIC connections through the connection pool or direct connect path for forwarding.

### Requirement: Write forwarding rule 3

Write forwarding MUST return `NotLeader` without chain-forwarding when the forwarding target is also not the leader.

#### Scenario: Rule 3 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST return `NotLeader` without chain-forwarding when the forwarding target is also not the leader.

### Requirement: Write forwarding rule 4

Write forwarding MUST avoid holding local locks across the network forwarding call.

#### Scenario: Rule 4 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST avoid holding local locks across the network forwarding call.

### Requirement: Write forwarding rule 5

Write forwarding MUST bound the forwarding timeout to at most 30 seconds.

#### Scenario: Rule 5 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST bound the forwarding timeout to at most 30 seconds.

### Requirement: Write forwarding rule 6

Write forwarding MUST preserve existing `NotLeader` behavior when no `WriteForwarder` is set.

#### Scenario: Rule 6 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST preserve existing `NotLeader` behavior when no `WriteForwarder` is set.

### Requirement: Write forwarding rule 7

Write forwarding MUST bypass the follower write batcher for forwarded writes.

#### Scenario: Rule 7 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST bypass the follower write batcher for forwarded writes.

### Requirement: Write forwarding rule 8

Write forwarding MUST log worker stats KV write failures at DEBUG rather than WARN.

#### Scenario: Rule 8 is enforced

- **WHEN** a follower processes a write-forwarding path covered by this rule
- **THEN** write forwarding MUST log worker stats KV write failures at DEBUG rather than WARN.
