## Why

The existing service-supervision roadmap is broad enough to describe the target, but the implementation needs a smaller first Cairn that can land without starting actors or changing scheduler behavior. Molten needs stable canonical service records before demand-driven startup, monitoring, or dogfood workflows can rely on service evidence.

## What Changes

- Define canonical service manifest, demand, status, lifecycle, supervisor, restart-policy, and cleanup receipt records.
- Require explicit authority, policy, resource, effect-handle, actor/artifact, dependency, and provided-assertion refs in service manifests.
- Classify service artifacts and receipts in the local ledger, artifact registry, catalog, and read-only MCP views.
- Add safe rendered summaries that never replace canonical Preserves receipts as normative evidence.

## Impact

This creates the stable evidence substrate for SAM service supervision. It is intentionally schema/ledger/catalog first: no service is started by this change alone, and all runtime behavior remains fail-closed until the demand-runtime Cairn lands.
