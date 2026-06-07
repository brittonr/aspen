## Why

The Raft-backed coordination control-plane exists, but operators need a durable way to generate canonical manifests/requests, apply request batches, and inspect evidence without relying on a hard-coded fixture. Remote/live workers, service supervision, and job queues need a simple CLI path that still routes every mutation through coordination request receipts.

## What Changes

- Add `molten test coordination manifest` and `request` commands that emit canonical coordination records.
- Add `molten test coordination apply` to replay one or more request artifacts through the coordination control-plane runtime and write a receipt/evidence directory.
- Add a canonical apply report that binds the manifest, final state, receipt refs, assertion refs, and evidence refs.
- Extend `molten test coordination show` to summarize requests, fencing tokens, and apply reports as read-only artifacts.

## Impact

Coordination services become usable from operator scripts while preserving the existing trust model: CLI-generated records are evidence, and mutations still require authority, policy, resource, operation-id, and Raft/control-plane receipts before state changes.
