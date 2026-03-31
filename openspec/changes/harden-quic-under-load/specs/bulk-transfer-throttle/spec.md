## ADDED Requirements

### Requirement: Bulk streams use default priority

All non-Raft streams (git bridge writes, blob transfer, federation sync, client RPC) SHALL use the default stream priority (`STREAM_PRIORITY_BULK` = 0). This ensures they yield to Raft traffic under congestion without requiring explicit rate limiting.

#### Scenario: Git bridge push uses bulk priority

- **WHEN** the git bridge opens streams to push objects
- **THEN** those streams use `STREAM_PRIORITY_BULK` (0)

#### Scenario: Blob transfer uses bulk priority

- **WHEN** blob replication opens streams between nodes
- **THEN** those streams use `STREAM_PRIORITY_BULK` (0)

### Requirement: Fair queuing among bulk streams

The QUIC endpoint SHALL have `send_fairness` enabled so that multiple bulk streams at the same priority level are scheduled round-robin rather than FIFO. This prevents a single large transfer from starving other bulk streams.

#### Scenario: Multiple concurrent git object writes

- **WHEN** two git bridge streams are both writing large objects at priority 0
- **THEN** noq schedules their data round-robin
- **AND** neither stream starves the other
