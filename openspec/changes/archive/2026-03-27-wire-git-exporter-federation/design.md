## Context

The federation protocol handler on Alice's node receives `SyncObjects` requests from Bob. It dispatches to `ForgeResourceResolver::sync_objects`, which checks `self.git_exporter`. When `None`, it logs "git objects requested but no exporter configured" and returns zero objects. The resolver has a `with_git_exporter()` constructor but `src/node/mod.rs` uses `new()` (no exporter).

The `GitExporter` needs a `HashMappingStore`, `BlobStore`, `RefStore`, `SecretKey`, and `HLC` — all available at federation setup time from the existing `ForgeNode` or node context.

## Goals / Non-Goals

**Goals:**

- `sync_objects` returns git objects when `want_types` includes "commit", "tree", "blob"
- Federated git clone produces a working tree with file content

**Non-Goals:**

- Changing the federation protocol or wire format
- Restructuring the exporter or resolver

## Decisions

### D1: Construct exporter from available node context

At federation setup time in `src/node/mod.rs`, construct a `GitExporter<K, B>` from the existing `ForgeNode` components (or directly from the KV, blob store, ref store, secret key, HLC that are already in scope). Pass it to `ForgeResourceResolver::with_git_exporter`.

The blob store and KV are already used for other things at this point, so no new dependencies are needed.
