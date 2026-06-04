## Why

Molten's receipts and catalogs are useful only if operators can run real workflows against the node and trust the evidence. The project needs a dogfood workflow that installs artifacts, starts a node, runs a service, syncs/executions a job, gates the evidence, exports a repro bundle, and records a release/admission decision.

## What Changes

- Define canonical operator workflow, step, checkpoint, release gate, and dogfood report records.
- Add a named dogfood suite that uses the node daemon, service supervision, remote dataspace, job DAG, artifact registry, catalog, and repro bundle rails.
- Store operator receipts in the local evidence ledger and expose summaries via catalog/MCP.
- Require deterministic replay or explicit non-replayable classification for every dogfood step.
- Use dogfood pass receipts as local release/admission gates.

## Impact

This gives Molten an operational confidence rail and keeps implementation honest. It turns the many local slices into a single operator-visible workflow aligned with Aspen 2.0 goals.
