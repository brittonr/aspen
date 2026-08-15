## Why

Molten uses content-addressed artifacts, blobs, storage values, snapshots, traces, transcripts, replay logs, and remote-sync payloads, but the architecture does not yet define chunk-level content addressing. Treating every large object as a single opaque blob wastes bandwidth and storage, weakens resumability, complicates range reads, and makes retention/GC coarser than necessary.

## What Changes

- Add a content-addressed chunk-store model for large canonical objects.
- Represent large objects as chunk manifests/Merkle roots over deterministic chunk refs.
- Support fixed-size deterministic chunking first, with content-defined chunking as an explicit future chunker version.
- Verify chunks as they stream and verify object manifests before admission/use.
- Deduplicate chunks across artifacts, blob payloads, typed storage values, snapshots, replay logs, transcripts, traces, docs, and job DAG data.
- Track pins, retention, GC eligibility, receipts, and redaction/encryption policy at both object and chunk granularity.
- Integrate chunk manifests with Iroh blobs, Redb indexes, remote artifact sync, typed storage, deterministic replay, evaluation cache, and catalog/MCP inspection.

## Impact

This turns Molten's content-addressed storage into an efficient and replay-friendly CA store. The first milestone can implement chunk manifests, fixed-size chunking, streaming verification, local Redb chunk indexes, and tests proving dedup and GC safety for pinned manifests.
