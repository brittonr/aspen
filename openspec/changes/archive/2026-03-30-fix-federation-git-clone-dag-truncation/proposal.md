## Why

Federated git clone (`git clone aspen://.../fed:...`) fails for the Aspen workspace (33,897 objects). The federation sync transfers all objects to the mirror, but the git bridge fetch returns only 7,564 of them — the Forge DAG is structurally truncated. Git receives an incomplete object set and fails with `Could not read 387a1814...; Failed to traverse parents of commit 7e42deb6...`. This blocks the federated dogfood pipeline (`nix run .#dogfood-federation -- full`).

Three separate issues compound:

1. **Stream-per-RPC exhausts QUIC streams**: Each `sync_remote_objects` round opens a new bidirectional stream via `connection.open_bi()`, and the server counts total (not concurrent) streams. After 16 rounds the connection is killed. The correct iroh pattern is to keep a single stream alive for the whole sync conversation — write request, read response, repeat — using the existing length-prefixed wire framing.

2. **Origin SHA-1 mappings stored too late**: Trees exported from the origin reference sub-objects by their original SHA-1. On the mirror, sub-objects are imported with re-serialized SHA-1 (which may differ for trees/commits). The origin SHA-1 → BLAKE3 mappings are stored only in Phase 4 (after convergence), so the convergent loop can't resolve cross-batch references.

3. **Forge DAG truncated after import**: Even with all 33,897 objects imported (`missed=0`), the DAG walk from HEAD finds only 7,564 objects. Trees imported in early rounds have entry BLAKE3 references set using the mapping state *at import time*. When a tree's entry referenced a sub-object that was in a later batch, the tree's import failed and was deferred to the retry pass. But trees that *did* import in early rounds may reference sub-objects by a BLAKE3 that was computed from incomplete mapping state — the sub-object's envelope hash changed when it was re-imported later with different dependencies resolved. The DAG ends up with dangling BLAKE3 references that don't raise errors (the objects exist, just under different hashes).

## What Changes

- **Reuse a single QUIC stream for multi-round sync**: Replace the open-stream-per-RPC pattern with a persistent bidirectional stream. The client writes sequential requests on the same `SendStream`; the server reads in a loop on the same `RecvStream`. No stream exhaustion, no reconnection needed.
- **Store origin SHA-1 mappings after each convergent pass**: So trees can resolve sub-object references by original SHA-1 during convergence, not only after it.
- **Post-import DAG re-resolution**: After the final retry pass imports all objects, re-resolve BLAKE3 references in trees that were imported in earlier rounds with stale mapping state. Walk the mirror's ref heads, read each tree, check that every entry's BLAKE3 points to an object that actually exists, and fix any stale references using the now-complete mapping store.

## Capabilities

### New Capabilities

- `federation-dag-integrity`: Ensures the Forge DAG in a federation mirror is structurally complete — every BLAKE3 reference in a tree or commit points to an object that exists in the mirror under that exact BLAKE3 hash. Covers single-stream sync, import-side reference resolution, post-import DAG validation, and re-resolution of stale references.

### Modified Capabilities

- `federation-git-import`: The convergent import loop must store origin SHA-1 mappings between passes and re-resolve tree entry BLAKE3 references after the final retry so that cross-batch dependencies produce correct DAG links.
- `federated-git-clone`: The clone must succeed for repositories with 30K+ objects where the multi-round sync spans many batches. The sync must use a single persistent QUIC stream rather than one stream per round.

## Impact

- **`crates/aspen-federation/src/sync/client.rs`** — `sync_remote_objects` changes from opening a new `open_bi()` per call to accepting an existing `(SendStream, RecvStream)` pair. New `open_sync_stream()` / `SyncSession` API that callers hold for the conversation lifetime.
- **`crates/aspen-federation/src/sync/handler.rs`** — `handle_federation_stream` changes from single request/response to a loop reading multiple requests on the same stream. Stream counting becomes unnecessary (one stream per sync conversation).
- **`crates/aspen-forge-handler/src/handler/handlers/federation_git.rs`** — `sync_from_origin` opens one stream and passes it through the multi-round loop.
- **`crates/aspen-forge-handler/src/handler/handlers/federation.rs`** — `federation_import_objects` gains origin SHA-1 cross-pass resolution in the convergent loop and a post-retry DAG re-resolution pass that walks imported trees and fixes stale BLAKE3 references.
- **`crates/aspen-forge/src/git/bridge/importer.rs`** — May need a `re_resolve_tree_references()` method that reads a stored tree, re-looks up each entry's SHA-1 → BLAKE3, and re-stores the tree if any references changed.
- **`scripts/dogfood-federation.sh`** — Primary validation vehicle (no code changes).
- **`tests/forge_federation_test.rs`** — Integration tests for single-stream sync and DAG integrity.
