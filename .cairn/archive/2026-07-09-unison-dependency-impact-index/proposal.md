## Why

Unison's structured codebase is valuable because tools can ask dependency and dependent questions without scraping source text. Molten needs the same property for runtime artifacts: operators should know what depends on a schema, effect, migration, policy, protocol, transcript, handler profile, or artifact before changing or deleting it.

Molten already records dependency refs in several artifacts. This change makes dependency edges and reverse-impact indexes explicit, deterministic, rebuildable, and receipt-backed.

## What Changes

- Add canonical dependency-edge records for artifact-to-artifact, artifact-to-schema, artifact-to-policy, artifact-to-effect, artifact-to-capability, transcript, storage, and release relationships.
- Maintain deterministic reverse dependency indexes that can be rebuilt from registry and ledger records.
- Add impact query receipts for upgrade planning, retention/GC, release review, and catalog/MCP inspection.
- Add validation fixtures for complete graphs, missing edges, stale reverse indexes, duplicate edges, cycles, and unauthorized hidden dependency exposure.

## Impact

- **Files**: artifact registry, catalog, upgrade sessions, retention/GC planning, release evidence, fixtures.
- **Testing**: positive fixtures for direct and transitive impact queries; negative fixtures for stale indexes, missing dependencies, duplicate edges, and redaction leaks.
- **Security**: dependency indexes support decisions but do not grant authority, retention rights, provenance, or policy admission.