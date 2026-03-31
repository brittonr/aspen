## Approach

Two complementary fixes: make the export BFS fail loudly on missing objects, and add a post-import integrity check that catches gaps before export is ever called.

## Design

### 1. Export BFS: propagate errors instead of silent skip

In `export_commit_dag_collect`, the `Ok(None)` remap case and the `Err(_)` remap case both `continue`, silently orphaning subtrees. Change to:

- Collect missing BLAKE3 hashes in a `Vec<blake3::Hash>` alongside the exported objects
- After the BFS completes, if any hashes are unresolved AND the caller didn't request partial results, return a new `BridgeError::IncompleteDag` variant with the list of missing hashes
- For the federation export path (`collect_dag_blake3`), the same pattern applies — collect unresolvable hashes, report to caller

This makes the error visible at the RPC layer so `handle_git_bridge_fetch` returns a proper error message instead of a truncated object set.

### 2. Post-import DAG integrity verification

After `federation_import_objects` completes and refs are updated, walk the DAG from each ref head using the mirror's BLAKE3 hashes. For each object:

- Read from KV (`forge:obj:{repo}:{blake3}`)
- Decode, extract child references
- Verify all children are reachable

If any objects are missing, log an error with the count and first few missing hashes. This catches import gaps immediately, before any client tries to clone.

Extract this into a reusable `verify_dag_integrity` function in the forge crate so it can be used by both federation import and future diagnostics.

### 3. Test coverage

- **gpgsig round-trip test**: Import a commit with gpgsig header, export it, verify SHA-1 matches
- **federation import→export completeness test**: Build a DAG with blobs, nested trees, merge commits, signed commits. Run through the federation import path (SyncObjects → `federation_import_objects`). Then call `export_commit_dag`. Assert exported object count == total objects in the DAG.

## Key Decisions

- **No partial-success mode for export_commit_dag**: The SHA-1 based `export_commit_dag` (used by `handle_git_bridge_fetch`) should fail cleanly rather than return partial results. Git can't use an incomplete packfile. The BLAKE3 based `collect_dag_blake3` (used by federation sync) already handles partial batches via `has_more`.
- **Integrity check is diagnostic, not blocking**: The post-import walk logs errors but doesn't fail the federation sync. The sync succeeded in transferring objects — the integrity issue needs investigation, not a retry.
- **`verify_dag_integrity` lives in `aspen-forge`**, not the handler crate, so it's unit-testable.

## Interfaces

```rust
// New error variant
BridgeError::IncompleteDag {
    missing: Vec<String>,  // hex-encoded BLAKE3 hashes
    exported: u32,         // objects successfully collected
}

// New function in aspen-forge
pub async fn verify_dag_integrity(
    kv: &dyn KeyValueStore,
    repo_id: &RepoId,
) -> DagIntegrityResult;

pub struct DagIntegrityResult {
    pub total_stored: u32,
    pub reachable: u32,
    pub missing: Vec<String>,
    pub ref_heads: Vec<(String, [u8; 32])>,
}
```
