## Context

When a git object is imported via `GitImporter::import_objects`, it's wrapped in a `SignedObject` using the importing node's secret key. The BLAKE3 hash of the `SignedObject` bytes becomes the storage key. On the origin cluster, objects get origin-BLAKE3 hashes. Commits and trees embed these origin-BLAKE3 hashes as references to their dependencies (parent commits, tree entries).

When federation sync transfers objects to a mirror cluster and re-imports them, each object gets a **different** BLAKE3 hash because the mirror's secret key produces a different signature. The embedded references inside commits and trees still point to origin-BLAKE3 hashes. The exporter's BFS (`export_commit_dag_collect`) reads an object, extracts dependency BLAKE3 hashes from its payload, then tries to read those hashes from the mirror's KV store — but they don't exist there, so the walk silently stops.

Currently the SHA-1→BLAKE3 mapping tracks origin SHA-1 to mirror BLAKE3, but there's no mapping from origin-BLAKE3 to mirror-BLAKE3. The exporter needs this to follow the DAG.

## Goals / Non-Goals

**Goals:**

- Federated `git clone` of the full Aspen repo (33K+ objects) succeeds end-to-end
- DAG exporter walks the complete object graph on the mirror
- No performance regression for non-federated git operations

**Non-Goals:**

- Changing the `SignedObject` format or signing behavior
- Streaming/chunked fetch responses (separate concern; 256MB limit is sufficient)
- Re-architecting the federation sync protocol

## Decisions

### 1. Origin-BLAKE3 → Mirror-BLAKE3 remap index in KV

Store a remap entry for every object imported via federation:

```
forge:remap:{mirror_repo_hex}:{origin_blake3_hex} → {mirror_blake3_hex}
```

Written during `federation_import_objects` alongside the existing SHA-1→BLAKE3 mapping. The origin BLAKE3 is available from the `SyncObject.envelope_hash` field (set by the Forge resolver during export).

**Rationale**: KV-based like existing mappings. Prefix-scoped to the mirror repo. Lookup is O(1) per object during BFS, adding ~1 KV read per DAG node.

### 2. Exporter resolves BLAKE3 references through remap

In `export_commit_dag_collect`, after extracting dependency BLAKE3 hashes from a `GitObject` payload, the exporter first tries the hash directly (works for non-federated repos). If the object isn't found in KV, it falls back to the remap index to translate origin-BLAKE3 → mirror-BLAKE3, then reads the mirror object.

This keeps non-federated codepaths unchanged (no remap lookup when direct read succeeds).

### 3. Remap index populated during convergent import

Both the initial per-batch import and the post-sync convergent retry write remap entries. For objects where `SyncObject.envelope_hash` is `None` (shouldn't happen for git objects exported by the Forge resolver, but defensive), skip the remap write — the object is still importable, just not remappable by origin-BLAKE3.

### 4. No remap for non-federation paths

The `GitImporter::import_objects` internal method doesn't write remap entries. Only `federation_import_objects` does, since it has access to the origin `envelope_hash`. This avoids overhead for regular git push operations.

## Risks / Trade-offs

- **Extra KV writes**: One remap entry per object (~33K writes for Aspen). These are batched with existing mapping writes during import, so overhead is marginal.
- **Extra KV reads during export**: One remap lookup per BFS node that isn't found directly. For non-federated repos this never triggers. For federated repos, every node needs a remap lookup since direct reads always fail — but these are fast KV reads (~0.1ms each).
- **Remap index staleness**: If an object is re-imported (e.g., re-sync), the remap entry is overwritten with the same value. No staleness risk.
- **Missing envelope_hash**: If a SyncObject lacks `envelope_hash`, the remap entry can't be written. The BFS will fail to resolve that object's dependencies. Mitigation: the Forge resolver always sets `envelope_hash` for git objects; add a warning log if it's missing.
