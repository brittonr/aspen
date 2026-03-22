# Blob DAG Sync — Delta Spec

## ADDED Requirements

### Requirement: DAG-Aware Replication Repair

The blob replication system SHALL use DAG traversal for repair scans when blobs form content-addressed graphs (e.g., Git object trees, snix directory hierarchies). Instead of scanning individual blob hashes, repair SHALL traverse from known roots and discover under-replicated subgraphs.

#### Scenario: Repair scan via traversal

- GIVEN a repository root with 500 reachable blobs, 3 of which are under-replicated
- WHEN the repair loop runs a DAG traversal from the root
- THEN it SHALL discover all 3 under-replicated blobs in a single pass
- AND it SHALL batch replication requests for them

#### Scenario: Traversal respects replication factor

- GIVEN a blob with replication factor 3 and only 1 replica
- WHEN the DAG traversal encounters this blob
- THEN it SHALL be marked for replication to 2 additional nodes
- AND the traversal SHALL continue to child nodes

### Requirement: BAO-Framed Streaming Transfer

Blob transfers within the DAG sync protocol SHALL use BAO4 encoding for incremental verification. Each chunk group (16 KiB) SHALL be independently verifiable against the BLAKE3 hash without requiring the full blob.

#### Scenario: Incremental verification during transfer

- GIVEN a 100 MB blob being transferred via DAG sync
- WHEN the receiver has received the first 1 MB (64 chunk groups)
- THEN all 64 chunk groups SHALL have been verified against the BLAKE3 hash
- AND if any chunk group fails verification, the transfer SHALL abort immediately

#### Scenario: Early rejection of corrupt data

- GIVEN a sender that sends corrupted data for a blob
- WHEN the receiver verifies the first chunk group
- THEN the receiver SHALL detect the mismatch within the first 16 KiB
- AND the receiver SHALL abort the stream and try the next peer

## MODIFIED Requirements

### Requirement: P2P Blob Transfer

Blob transfer SHALL support both individual hash-based downloads (existing behavior) and streaming DAG-ordered transfers (new). The DAG sync protocol SHALL coexist with the existing iroh-blobs download path — individual blob downloads remain available for ad-hoc fetches outside of DAG traversals.

#### Scenario: DAG sync for batched transfer

- GIVEN a set of 200 related blobs forming a tree
- WHEN node B requests the tree root via DAG sync
- THEN all 200 blobs SHALL be transferred over a single QUIC stream
- AND the transfer SHALL be more efficient than 200 individual downloads

#### Scenario: Individual download fallback

- GIVEN a single blob hash from an inline predicate's hash-only reference
- WHEN node B needs that blob
- THEN node B SHALL download it via the existing iroh-blobs protocol
- AND the blob SHALL be stored and verifiable as before
