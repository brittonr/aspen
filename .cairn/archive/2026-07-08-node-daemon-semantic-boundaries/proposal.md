## Why

`src/node/daemon.rs` is one of the largest shard entry points in the tree. It mixes startup, locks, inboxes, ingress, dispatch, supervisor policy, live transport, workflow bundles, and receipt construction in one shared namespace, making node-runtime behavior difficult to review or extract.

## What Changes

- Split node daemon internals into semantic modules with stable ownership.
- Separate pure node planning and admission from filesystem, service-lock, live-Iroh, and control-socket shells.
- Preserve current node CLI behavior and canonical node receipts during migration.
- Add positive and negative tests around one extracted node daemon boundary.

## Impact

The node daemon becomes a reviewable set of smaller boundaries, preparing it for runtime crate extraction and reducing the chance that transport or filesystem availability is mistaken for node authority.
