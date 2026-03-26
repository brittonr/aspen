## Why

Federation `sync_objects` returns `refs=1 objects=0` for git object types because the `ForgeResourceResolver` is created without a `GitExporter`. In `src/node/mod.rs`, the resolver is constructed as `ForgeResourceResolver::new(kv)` — the `new` path that sets `git_exporter: None`. The resolver's `sync_objects` skips git object export when no exporter is configured. One-line wiring fix.

## What Changes

- Wire `GitExporter` into `ForgeResourceResolver` at federation setup time in `src/node/mod.rs`, using `ForgeResourceResolver::with_git_exporter(kv, exporter)` instead of `ForgeResourceResolver::new(kv)`
- Verify the existing KV object store read path works end-to-end (objects are stored in KV by the importer, read by the exporter's `read_object_bytes`)
- Tighten the VM test to assert file content

## Capabilities

### New Capabilities

- `wire-git-exporter-federation`: Connect the git object exporter to the federation protocol handler so `sync_objects` returns git object content alongside refs.

### Modified Capabilities

## Impact

- `src/node/mod.rs` — federation setup resolver construction (1 line change + blob store access)
- `nix/tests/federation-git-clone.nix` — verify file content assertion passes
