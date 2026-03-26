## 1. Diagnose

- [x] 1.1 Add diagnostic logging in `GitExporter::export_object` — before `get_bytes`, call `has()` and log both the BLAKE3 hash and `has()` result when `get_bytes` fails
- [x] 1.2 Add a unit test that imports a git object via `GitImporter` then immediately exports it via `GitExporter` using `InMemoryBlobStore` — verify round-trip succeeds
- [x] 1.3 Confirmed issue is IrohBlobStore/FsStore bao corruption (not hash mismatch, not GC)

## 2. KV Object Store

- [x] 2.1 Add KV write in `GitImporter::import_object_store_blob` — after `blobs.add_bytes`, store serialized bytes at `forge:obj:{repo_id}:{blake3_hex}`
- [x] 2.2 Add KV read in `GitExporter::export_object` — try `forge:obj:{repo}:{b3}` first, fall back to iroh-blobs `get_bytes`
- [x] 2.3 Add KV read in `GitExporter::collect_dag_blake3` — same KV-first pattern for the DAG walk
- [x] 2.4 Add KV read in `GitExporter::export_commit_dag_collect` — same KV-first pattern
- [x] 2.5 Update the round-trip unit test to verify the KV path works

## 3. VM Test

- [x] 3.1 Tighten `federation-git-clone.nix` — assert `README.md` content matches
- [ ] 3.2 Run VM test and verify federated git clone produces working tree with files
