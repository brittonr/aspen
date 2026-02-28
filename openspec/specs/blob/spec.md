# Blob & Docs Specification

## Purpose

Content-addressed blob storage via iroh-blobs (BLAKE3 hashed) and CRDT document replication via iroh-docs. Blobs are the primary mechanism for distributing large data (Git objects, CI artifacts, plugin binaries) across the cluster.

## Requirements

### Requirement: Content-Addressed Blob Storage

The system SHALL store binary data as content-addressed blobs using BLAKE3 hashing via iroh-blobs. Blobs SHALL be immutable and deduplicated by hash.

#### Scenario: Store and retrieve blob

- GIVEN binary data of 10 MB
- WHEN the data is stored as a blob
- THEN a BLAKE3 hash SHALL be returned
- AND retrieving by that hash SHALL return the identical data

#### Scenario: Deduplication

- GIVEN the same 10 MB data is stored twice
- WHEN the second store completes
- THEN only one copy SHALL exist on disk
- AND both store operations SHALL return the same hash

### Requirement: P2P Blob Transfer

The system SHALL transfer blobs between nodes using iroh-blobs' P2P protocol. Any node that has a blob SHALL be able to serve it to other nodes.

#### Scenario: Fetch blob from remote node

- GIVEN node A has blob with hash H, node B does not
- WHEN node B requests blob H
- THEN node B SHALL download it from node A over iroh QUIC
- AND the received data SHALL match hash H

#### Scenario: Multi-source download

- GIVEN nodes A, B, and C all have blob H
- WHEN node D requests blob H
- THEN node D MAY download chunks from multiple sources in parallel

### Requirement: CRDT Document Replication

The system SHALL support real-time document replication via iroh-docs, using CRDTs for conflict-free merging of concurrent edits.

#### Scenario: Create and sync document

- GIVEN a document created on node A
- WHEN node A writes key `"title"` with value `"Hello"`
- THEN the write SHALL replicate to all subscribed nodes
- AND all nodes SHALL eventually see the same value

#### Scenario: Concurrent edits

- GIVEN nodes A and B both edit the same document offline
- WHEN they reconnect
- THEN the CRDT SHALL merge both edits without conflict
- AND all nodes SHALL converge to the same state

### Requirement: Blob Garbage Collection

The system SHALL periodically garbage-collect blobs that are no longer referenced by any KV entry, document, or active job.

#### Scenario: Unreferenced blob cleanup

- GIVEN a blob H was stored as a CI artifact
- WHEN the artifact record is deleted and GC runs
- THEN blob H SHALL be removed from local storage if no other references exist
