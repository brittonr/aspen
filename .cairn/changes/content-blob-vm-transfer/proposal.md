# Bounded live content handoff for VM peers

## Why

The existing Iroh Blobs adapter transfers verified chunks, but its client borrows a process-local publication containing the server router. Onix needs a real two-VM replay of an existing signed archive. It must not copy this adapter or confuse local-directory compatibility helpers with network transfer.

## What Changes

Separate a bounded public locator handoff from private publication state. Add an opt-in read grant bound to one canonical manifest and explicit authenticated reader keys. Reuse existing chunk verification, content transitions, opaque transport identity, and pins. Add a fixture-only executable for VM storage/client roles and retain the existing API as a compatibility wrapper.

## Non-goals

No host service, public deployment, package trust, arbitrary node control, new signature scheme, Stage0, toolchain rebuild, or production-readiness claim. A configured read grant is local operator policy, not a Basalt credential or global authority proof.

## Impact

Molten owns handoff and serving semantics. Onix owns archive binding and VM/replay evidence. Existing content identities and old APIs stay unchanged.
