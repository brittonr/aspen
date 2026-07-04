# Design: Contract export envelope standardization

## Context

Production profiles already use explicit metadata. Plugin extension authoring and some fixture exports should follow the same evidence discipline so generated artifacts are self-describing.

## Envelope

Use a small common shape for Nickel-authored exports:

- `schema_id`: exact reviewed schema identifier.
- `schema_version`: exact reviewed version number.
- `source_language`: exact `nickel` marker.
- `export_identity`: stable id for the profile, fixture, contract, or grant.
- `payload`: the contract-specific reviewed value.

Preserves exports may map this metadata to records that are compatible with existing boundary schemas, or introduce a reviewed wrapper when compatibility requires the old payload shape to remain unchanged.

## Migration

Start with plugin extension contract and grant authoring surfaces because they currently depend most on filename/context. Then align peer profile and multinode fixture exports if any metadata is missing or named inconsistently.

## Boundary

Metadata makes evidence inspectable; it is not authority. Receipts must continue to bind content refs, admission decisions, and subsystem-specific gate evidence.
