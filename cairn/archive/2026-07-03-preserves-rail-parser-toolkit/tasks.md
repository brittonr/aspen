# Tasks: preserves-rail-parser-toolkit

## Phase 1: Shared helpers

- [x] [serial] r[molten.preserves_rail_toolkit.parser_builders] Add shared parser and builder helpers for simple records, required strings, content refs, optional refs, ref sequences, and schema fields.
- [x] [parallel] r[molten.preserves_rail_toolkit.check_sets] Add shared check-list builders/parsers with required-check diagnostics.
- [x] [parallel] r[molten.preserves_rail_toolkit.negative_shapes] Add helper-level negative tests for wrong label, wrong arity, wrong type, invalid ref, missing check, and duplicate unsupported check cases.

## Phase 2: Migration

- [x] [serial] r[molten.preserves_rail_toolkit.hash_stability] Migrate a first module group and prove representative canonical hashes are unchanged.
- [x] [serial] r[molten.preserves_rail_toolkit.parser_builders] Migrate additional duplicated helper call sites in schema identity, service, job, protocol, node runtime, retention, catalog, plugin, and evidence modules.

## Phase 3: Validation

- [x] [parallel] r[molten.preserves_rail_toolkit.hash_stability] Add before/after fixture tests for migrated record families.
- [x] [serial] r[molten.preserves_rail_toolkit.parser_builders] r[molten.preserves_rail_toolkit.check_sets] Run focused module tests and confirm public CLI fixture output remains stable.
