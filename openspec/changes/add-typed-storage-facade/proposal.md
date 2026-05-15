## Why

Unison Cloud's `Cell`, `OrderedTable`, `Blobs`, `Config`, `Scratch`, and transactional storage APIs show a useful operator/developer abstraction: typed durable resources over a distributed substrate. Aspen has Raft-backed KV, redb storage, blob storage, secrets, and Snix artifacts, but higher-level consumers still hand-roll key encoding, schema compatibility, and receipt boundaries.

Aspen should add a typed storage facade that preserves explicit serialization and schema hashes while giving services/jobs a safer API for cells, ordered tables, typed blobs, config secrets, scratch values, and batched reads.

## What Changes

- Define typed storage resource models: `Cell<T>`, `OrderedTable<K,V>`, `TypedBlob<T>`, `ConfigSecret`, `Scratch<T>`, and `Batch`.
- Require schema/type hashes and explicit codec/version metadata for durable values.
- Require transactional semantics for multi-key/table updates where backed by Raft/KV.
- Require migration/readback errors for schema mismatch instead of silent decode.

## In Scope

- Spec and initial implementation for one or two resource types, preferably `Cell<T>` and `OrderedTable<K,V>`.
- Explicit schema-hash validation and bounded receipts.
- Negative tests for decode/schema mismatch and secret redaction.

## Out of Scope

- Magical serialization of arbitrary functions.
- Replacing redb/Raft KV internals.
- Full relational query language.

## Verification

- `openspec validate add-typed-storage-facade --strict`
- Focused unit/property tests for typed resource keys, schema hashes, transactions, and mismatch failures.
- Secret redaction tests for config-like resources.
- `openspec validate --all --strict --json`
- `git diff --check`
