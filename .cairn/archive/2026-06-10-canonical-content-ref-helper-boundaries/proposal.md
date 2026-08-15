# Proposal: Canonical Content-Ref Helper Boundaries

## Summary

Tighten Molten's canonical content-ref discipline by making construction and filename/readback conversion explicit shared-helper responsibilities, not ad-hoc string formatting at subsystem boundaries.

## Motivation

Molten already requires canonical `blake3:<64 lowercase hex>` refs for runtime and evidence boundaries. Recent implementation work centralized construction and readback helpers in `preserves_rail`; the lifecycle specs should name those boundaries so future subsystems do not reintroduce manual `blake3:` concatenation, unchecked filename reconstruction, or unscoped transitional `b3:` aliases.

## Scope

- Specify shared helper construction for bytes, BLAKE3 hash values, and validated lowercase hex.
- Specify fail-closed materialized filename/readback conversion.
- Specify that transitional aliases such as Octet `b3:` evidence stay explicitly scoped and are derived from canonical refs.
- No new authority, provenance, policy, retention, source-gate, resource, or transport trust is granted by content-ref shape.

## Non-goals

- Changing the canonical content-ref algorithm.
- Treating `b3:` as a runtime content-ref format.
- Replacing existing materialized byte readback checks.
