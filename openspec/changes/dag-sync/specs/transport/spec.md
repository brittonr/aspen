# Transport DAG Sync — Delta Spec

## ADDED Requirements

### Requirement: DAG Sync ALPN

The system SHALL define a `DAG_SYNC_ALPN` protocol identifier for deterministic DAG synchronization. Incoming connections with this ALPN SHALL be routed to the DAG sync request handler.

#### Scenario: DAG sync connection routing

- GIVEN an incoming QUIC connection with ALPN `DAG_SYNC_ALPN`
- WHEN the connection is accepted
- THEN it SHALL be routed to the DAG sync protocol handler
- AND the handler SHALL read a `DagSyncRequest` from the bidirectional stream

#### Scenario: ALPN coexistence

- GIVEN the DAG sync ALPN is registered alongside RAFT_ALPN, CLIENT_ALPN, and GOSSIP_ALPN
- WHEN connections arrive on different ALPNs
- THEN each SHALL be routed to the correct handler independently

### Requirement: DAG Sync Wire Protocol

The DAG sync protocol SHALL use a request/response pattern over a single QUIC bidirectional stream. The request is a postcard-encoded `DagSyncRequest`. The response is a sequence of frames, each containing a discriminator byte, a BLAKE3 hash, and optionally BAO4-encoded blob data.

#### Scenario: Request encoding

- GIVEN a `DagSyncRequest` with root hash, traversal options, and inline predicate
- WHEN the request is serialized
- THEN it SHALL be postcard-encoded
- AND the encoded size SHALL be bounded by `MAX_DAG_SYNC_REQUEST_SIZE` (16 MiB)

#### Scenario: Response frame format

- GIVEN the sender has a blob to inline
- WHEN it writes a response frame
- THEN the frame SHALL contain: discriminator byte (0x01 for data, 0x00 for hash-only), BLAKE3 hash (32 bytes), and optionally the blob size as little-endian u64 followed by BAO4-encoded chunk data

#### Scenario: Stream completion

- GIVEN the sender has finished its traversal
- WHEN the last frame has been written
- THEN the sender SHALL call `finish()` on the QUIC send stream
- AND the receiver SHALL treat end-of-stream as traversal complete

### Requirement: DAG Sync Request Bounds

All DAG sync requests SHALL be bounded per Tiger Style. The request size, traversal depth, visited set size, and total transferred bytes SHALL have fixed limits.

#### Scenario: Request size limit

- GIVEN a `DagSyncRequest` with a visited set containing 100,000 hashes
- WHEN the request is serialized
- THEN it SHALL be rejected if the encoded size exceeds `MAX_DAG_SYNC_REQUEST_SIZE`

#### Scenario: Traversal depth limit

- GIVEN a DAG with depth 50,000
- WHEN the traversal reaches `MAX_DAG_TRAVERSAL_DEPTH` (10,000)
- THEN the traversal SHALL stop and the response SHALL indicate truncation

#### Scenario: Total transfer limit

- GIVEN a DAG sync request for a very large repository
- WHEN the total bytes transferred exceed `MAX_DAG_SYNC_TRANSFER_SIZE` (10 GiB)
- THEN the sender SHALL stop streaming and indicate the limit was reached
