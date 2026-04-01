## ADDED Requirements

### Requirement: RPC latency histogram per operation category

The system SHALL record a latency histogram for every RPC request processed by the `ClientProtocolHandler` dispatch loop. The histogram SHALL be labeled with the operation name (the `ClientRpcRequest` variant name) and the handler name (from `RequestHandler::name()`).

#### Scenario: Successful request records latency

- **WHEN** a `ClientRpcRequest::Read` is processed by `CoreHandler` and completes in 1.2ms
- **THEN** the metrics registry SHALL contain a histogram observation of 1.2ms under metric name `aspen.rpc.duration_ms` with labels `{operation="Read", handler="CoreHandler"}`

#### Scenario: Failed request records latency and error

- **WHEN** a `ClientRpcRequest::Write` fails with an error after 5ms
- **THEN** the metrics registry SHALL contain a histogram observation of 5ms under `aspen.rpc.duration_ms` with labels `{operation="Write", handler="CoreHandler"}`
- **AND** a counter `aspen.rpc.errors_total` SHALL be incremented with labels `{operation="Write", handler="CoreHandler"}`

#### Scenario: Unhandled request records no handler

- **WHEN** no registered handler returns `can_handle() == true` for a request
- **THEN** the metrics registry SHALL increment `aspen.rpc.errors_total` with labels `{operation="<variant>", handler="none"}`

### Requirement: RPC request throughput counter

The system SHALL maintain a monotonic counter of total RPC requests processed, labeled by operation category.

#### Scenario: Counter increments on each request

- **WHEN** 100 `Read` requests and 50 `Write` requests are processed
- **THEN** `aspen.rpc.requests_total{operation="Read"}` SHALL equal 100
- **AND** `aspen.rpc.requests_total{operation="Write"}` SHALL equal 50

### Requirement: Write batcher metrics

The system SHALL record write batcher operational metrics: batch size histogram, flush count, flush latency, forwarding count (writes forwarded to leader), and follower skip count.

#### Scenario: Batch flush records size and latency

- **WHEN** the write batcher flushes a batch of 15 operations taking 3.1ms
- **THEN** `aspen.write_batcher.batch_size` histogram SHALL record 15
- **AND** `aspen.write_batcher.flush_duration_ms` histogram SHALL record 3.1

#### Scenario: Follower write forwarding increments counter

- **WHEN** a follower node receives a write and forwards it to the leader
- **THEN** `aspen.write_batcher.forwarded_total` counter SHALL be incremented

#### Scenario: Follower batcher skip increments counter

- **WHEN** a follower node skips the batcher and goes directly to the forwarding path
- **THEN** `aspen.write_batcher.batcher_skipped_total` counter SHALL be incremented

### Requirement: Snapshot transfer metrics

The system SHALL record metrics for every Raft snapshot transfer: size in bytes, duration in milliseconds, source and target node IDs, and success/failure outcome.

#### Scenario: Successful snapshot send

- **WHEN** node 1 sends a 200MB snapshot to node 2 in 4500ms
- **THEN** `aspen.snapshot.transfer_size_bytes` histogram SHALL record 209715200 with labels `{direction="send", peer="2"}`
- **AND** `aspen.snapshot.transfer_duration_ms` histogram SHALL record 4500 with labels `{direction="send", peer="2"}`
- **AND** `aspen.snapshot.transfers_total{direction="send", outcome="success"}` SHALL be incremented

#### Scenario: Failed snapshot receive

- **WHEN** node 3 fails to receive a snapshot from node 1 (size exceeded)
- **THEN** `aspen.snapshot.transfers_total{direction="receive", outcome="error"}` SHALL be incremented
