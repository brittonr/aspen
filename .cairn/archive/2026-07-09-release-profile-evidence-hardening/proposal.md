## Why

Molten has strong typed Nickel contracts for production profiles and release evidence, but several current reviewed examples still use placeholder BLAKE3 refs and optional stack-provenance settings. That is acceptable for fixtures and local diagnostics, but release-scoped profiles should fail closed unless the evidence refs are real, current, and explicitly scoped.

Release evidence should distinguish development fixtures, pilot review evidence, and release-candidate evidence so placeholder refs and optional upstream stack provenance cannot accidentally look production-ready.

## What Changes

- Split deployment profile expectations into explicit development, pilot, and release tiers.
- Reject zero/dummy placeholder BLAKE3 refs in release-scoped deployment profiles and release promotion inputs.
- Require release profiles to bind reviewed source-gate, policy, Octet/Cairn, stack-provenance, and production-profile refs rather than fixture placeholders.
- Make stack-provenance policy required for release-tier evidence with non-placeholder accepted Valence policy hashes while keeping its evidence-only boundary explicit.
- Add positive pilot fixture coverage and negative release fixture coverage for zero refs, repeated dummy refs, optional stack provenance, stale refs, and missing release evidence.

## Impact

- **Files**: production profile contracts/fixtures, Cairn policy defaults, release/dogfood evidence gates, operator runbooks, and tests.
- **Testing**: positive dev/pilot fixtures; negative release fixtures for placeholders, stale refs, optional stack provenance, and missing required evidence.
- **Safety**: stronger release config evidence still does not grant runtime authority, policy, provenance, resource, transport, source-gate, retention, destructive-operation, or deployment trust by itself.
