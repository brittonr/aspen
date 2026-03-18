# Forge DAG Sync — Delta Spec

## ADDED Requirements

### Requirement: Streaming DAG Sync Protocol

The Forge SHALL sync Git object graphs using a single-stream deterministic traversal protocol over iroh QUIC. The receiver sends a `DagSyncRequest` specifying a root hash, traversal method, and known heads. The sender executes the traversal and streams objects back in traversal order. The receiver executes the same traversal and inserts objects as they arrive.

#### Scenario: Full repository clone

- GIVEN node A has a repository with root commit C and 2,000 reachable objects
- GIVEN node B has no objects for this repository
- WHEN node B sends a `DagSyncRequest` with root C, traversal `DepthFirstPreOrder`, and empty known heads
- THEN node A SHALL stream all 2,000 objects over a single QUIC stream in traversal order
- AND node B SHALL insert each object as it arrives, advancing its local traversal
- AND the total number of QUIC round trips SHALL be 1 (request + streamed response)

#### Scenario: Incremental sync with known heads

- GIVEN node B already has history up to commit P (parent of the current head)
- WHEN node B sends a `DagSyncRequest` with root C and known heads `{P}`
- THEN node A's traversal SHALL terminate when it encounters P
- AND only objects between C and P SHALL be transferred

#### Scenario: Traversal order consistency

- GIVEN the same root hash C, traversal method, and known heads
- WHEN both sender and receiver execute the traversal independently
- THEN both SHALL produce the identical sequence of hashes
- AND any mismatch SHALL cause the receiver to abort with an error

### Requirement: Stem/Leaf Split Sync

The Forge SHALL support two-phase sync where structural nodes (commits, trees) are synced first, followed by leaf nodes (blobs) which MAY be downloaded from multiple peers in parallel.

#### Scenario: Structure-first sync

- GIVEN a repository with 500 commits, 3,000 trees, and 10,000 blobs
- WHEN node B syncs from node A with filter `NoBlob`
- THEN only commits and trees SHALL be transferred (structural stem)
- AND the transfer size SHALL be a fraction of the full repository

#### Scenario: Parallel leaf download

- GIVEN node B has completed stem sync and knows all blob hashes
- GIVEN nodes A, C, D each have subsets of the blobs
- WHEN node B initiates leaf sync with filter `JustBlob`
- THEN node B MAY shard blob downloads across nodes A, C, and D
- AND each node SHALL serve only the blobs requested from it

### Requirement: Inline Predicate

The Forge SHALL support sender-side inline predicates that determine whether object data is sent inline in the sync stream or as a hash-only reference for deferred retrieval.

#### Scenario: Size-based inlining

- GIVEN an inline predicate of `SizeThreshold(16384)` (16 KiB)
- WHEN the sender encounters a 500-byte tree object
- THEN the object data SHALL be inlined in the sync stream
- AND when the sender encounters a 5 MB blob
- THEN only the BLAKE3 hash SHALL be sent as a reference

#### Scenario: Metadata-only inlining

- GIVEN an inline predicate of `MetadataOnly`
- WHEN the sender traverses a commit object
- THEN the commit data SHALL be inlined
- AND when the sender encounters a blob object
- THEN only the hash reference SHALL be sent

## MODIFIED Requirements

### Requirement: Git Object Sync

The Forge's object synchronization SHALL use the DAG sync protocol instead of per-object fetch loops. The `SyncService` SHALL construct `DagSyncRequest` messages with appropriate traversal options and known heads derived from local branch tips.

#### Scenario: Push with DAG sync

- GIVEN a user pushes 50 new commits to Forge
- WHEN the push handler receives the new refs
- THEN the receiving node SHALL use DAG sync to fetch the new object graph
- AND the fetch SHALL complete in a single streaming exchange per peer

### Requirement: COB Change Sync

The Forge's COB change synchronization SHALL use the DAG sync protocol. COB change DAGs SHALL be traversable using the same `DagTraversal` trait as Git objects, following parent references.

#### Scenario: Sync COB changes via DAG traversal

- GIVEN node A has 20 new COB changes forming a DAG
- WHEN node B syncs COB changes for a repository
- THEN node B SHALL send a `DagSyncRequest` with the COB head hashes
- AND the exchange SHALL use a single QUIC stream per COB namespace
