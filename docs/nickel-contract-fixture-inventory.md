# Nickel contract fixture inventory

Runtime Rust consumes checked JSON or Preserves exports. These Nickel modules are authoring-time review gates only; a passing Nickel export does not grant runtime authority, freshness, provenance, transport, resource, retention, source-gate, or deployment trust.

## Shared prelude

`docs/nickel-contract-prelude.ncl` provides pure helpers for common domains: non-empty strings and arrays, BLAKE3 refs, stable ids, semantic versions, absolute and safe relative paths, positive and non-negative integers, exact metadata values, allowed-value membership, required-value coverage, uniqueness, and metadata envelopes.

## Production profile contracts

Module: `docs/production-profile-contracts.ncl`

Positive fixtures:

- `docs/production-node-profile.ncl`
- `docs/production-profile-fixtures/valid.ncl`

Negative fixture classes:

- malformed refs: `malformed-ref.ncl`
- missing metadata or required adapters: `missing-metadata.ncl`, `missing-required-adapter.ncl`
- unsafe paths and layout collisions: `unsafe-state-path.ncl`, `unsafe-layout-dir.ncl`, `layout-collision.ncl`
- invalid limits: `zero-limit.ncl`, `fractional-limit.ncl`, `incoherent-resource-limits.ncl`
- unsupported vocabulary/source-gate metadata: `vocabulary-typo.ncl`, `unsupported-metadata.ncl`, `missing-source-gate.ncl`

## Peer profile contracts

Module: `docs/peer-profile-contracts.ncl`

Positive fixture:

- `docs/peer-profile-fixtures/valid.ncl`

Negative fixture classes:

- missing bootstrap evidence: `missing-bootstrap.ncl`
- malformed refs and duplicate identities: `malformed-peer-ref.ncl`, `duplicate-peer-ref.ncl`

## Multinode scenario contracts

Module: `docs/multinode-scenario-contracts.ncl`

Positive fixtures:

- `docs/multinode-scenario-fixtures/valid/*.ncl`

Negative fixture classes:

- missing topology or command surface: `missing-topology.ncl`, `missing-command-surface.ncl`
- stale or mismatched evidence refs: `stale-receipt-ref.ncl`, `mismatched-artifact-kind.ncl`
- undeclared variance and unsupported pass claims: `undeclared-variance.ncl`, `unsupported-pass-claim.ncl`

## Plugin extension contracts

Modules: `docs/plugin-extension-contracts/contract.ncl`, `grant.ncl`, and `envelope.ncl`

Positive fixtures:

- `storage.ncl`
- `storage.grant.ncl`
- `storage-revoked.grant.ncl`
- `storage.contract-envelope.ncl`
- `storage.grant-envelope.ncl`

Negative fixture classes:

- malformed or missing refs/schema: `storage-malformed-ref.ncl`, `storage-missing-schema.ncl`, `storage-malformed-ref.grant.ncl`
- invalid ids, profiles, and versions: `storage-invalid-extension-id.ncl`, `storage-invalid-profile.ncl`, `storage-invalid-version.ncl`
- empty evidence arrays: `storage-empty-evidence.ncl`, `storage-empty-effect-receipts.grant.ncl`
- duplicate descriptor identities: `storage-duplicate-descriptor.ncl`
- grant invariants: `storage-missing-proof.grant.ncl`, `storage-over-delegation.grant.ncl`, `storage-inverted-validity.grant.ncl`, `storage-revoked-missing-evidence.grant.ncl`
- envelope metadata: `storage-envelope-missing-schema.ncl`, `storage-envelope-missing-identity.ncl`, `storage-envelope-unsupported-schema.ncl`, `storage-envelope-unsupported-source.ncl`

## Cairn policy contracts

Modules: `cairn-policy/contracts.ncl`, `default.ncl`, and `structured-lifecycle-contracts.ncl`

Positive fixtures:

- `cairn-policy/default.ncl`
- `cairn-policy/fixtures/valid-with-exemption.ncl`

Negative fixture classes:

- malformed scalar domains: `invalid-marker.ncl`, `invalid-receipt-hash-policy.ncl`, `invalid-exemption-marker-policy.ncl`
- stale internal refs: `invalid-artifact-dependency.ncl`, `invalid-determinism-surface-coverage.ncl`, `invalid-determinism-surface-group.ncl`, `invalid-replay-group-case.ncl`, `invalid-receipt-contract-command.ncl`
- duplicate identities: `invalid-duplicate-artifact.ncl`, `invalid-duplicate-marker.ncl`, `invalid-duplicate-marker-token.ncl`, `invalid-duplicate-replay-case.ncl`, `invalid-duplicate-replay-group.ncl`, `invalid-duplicate-receipt-schema-command.ncl`

## Diagnostic caveats

Nickel reports field-local contract failures precisely for scalar, enum, ref, path, and required-field domains. Whole-record predicates still report at the enclosing contract when the invariant spans multiple arrays or records, including plugin descriptor uniqueness, grant attenuation coherence, revoked-grant evidence, Cairn policy cross-references, and policy-wide duplicate ids. Those predicates remain whole-record checks to avoid weakening validation or duplicating generated data into artificial diagnostic fields.

## Fixture exemptions

No repository-owned Nickel fixture is exempt from execution. The `contract-export-drift-gate` check evaluates all listed positive and negative fixture classes locally and deterministically.
