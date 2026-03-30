## 1. Exporter: Dependency-closed batches

- [x] 1.1 Add `ensure_closure` function to `exporter.rs` that takes a collected object set + `known_blake3` and removes objects whose dependencies are unresolvable, returning the closed subset
- [x] 1.2 Modify `collect_dag_blake3` to track dependency edges (each object → set of BLAKE3 hashes it references) alongside the collected objects
- [x] 1.3 Call `ensure_closure` in `export_commit_dag_blake3` after collection, before converting to `FederationExportedObject`
- [x] 1.4 Unit test: repo with objects exceeding the limit — verify returned batch is closed and `has_more = true`
- [x] 1.5 Unit test: repo fitting in one batch — verify all objects returned, `has_more = false`

## 2. Resolver: Propagate closure to `export_git_objects`

- [x] 2.1 Verify `export_git_objects` in `resolver.rs` passes the `have_set` through to `export_dag_blake3` correctly (it already does — confirm after exporter changes)
- [x] 2.2 Test via `ForgeResourceResolver::sync_objects` with a limit that forces multi-batch: verify each batch is importable independently

## 3. Importer: Retry pass for unresolved dependencies

- [x] 3.1 In `federation_import_objects` (federation.rs and federation_git.rs), after the two-pass import (blobs then trees+commits), collect objects that failed with mapping errors
- [x] 3.2 Run a single retry pass on failed objects, add successes to the stats
- [x] 3.3 Unit test: batch with intentionally swapped import order — verify retry resolves it

## 4. Integration test

- [x] 4.1 Add a test in `crates/aspen-forge/src/git/bridge/` that creates a repo with 100+ objects (enough to force multi-batch at a low limit), exports via the resolver with limit=50, imports each batch on a separate KV store, and verifies all objects round-trip
- [x] 4.2 Run `cargo nextest run` to confirm no regressions

## 5. Dogfood validation

- [x] 5.1 Fix c2e dedup causing exporter to return empty batches — replaced unreliable c2e index with authoritative SHA-1 hash mapping store; fixed BFS to traverse known objects' deps instead of skipping entire subtrees
- [ ] 5.2 Run `nix run .#dogfood-federation -- full` and verify federated git clone succeeds for the full 33K-object repo (requires clean nix rebuild)
