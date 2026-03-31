## ADDED Requirements

### Requirement: QUIC stream priority for Raft traffic

The system SHALL set noq `SendStream::set_priority()` on all Raft RPC streams to ensure Raft protocol traffic is scheduled ahead of bulk data on the wire. Priority MUST be set after `open_bi()`/`accept_bi()` and before any bytes are written.

#### Scenario: Raft heartbeat during saturated bulk transfer

- **WHEN** multiple bulk streams (priority 0) are actively sending data
- **AND** a Raft AppendEntries heartbeat RPC opens a new stream
- **THEN** the heartbeat stream is set to `STREAM_PRIORITY_RAFT` (100)
- **AND** noq's scheduler transmits the heartbeat bytes ahead of pending bulk data

#### Scenario: Raft Vote uses high priority

- **WHEN** the Raft network client sends a Vote RPC
- **THEN** the send stream priority is set to `STREAM_PRIORITY_RAFT` before writing

#### Scenario: Raft AppendEntries uses high priority

- **WHEN** the Raft network client sends an AppendEntries RPC
- **THEN** the send stream priority is set to `STREAM_PRIORITY_RAFT` before writing

#### Scenario: Snapshot install uses bulk priority

- **WHEN** the Raft network client sends an InstallSnapshot RPC
- **THEN** the send stream priority is set to `STREAM_PRIORITY_BULK` (0)

### Requirement: Priority on response streams

The Raft protocol handler SHALL set the same priority on response `SendStream`s as the request type warrants. Vote and AppendEntries responses MUST use `STREAM_PRIORITY_RAFT`. Snapshot responses MUST use `STREAM_PRIORITY_BULK`.

#### Scenario: Heartbeat response prioritized

- **WHEN** the protocol handler receives an AppendEntries request
- **AND** prepares to write the response
- **THEN** the response send stream is set to `STREAM_PRIORITY_RAFT` before writing

#### Scenario: Snapshot response uses bulk priority

- **WHEN** the protocol handler receives an InstallSnapshot request
- **AND** prepares to write the response
- **THEN** the response send stream is set to `STREAM_PRIORITY_BULK` before writing

### Requirement: Priority constants

Stream priority levels SHALL be defined as compile-time constants in `aspen-constants`. Exactly two priority levels SHALL be used.

#### Scenario: Constants are valid

- **WHEN** the system compiles
- **THEN** `STREAM_PRIORITY_RAFT` is greater than `STREAM_PRIORITY_BULK`
- **AND** `STREAM_PRIORITY_RAFT` is a positive `i32`
- **AND** `STREAM_PRIORITY_BULK` equals 0 (the noq default)

### Requirement: Backward compatibility

Stream priority is sender-local and MUST NOT require any wire protocol changes. Nodes without priority support MUST interoperate with nodes that set priority.

#### Scenario: Mixed-version cluster

- **WHEN** an upgraded node sets stream priority on Raft RPCs
- **AND** the peer is an older node that does not set priority
- **THEN** both nodes communicate without error
- **AND** the upgraded node's outgoing Raft streams are still prioritized locally
