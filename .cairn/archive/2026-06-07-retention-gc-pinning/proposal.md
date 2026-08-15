## Why

Molten will accumulate artifacts, blobs, receipts, snapshots, traces, storage records, cached evaluations, remote-sync payloads, and upgrade metadata. Deleting any of these prematurely can break replay, durable storage, evidence validation, active sessions, or upgrades. Keeping everything forever is also untenable.

## What Changes

- Define retention policy, pin sources, garbage-collection eligibility, and deletion receipts.
- Track pins from active actors, protocol sessions, job DAGs, typed storage refs, receipts, snapshots, transcripts, docs, policies, upgrade sessions, artifact names/channels, and operator holds.
- Require GC to prove no live or retained reference requires the object before deletion.
- Support retention classes for ephemeral traces, audit receipts, durable storage, private secrets, cache entries, and public artifacts.
- Emit receipts for pin, unpin, retention decision, GC eligibility, deletion, compaction, and denial.

## Impact

This makes deletion safe and auditable. The first milestone can track pins for artifact registry entries, blob refs, receipts, and active sessions, then deny deletion when pins remain.
