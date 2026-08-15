## Why

Molten's architecture names Trellis choreography as the finite protocol-shape layer, but no end-to-end protocol installation or session interpreter exists yet. To move beyond ad hoc remote envelopes, Molten needs protocol manifests that compile to Trellis, project to local endpoints, and drive dataspace messages with explicit receipts.

## What Changes

- Add canonical protocol manifest, installation receipt, endpoint projection, session state, and protocol-message envelope records.
- Lower role/label/payload names to Trellis ids and reject non-projectable choreographies before installation.
- Interpret projected local endpoints over the dataspace/remote dataspace boundary.
- Bind sequence/replay, payload schema refs, policy/capability/resource refs, and Trellis predicate evidence into per-operation receipts.
- Add replay-bound lifecycle gate receipts that recompute install/operation evidence and prove terminal session state without granting authority.
- Add a first two-role request/response protocol example and tests.

## Impact

This makes Trellis a working protocol gate rather than a design reference. It creates the bridge from SAM dataspaces to finite multi-party workflows without using Raft for ordinary protocol messages.
