## Why

Federated git clone (`fed:` URL) returns an empty repo for large repositories (33K+ objects). The DAG walk exporter fills a batch up to the limit, but trees in the batch reference blobs that landed in a different batch (or haven't been fetched yet). The importer on the receiving side fails with "hash mapping not found" because it can't resolve those forward references. This blocks the federation dogfood pipeline's ability to clone repos across clusters.

## What Changes

- **Dependency-closed batches on the exporter**: Change `export_git_objects` / `collect_dag_blake3` so every batch is a closed set — every tree entry's target is either in the batch or in the receiver's `have_set`. When the limit would break a closure, finish the current closure and report `has_more`.
- **Multi-pass importer with retry**: On the import side, objects that fail due to missing dependencies are queued for a retry pass after the rest of the batch succeeds. This handles residual ordering issues within a single batch.
- **Integration test**: A deterministic test that creates a repo with enough objects to require multiple batches, runs a full federated export→import cycle, and verifies all objects resolve correctly.

## Capabilities

### New Capabilities

- `federation-dag-closure`: Ensures federation sync batches are dependency-closed so the importer can always resolve tree→blob and commit→tree references within a single batch plus the receiver's existing objects.

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/git/bridge/exporter.rs` — `collect_dag_blake3` and `export_commit_dag_blake3` gain closure enforcement
- `crates/aspen-forge/src/resolver.rs` — `export_git_objects` batch assembly respects closure
- `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` — `federation_import_objects` gets retry logic for unresolved deps
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — `federation_import_objects` same
- New integration test in `crates/aspen-forge/` or `tests/`
