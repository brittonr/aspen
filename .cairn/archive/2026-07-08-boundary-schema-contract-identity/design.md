# Design: boundary schema contract identity

## Scope

This change strengthens schema identity for repository-owned Preserves boundary schema specs. It does not change authority, policy, provenance, resource, transport, or execution gates, and it does not make a schema pass sufficient for semantic admission.

## Proof checklist

- **Proof claim**: boundary schema refs change whenever the reviewed field contract changes.
- **Out of scope**: generating all Rust DTOs from schemas or changing existing Preserves record layouts.
- **Trusted assumptions**: reviewed `BoundarySchemaSpec` values are the source of truth for the first allowlist of boundary records.
- **Positive evidence**: unchanged specs produce stable schema refs and existing valid boundary fixtures still pass.
- **Negative evidence**: field reorder, same-arity kind changes, label changes, and constraint changes produce different schema refs or stale-schema denials.
- **Canonical refs**: boundary family, schema artifact ref, field contract list ref, validation report ref.
- **Regeneration command**: focused `preserves_rail` schema tests plus the Cairn validation gate.

## Functional core

Add a pure function that converts a `BoundarySchemaSpec` into a canonical contract descriptor value containing family, version, record label, schema id, arity, and ordered field descriptors. The schema ref is derived from this descriptor, not from arity alone.

## Imperative shell

Call sites continue to pass `BoundarySchemaSpec` values to validation. Receipt or diagnostic writers read the strengthened schema ref from the pure core and do not inspect the filesystem, network, clock, or runtime environment.

## Migration

Start with the existing schema-backed allowlist. If any checked-in expected refs exist, update them in the same change with before/after evidence and a short migration note.
