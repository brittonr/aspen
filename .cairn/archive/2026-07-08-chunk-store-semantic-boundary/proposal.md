## Why

Chunk store code mixes manifest identity, filesystem byte IO, Redb indexes, Iroh exchange, retention hooks, lineage evidence, and CLI/test helpers. Because chunks are a core content-addressed boundary, these responsibilities should be explicit and independently testable.

## What Changes

- Split chunk store into semantic boundaries for model, manifest codec, chunk filesystem IO, index adapter, Iroh exchange, retention integration, lineage receipts, and shell orchestration.
- Preserve canonical manifest and chunk refs during migration.
- Add tests for byte-preserving manifests and denied retention/IO plans.

## Impact

The chunk store remains content-addressed and evidence-bearing while becoming a better candidate for standalone storage and exchange crates.
