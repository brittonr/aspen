## Why

Issue #48: the upgrades store wrote plans, receipts, status files and name pointers through a local shim over ambient `std::fs`. Paths were joined onto an ambient root, every write truncated in place without synchronization or rename, and a symlink planted under the store could redirect a write or a read. AGENTS.md requires capability-rooted I/O for ongoing state.

## What Changes

- A typed `local_store::UpgradeStoreRoot` holds the store directory capability. The CLI opens it once from the operator-selected path; every upgrades function takes the root instead of a `Path`, and the ambient `fs` shim is removed.
- `local_store::StorageRoot` gains `publish` (atomic replacement) and `publish_new` (atomic creation without replacement) over the node-state staged publication helper, and its listings skip publication stages.
- Plans and receipts are content-addressed and published without replacement; republishing identical bytes succeeds and different bytes are a reported conflict. Name pointers and status files are replaced atomically. `set_name_pointer` stores its receipt before the pointer.
- Reads, recursive scans and leaf writes refuse symlinks and non-regular members; scans ignore interrupted publication stages. The on-disk layout and file names are unchanged, so existing stores remain readable.

## Impact

- **Files**: `crates/molten-node-host/src/local_store/parts/mod/{p000,p001}/body.rs`, `crates/molten-node-host/src/node/{state.rs,state/publication.rs}`, `src/upgrades/parts/mod/*`, `src/cli/runtime/upgrade/ops.rs`, upgrades tests and the CLI upgrade test, `docs/node-state-filesystem-authority.md`.
- **Testing**: the upgrades suite, new store tests for immutable conflicts, whole pointer replacement, invisible stages and refused links, node-host tests, and the CLI upgrade workflow test.
- **Owner / consumer**: `molten-node-host` owns typed store roots; the upgrades module and `molten upgrade` consume them.
