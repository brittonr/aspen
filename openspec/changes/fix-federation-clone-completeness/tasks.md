## Tasks

### Task 1: Add IncompleteDag error variant to BridgeError

- [ ] Add `IncompleteDag { missing: Vec<String>, exported: u32 }` to `BridgeError` in `crates/aspen-forge/src/git/bridge/error.rs`
- Spec: FCLONE-1

### Task 2: Make export_commit_dag_collect propagate missing-object errors

- [ ] In `export_commit_dag_collect`, collect unresolvable BLAKE3 hashes instead of `continue`
- [ ] After BFS, return `BridgeError::IncompleteDag` if any hashes are unresolved
- [ ] Update `handle_git_bridge_fetch` to surface the error message in the response
- Spec: FCLONE-1

### Task 3: Add verify_dag_integrity function

- [ ] Create `crates/aspen-forge/src/git/bridge/integrity.rs` with `verify_dag_integrity`
- [ ] Walk from ref heads, read each object from KV, extract child refs, check reachability
- [ ] Return `DagIntegrityResult` with stored/reachable/missing counts
- Spec: FCLONE-2

### Task 4: Call verify_dag_integrity after federation import

- [ ] In `federation_git.rs:handle_federation_git_sync`, after the existing diagnostic BFS, call `verify_dag_integrity`
- [ ] Log warn if missing > 0, info if all reachable
- Spec: FCLONE-2

### Task 5: Add gpgsig commit round-trip test

- [ ] Test in `crates/aspen-forge/src/git/bridge/tests.rs`: import a commit with gpgsig multi-line header, export it, verify SHA-1 matches
- [ ] Test with mergetag and encoding headers too
- Spec: FCLONE-3

### Task 6: Add federation import→export completeness test

- [ ] Build a DAG: 5 blobs, 3 nested trees, 2 commits (one merge), 1 signed commit with gpgsig
- [ ] Convert to SyncObjects with envelope_hash and origin_sha1
- [ ] Import via convergent loop (federation_import_objects or equivalent)
- [ ] Call export_commit_dag from HEAD
- [ ] Assert exported count == total objects in DAG
- Spec: FCLONE-4
