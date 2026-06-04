## Why

Molten nodes need stable identities across restarts for Iroh peer authentication, capability delegation, receipts, replay, federation, remote sync, and control-plane membership. Aspen's identity persistence model is useful prior art: a configured node id is not enough if the P2P endpoint key changes on every restart.

## What Changes

- Define persistent node identity records for Molten nodes and Iroh endpoints.
- Persist node endpoint secret material in an explicitly configured data directory or secret backend.
- Support explicit key override for controlled deployments and recovery.
- Detect and warn or deny unexpected endpoint identity drift.
- Bind node identity to authority contexts, peer bootstrap, receipts, replay snapshots, and cluster/control-plane membership.
- Keep node identity separate from authority: stable identity does not grant capabilities by itself.

## Impact

This gives Molten stable peer identity before real remote operation. The first milestone can load-or-generate an Iroh endpoint key, persist it with restricted permissions, emit identity receipts, and include the node identity hash in local runtime startup evidence.
