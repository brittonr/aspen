## Context

The federated git clone shipped with three known loose ends from the debugging process.

## Goals / Non-Goals

**Goals:**

- Incremental federation sync skips already-transferred objects
- Multi-branch repos (main + dev) translate refs correctly
- Production-clean logging

**Non-Goals:**

- Federated git push (separate feature)
- Background mirror auto-refresh (separate feature)

## Decisions

### D1: Align hash types for incremental dedup

The `SyncObject.hash` is now `blake3::hash(content)`. The client sends these back as `have_hashes`. The resolver passes them to `export_dag_blake3` as `known_blake3`. But the DAG walk compares `SignedObject::hash()` (envelope BLAKE3) against `known_blake3`.

Fix: in `export_git_objects`, build a parallel set that maps content hashes to envelope hashes. Pass envelope hashes as `known_blake3` to the DAG walk. Keep content hashes in `SyncObject.hash` for wire verification.

### D2: Match refs to commits via SHA1

Each git commit has a deterministic SHA1 (from the raw content). The `HashMappingStore` stores SHA1↔BLAKE3 mappings. After importing all objects, look up each ref's commit by: (1) compute SHA1 from each commit SyncObject's raw content, (2) look up the local BLAKE3 via the hash mapping store. This gives a 1:1 mapping from each commit to its local BLAKE3, regardless of how many refs there are.
