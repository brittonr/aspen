## Why

Federated git clone returns 0 objects because 2,902 out of 33,807 objects fail to import. Trees reference blobs that arrived in a different sync batch, and the current import pipeline makes a single topological-sort pass per call to `import_objects`. Objects whose dependencies were imported in a prior batch but whose mapping wasn't visible at sort time get dropped — including the HEAD commit, which means no SHA-1 mapping exists and `git clone` fetches nothing.

Six incremental fixes already landed (dedup via envelope_hash, QUIC reconnection, message size limit bump, origin SHA-1 plumbing, skipped-object KV fallback) and brought the mapping ratio from 2% to near-complete. This change fixes the remaining blocker: the import function needs to converge via repeated passes rather than giving up after one wave.

## What Changes

- **Convergent retry loop in `federation_import_objects`**: After all sync batches are collected, run import passes in a loop. Each pass attempts all objects that still lack a SHA-1→BLAKE3 mapping. Stop when a pass imports zero new objects (fixed-point). This replaces the current single retry pass.
- **Multi-wave `import_objects` error resilience**: Change wave processing in `GitImporter::import_objects` so that a single object failure within a wave doesn't abort the entire import. Collect per-object errors, continue with remaining objects, and report failures to the caller so the convergent loop can retry them.
- **Import progress tracking**: Return structured stats (imported / skipped / failed per pass) so the federation orchestrator can log convergence and detect stuck imports.

## Capabilities

### New Capabilities

- `convergent-import`: Fixed-point retry loop for federation object import that handles cross-batch dependencies

### Modified Capabilities

- `federation-git-import`: The federation git sync pipeline (`handle_federation_git_fetch`) gains the convergent retry after batch collection

## Impact

- **Files**: `crates/aspen-forge-handler/src/handler/handlers/federation_git.rs` (orchestrator), `crates/aspen-forge-handler/src/handler/handlers/federation.rs` (`federation_import_objects`), `crates/aspen-forge/src/git/bridge/importer.rs` (`import_objects` error handling)
- **APIs**: `ImportResult` gains a `failed` count or a `failures` vec; `federation_import_objects` return type may grow to include per-pass stats
- **Dependencies**: None new
- **Testing**: Regression test that imports objects where trees arrive before their blob dependencies, verifying convergence
