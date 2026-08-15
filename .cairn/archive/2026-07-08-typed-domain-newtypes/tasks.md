# Tasks: typed-domain-newtypes

## Phase 1: Domain core

- [x] [serial] r[molten.typed_domains.content_refs] Reuse `ContentRef` in migrated DTOs and add sequence helpers for typed content refs.
- [x] [parallel] r[molten.typed_domains.decisions] Add typed decisions and check statuses with pure parse/format methods.
- [x] [parallel] r[molten.typed_domains.identifiers] Add typed stable ids, schema ids, operation ids, replay classes, and profile ids for high-risk boundaries.

## Phase 2: Migration

- [x] [serial] r[molten.typed_domains.migrated_dtos] Migrate representative plugin, capability, chunk, evidence, and consensus DTOs away from raw strings where domain rules are known.
- [x] [parallel] r[molten.typed_domains.hash_stability] Add before/after canonical fixture checks for migrated record builders.

## Phase 3: Tests and validation

- [x] [parallel] r[molten.typed_domains.negative_domains] Add negative tests for malformed refs, unsupported decisions, invalid ids, and unsupported replay classes.
- [x] [serial] r[molten.typed_domains.migrated_dtos] Run focused parser/newtype tests and `nix run path:$PWD#cairn -- validate --root .`.
