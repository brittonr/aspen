## Why

Molten already uses Preserves as the canonical envelope, evidence, replay, and storage boundary. rkyv is attractive for fast local, read-mostly Rust access, but it would weaken reviewability and interop if it became the canonical artifact format.

This change records the intended split before any implementation: Preserves remains the source of truth and rkyv may only appear as a rebuildable, validated, derived cache/archive sidecar.

## What Changes

- Define a derived zero-copy archive boundary for rkyv-backed local caches.
- Require archive use to bind canonical Preserves source refs, BLAKE3 source digests, format/schema versions, and validation status.
- Forbid derived archive bytes from becoming canonical evidence, storage, policy, or release identities.
- Add positive and negative test expectations for cache admission, rebuild, tamper, stale-source, and overclaim cases.

## Impact

This keeps Molten's public and evidence-facing data model stable while leaving room for local performance work. rkyv adoption becomes an optimization behind explicit gates, not a competing serialization contract.