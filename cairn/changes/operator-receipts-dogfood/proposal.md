## Why

Molten needs an operator confidence rail that proves the runtime can run itself, emit durable evidence, and let operators inspect what happened without reading logs. Aspen's dogfood-local flow and operator receipts are useful prior art: run an end-to-end self-host path, publish a final success receipt, and make local or cluster-backed receipt readback easy.

## What Changes

- Add a Molten dogfood workflow that exercises core runtime slices end to end.
- Emit operator-visible receipts for startup, config load, artifact install, local routing, policy decisions, storage, remote-like sync, transcript execution, and cleanup.
- Store final success/failure receipts in durable local metadata and later Raft-backed control-plane state when available.
- Add CLI surfaces for listing, showing, exporting, and validating receipts.
- Make dogfood receipts suitable for CI, local debugging, deterministic playback, and upgrade gates.

## Impact

This creates a practical evidence loop for Molten. The first milestone can implement `molten dogfood local` using the local dataspace, artifact registry, deterministic handlers, and receipt inspection commands.
