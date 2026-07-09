## Why

`molten-core::stack` already validates stack evidence roles, BLAKE3 artifact refs, verification roles, and evidence-only non-claims for Basalt, UCAN, Trellis, Octet, Valence, Cairn, and Mantle. Valence is the canonical evidence identity layer for the same stack. Keeping a Molten-only role vocabulary risks drifting from Valence's role registry and stack-provenance non-claim contracts.

This change defines a Molten-to-Valence stack evidence adapter so Molten can keep runtime and release evidence composition local while delegating shared identity/role compatibility to Valence contracts.

## What Changes

- Define a Molten stack evidence adapter row that maps `StackEvidenceMember` roles to Valence role/schema vocabulary.
- Preserve Molten ownership of runtime authority, release promotion, signed receipts, and local evidence gates.
- Add positive fixtures for a complete evidence-only stack envelope.
- Add negative fixtures for missing roles, malformed BLAKE3 refs, unsupported schemas, missing verification roles, missing evidence-only non-claims, and overbroad authority claims.
- Document that the adapter proves stack evidence identity/role compatibility only.

## Impact

- **Molten** can consume Valence role/schema contracts without making Valence a runtime authority oracle.
- **Valence** remains the shared evidence identity source of truth.
- **Aspen/Molten tests** gain a clean migration path away from local-only stack role helpers.
