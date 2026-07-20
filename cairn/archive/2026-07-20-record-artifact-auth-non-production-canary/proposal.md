## Why

Molten's first landed artifact-auth operational canary passed against real capability-file node state, but its public receipts and exact harness remain only in an unversioned workspace directory. A durable product-owned archive is required before any later authority evaluation can distinguish observed restart and rotation behavior from reconstructed claims.

## What Changes

- Archive the Molten-specific public artifacts from `artifact-auth/run-001` with the exact Molten and artifact-auth revisions.
- Preserve the exact temporary harness, operational receipt, successful fresh-process replay, generation rotation, stale-receipt denial, and fresh-process post-rotation status.
- Add a typed Nickel manifest and reproducible BLAKE3 inventory while excluding private key material and secret node state.
- Keep the canary explicitly non-production and non-authoritative; legacy behavior remains authoritative and rollback remains available.

## Impact

- **Files**: this Cairn change, its evidence bundle, and the accepted `artifact-auth-operational-receipt` specification.
- **Testing**: Nickel typecheck, deterministic BLAKE3 regeneration, negative symlink rejection, secret scan, expected-denial inspection, Cairn gates, sync, and archive validation.
