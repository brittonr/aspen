## Why

Molten owns the distributed runtime's storage path, currently served by the shipped Prolly and DoltLite delivery work. The F01–F14 findings concern source-level delivery, shutdown, scheduling, replication, and merge logic; they are not evidence of physical-storage faults or completed ChaosControl campaigns. A measured comparison against an alternative physical layer supports the storage-path decision without attributing those findings to storage.

The komora-io audit (2026-09-09) selected two Apache-2.0 components: `marble`, a garbage-collecting on-disk object store with atomic batch crash recovery, and `art`, an adaptive radix trie for fixed-length keys. BLAKE3 digests are fixed 32-byte keys, so `art` is the natural digest-to-ObjectId index over a marble heap. This change spikes both behind the storage seam and measures them against the current path. The current consumer is Molten's local object storage seam, the target outcome is a measured keep-or-replace decision, and the maintenance owner is this repository. The measured outcome retains Redb; the user-approved rejected-spike exemption closes the incorrect campaign-linkage premise without promoting the optional backend or inventing campaign evidence.

## What Changes

- Add a spike adapter that stores objects in marble keyed by allocated ObjectIds and indexes BLAKE3 digest to ObjectId with `art`.
- Keep BLAKE3 as content identity; marble ObjectIds are physical handles only and never become content identity.
- Measure the spike against the current Prolly-based path for write batch, point read, recovery, and space amplification under the campaign workloads.
- Record both pinned revisions in the dependency catalog as transport `crates.io`, plane `implementation`.

## Impact

The spike produces a measured comparison and a keep-or-replace decision record. It does not replace the storage path, weaken BLAKE3 identity, or extend runtime claims.

## Dependencies

The durable-authority-state marble pilot runs independently. Findings about the marble visibility contract are shared by reference.

## Non-goals

- Do not change dataspace, vat, networking, or replication semantics.
- Do not make marble or art the default storage path from this change.
- Do not claim content identity from ObjectIds or durability beyond recovered batches.
- Do not adopt GPL-licensed komora components; this spike uses only Apache-2.0 `marble` and `art`.
