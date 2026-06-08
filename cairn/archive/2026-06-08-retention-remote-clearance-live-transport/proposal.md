## Why

Molten can now build local remote-clearance requests, produce peer responses, and import passing peer clearance values. Operators still have to move request and response artifacts by hand. Destructive retention against remote refs needs a live peer workflow that carries those artifacts through node-control live transport while preserving the existing deletion-safety evidence model.

## What Changes

- Add a node-control live retention-clearance transport workflow that packages a `retention-remote-gc-clearance-request-v1`, carries it to a peer, and returns a `retention-remote-gc-clearance-response-v1`.
- Add CLI support for a deterministic local/loopback live workflow so operators can exercise request/send/respond/import without manual artifact handoff.
- Bind live workflow receipts to request, response, import, peer, remote, object, and diagnostics while keeping transport receipts separate from authority and policy.
- Add tests for passing loopback import, retained/stale peer denial, wrong peer/request denial, and tampered response denial.

## Impact

- **Files**: `src/node_daemon.rs`, `src/main.rs`, `src/retention.rs`, README/docs, Cairn runtime-spine specs.
- **Behavior**: Adds operator-facing live transport for remote retention clearance evidence. Existing local request/respond/import commands remain valid.
- **Trust boundary**: Live transport receipts remain transport/evidence records only. They do not grant deletion authority, policy, resource, provenance, execution, source-gate, or remote-GC clearance by themselves.
