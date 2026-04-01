## MODIFIED Requirements

### Requirement: Stream acquisition sets priority

The `PeerConnection::acquire_stream` method SHALL accept a `StreamPriority` parameter. After `open_bi()` succeeds, the method SHALL call `send.set_priority()` with the appropriate `i32` value before returning the `StreamHandle`.

#### Scenario: Critical stream gets high priority

- **WHEN** `acquire_stream(StreamPriority::Critical)` is called
- **THEN** the returned `StreamHandle.send` has priority set to `STREAM_PRIORITY_RAFT`

#### Scenario: Bulk stream keeps default priority

- **WHEN** `acquire_stream(StreamPriority::Bulk)` is called
- **THEN** the returned `StreamHandle.send` has priority set to `STREAM_PRIORITY_BULK` (0)

#### Scenario: Backward compatibility

- **WHEN** a caller uses `StreamPriority::Critical`
- **AND** the remote node is an older version
- **THEN** the stream functions identically (priority is sender-local, not a wire protocol change)

### Requirement: Connection health unaffected by priority

The connection health state machine SHALL NOT change behavior based on stream priority. Health transitions remain based on connection-level errors, not per-priority tracking.

#### Scenario: Health unchanged by priority usage

- **WHEN** streams at different priorities are opened and closed
- **THEN** the `ConnectionHealth` state is determined by connection errors only, not by priority
