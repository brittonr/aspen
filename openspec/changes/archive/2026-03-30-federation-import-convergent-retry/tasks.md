## 1. Partial-success import in GitImporter

- [x] 1.1 Add `failures: Vec<(Sha1Hash, String)>` field to `ImportResult` in `crates/aspen-forge/src/git/bridge/importer.rs`
- [x] 1.2 Change wave processing loop: replace `result?` with match that collects errors per-object and continues. Failed objects must not produce mappings or c2e index entries.
- [x] 1.3 When an object fails in wave N, remove it from the dependency graph for subsequent waves — objects in wave N+1 that depended on the failed object should themselves fail (not panic or hang).
- [x] 1.4 Add unit test: import a set of objects where one tree references a non-existent blob. Verify the tree appears in `failures`, sibling blobs and independent trees appear in `mappings`, and `objects_imported` counts only successes.

## 2. Convergent retry loop in federation_import_objects

- [x] 2.1 Replace the current blob→non-blob→retry three-pass structure in `federation_import_objects` (`crates/aspen-forge-handler/src/handler/handlers/federation.rs`) with a convergent loop:
  - Each pass: filter objects to those without SHA-1 mappings (`has_sha1` check), call `import_objects`, collect results.
  - Terminate when a pass imports zero new objects or iteration cap (10) is reached.
- [x] 2.2 Add per-pass logging: pass number, objects attempted, objects imported, objects failed, cumulative progress.
- [x] 2.3 On convergence-bound exit (10 passes, objects still unmapped), log at `warn` level with count and first 20 SHA-1 hashes of stuck objects.

## 3. Post-sync retry in handle_federation_git_fetch

- [x] 3.1 Replace the single retry pass in `handle_federation_git_fetch` (`crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`, around line 353) with a call to the updated `federation_import_objects` which now converges internally.
- [x] 3.2 Remove the separate `all_git_objects` accumulation if `federation_import_objects` now accepts the full object set and handles convergence. Alternatively, keep accumulation and pass the full set once.
- [x] 3.3 Verify that `translate_ref_hashes` uses the combined `sha1_to_local_blake3` from the converged import stats (HEAD commit must have a mapping).

## 4. Regression tests

- [x] 4.1 Integration test: simulate cross-batch dependencies — construct SyncObjects where tree T depends on blob B, import them as separate batches, verify both are mapped after convergent retry.
- [x] 4.2 Integration test: simulate deep chain (blob → subtree → tree → commit) split across 4 batches, verify convergence in ≤4 passes.
- [x] 4.3 Integration test: all objects in one batch — verify the loop completes in one pass with zero retries (no performance regression).
- [x] 4.4 Test: objects with genuinely missing dependencies — verify the loop terminates at the bound (10) and returns the stuck objects in the error log.

## 5. Cleanup

- [x] 5.1 Remove the pass2_failed / Pass 3 belt-and-suspenders retry in `federation_import_objects` (superseded by the convergent loop).
- [x] 5.2 Update doc comments on `federation_import_objects` and `import_objects` to describe the new semantics (partial success, convergent retry).
- [x] 5.3 Run `cargo nextest run -E 'test(/federation/)' -E 'test(/import/)' -E 'test(/bridge/)'` — all existing tests must pass.
