# Change: plugin-extension-domain-contract-hardening

## Why

Plugin extension Nickel authoring contracts currently accept broad `String`, `Number`, and `Array` shapes while the Rust admission layer enforces tighter runtime invariants. That gap lets malformed refs, empty evidence arrays, duplicate descriptors, invalid profiles, and incoherent grant attenuation survive authoring review until a later Preserves/Rust gate rejects them.

## What

- Tighten `docs/plugin-extension-contracts/contract.ncl` and `grant.ncl` with domain-specific Nickel predicates for refs, ids, versions, profiles, replay classes, required non-empty evidence arrays, descriptor uniqueness, proof refs, and attenuation windows.
- Keep Rust admission authoritative over checked-in Preserves evidence, but make authoring-time validation fail earlier for the same classes of invalid inputs.
- Add positive and negative fixtures that demonstrate valid exports and fail-closed authoring rejection for each new invariant.

## Impact

Plugin extension authors get earlier, review-local feedback. Runtime behavior remains fail-closed and continues to consume reviewed canonical Preserves exports rather than evaluating Nickel as authority.
