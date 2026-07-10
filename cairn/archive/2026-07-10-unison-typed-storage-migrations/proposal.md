## Why

Unison's durable typed storage idea is useful because persisted values carry enough type/code identity to survive code evolution. Molten should adapt that principle for canonical Preserves values stored with schema, artifact, policy, capability, and migration evidence.

This change hardens typed storage around evolution: a stored value can be read months later only when its schema/artifact bindings are understood and any migration is backed by admitted recipe receipts.

## What Changes

- Require typed storage records to bind value ref, schema identity, producing artifact, intended consumers, policy refs, capability refs, and evidence refs.
- Add migration recipe gates that bind source schema, target schema, executable recipe artifact, effect manifest, handler profile, policy, provenance, and test evidence.
- Require compatibility receipts before reading a value under a different expected schema or artifact contract.
- Deny arbitrary serialized functions, mutable names, or raw decoder claims as storage identity.

## Impact

- **Files**: typed storage, schema identity, artifact registry, migration recipes, eval cache, fixtures.
- **Testing**: positive fixtures for compatible reads and admitted migrations; negative fixtures for missing schema refs, wrong unique identity, stale migration recipe, unadmitted decoder, and function serialization.
- **Security**: storage type identity helps interpret data but does not grant capability, retention, policy, provenance, or execution rights.