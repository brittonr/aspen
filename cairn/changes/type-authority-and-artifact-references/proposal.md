# Proposal: Type authority and artifact references

## Why

Molten authority records distinguish principals, nodes, actors, services, sessions, authority contexts, delegations, revocations, keys, policies, resources, evidence, artifacts, operations, and receipts in Preserves fields.

Many Rust core models still carry these references as `String`. A value can therefore cross the wrong internal boundary before runtime admission reports the mismatch.

The authority and artifact cores need nominal Rust references while preserving canonical Preserves as the interchange authority.

## What Changes

- Inventory authority-bearing and artifact-bearing string references in pure core and adapter boundaries.
- Add private generic reference families with explicit domain markers and checked grammar.
- Define exact aliases for principal, node, actor, service, session, authority context, delegation, revocation, key, policy, resource, evidence, artifact, operation, and receipt references.
- Separate Preserves wire DTOs from admitted typed core models.
- Migrate authority admission, capability proofsets, effect admission, node control, artifact binding, retention, provenance, and replay seams in bounded groups.
- Preserve canonical Preserves bytes, receipt refs, legacy schema versions, and external wire fields.
- Add compile-fail, parser, canonical-byte, replay, and authority negative fixtures.
- Adopt the future Octet nominal-domain policy for the migrated scopes.

## Impact

- **Core**: Authority, capability, artifact, effect, node, retention, provenance, and replay models.
- **Shells**: Preserves decoding and CLI argument admission only.
- **Compatibility**: Existing Preserves schemas and receipt identities remain stable.
- **Concurrent changes**: Artifact-binding and semantic-effect work retains ownership of binding and operation semantics. This change supplies nominal Rust reference values only.

## Non-goals

- Do not treat a typed identity or reference as authority.
- Do not replace UCAN, Basalt, policy, provenance, resource, or revocation checks.
- Do not wrap display labels, diagnostic text, or every string in the codebase.
- Do not change canonical Preserves schemas in place.
- Do not claim remote, transport, runtime, or release correctness.
