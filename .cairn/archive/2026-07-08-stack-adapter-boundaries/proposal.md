## Why

Molten currently depends directly on several stack repos and crates: Basalt for policy/UCAN enforcement, Cairn core for lifecycle evidence, Octet-hosted Valence code, Trellis/verified-logic, and related transitive UCAN surfaces. That makes the runtime architecture look tightly coupled to upstream implementation crates rather than to stable adapter contracts and evidence envelopes.

This change defines Molten's stack adapter boundary so runtime code consumes canonical receipts, refs, and DTOs through explicit ports instead of reaching into upstream internals.

## What Changes

- Add a stack adapter boundary spec for Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle evidence inputs.
- Define a canonical Molten admission/evidence envelope that references upstream evidence by schema, role, BLAKE3 identity, and non-claim boundary.
- Add adapter-port guidance: upstream-specific crates live behind shell adapters; pure runtime cores consume parsed facts and refs.
- Add positive and negative fixtures for complete evidence envelopes, missing required roles, stale refs, overbroad claims, and direct-internal dependency leaks.

## Impact

- **Molten runtime cores** depend on stack facts, not upstream I/O or command surfaces.
- **Stack integrations** become narrower and easier to validate.
- **Downstream operators** can inspect which upstream receipt or contract supported a runtime decision.
- **Testing** needs envelope contract fixtures and dependency-boundary checks.
