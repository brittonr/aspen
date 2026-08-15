# Tasks: shared-bounded-sinks

## Phase 1: Shared helper core

- [x] [serial] r[molten.shared_bounded_sinks.checked_counts] Add checked add/multiply/count helpers and generic bounded push/extend functions to the shared bounded module.
- [x] [parallel] r[molten.shared_bounded_sinks.diagnostic_sink] Add a reusable diagnostic sink adapter that uses checked arithmetic and preserves fail-closed behavior.
- [x] [parallel] r[molten.shared_bounded_sinks.negative_bounds] Add helper-level tests for exact maximum, one-past maximum, arithmetic overflow, and no-mutation-on-error.

## Phase 2: Call-site migration

- [x] [serial] r[molten.shared_bounded_sinks.migration] Migrate duplicated bounded helpers in plugin, node, testing, coordination, retention, runtime dataspace, and parser modules where behavior is equivalent.
- [x] [parallel] r[molten.shared_bounded_sinks.hash_stability] Add representative fixture hash-stability checks for migrated receipt builders.

## Phase 3: Validation

- [x] [serial] r[molten.shared_bounded_sinks.checked_counts] r[molten.shared_bounded_sinks.migration] Run focused helper/module tests and `nix run path:$PWD#cairn -- validate --root .`.
