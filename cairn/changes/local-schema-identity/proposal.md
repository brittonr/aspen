## Why

Molten now has typed storage, storage migrations, upgrade sessions, and a local artifact registry. The remaining weak point is schema comparison. Today most trust boundaries either require exact schema refs or hand off mismatches to explicit migration recipes. That is safe but too coarse: some schemas are intentionally structural and should be reusable by fingerprint, while domain-specific schemas with the same shape must remain unique unless an alias or migration is explicitly admitted.

A local schema-identity layer gives typed storage, effect schemas, policy contracts, protocol payloads, and upgrade sessions a common compatibility decision model before exposing or transforming values.

## What Changes

- Add canonical schema identity artifacts with explicit modes: `structural`, `unique`, and `branded-structural`.
- Compute domain-separated structural fingerprints over normalized Preserves schema/value-shape metadata, independent of names, docs, filesystem paths, and registry aliases.
- Add schema alias artifacts/metadata for admitted unique-schema equivalence without changing artifact identity.
- Add structured compatibility decisions: exact artifact match, structural match, brand match, admitted alias, migration available, mismatch requiring migration, and policy denial.
- Emit receipts for compatibility decisions and alias admissions.
- Integrate typed-storage loads/writes/migrations with schema identity decisions while preserving fail-closed behavior for unique schemas and missing policy evidence.
- Add local CLI commands for fingerprinting, compatibility checks, alias artifacts, and registry search by fingerprint.

## Impact

This strengthens typed-storage migration and artifact-registry semantics. It prevents accidental interchange of nominally different schemas with equal shapes, while allowing explicitly structural schemas to interoperate without unnecessary migration recipes. Later choreography, effect-handler, and policy-contract integrations can reuse the same compatibility receipts.
