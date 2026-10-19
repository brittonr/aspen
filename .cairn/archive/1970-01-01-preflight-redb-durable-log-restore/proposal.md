# Admit persisted Redb log against its current profile before loading values

## Why

`DurableNativeHostJournal::history` filters decoded records from `RedbDurableStateAdapter::state().durable_log`; live `append_log` already enforces the caller's current canonical durability profile. `RedbDurableStateAdapter::open` validates that profile and namespace descriptor, but `load_state` iterates and owns every persisted legacy log value before checking the profile's record, operation-byte or namespace-byte budgets. A store created under a wider older profile can therefore bypass today's selected budget during reopen and history.

## What changes

- Reuse the **existing current** canonical profile supplied to `open` and validated there: `max_log_records`, `max_operation_bytes`, and `max_namespace_bytes` with the descriptor's existing `quota_bytes`. Do not choose a fallback, global default or new numeric limit.
- Check Redb `ReadableTableMetadata::len()` against the current log-record count **before** starting value iteration. Parse each admitted row through borrowed slices and reject payload/ref metadata or cumulative payload byte violations **before** cloning stored value bytes into `DurableState`.
- Preserve valid Redb log sequence, durability and other table semantics. A denied reopen does not mutate persisted legacy state; an exact-bound reopen and existing native-host journal roundtrip still work.
- Add red-green actual persisted Redb count and byte fixtures, a compiled external public API smoke and a real native journal restart test. Record the isolated owner and disjoint 70-package composition with focused fmt/Clippy/Octet/Cairn evidence.

## Scope and authority

The clean isolated owner `change/preflight-redb-durable-log-restore-20260929` changes the Redb adapter's log **restore** boundary and focused tests, handoff and change-local Cairn. Canonical `DurableStateProfile` already binds the supported count and byte budgets to append; this work uses the caller-selected *current* profile passed to `open` as restore authority. It does not change unrelated ordered/snapshot/effect persistence, journal public signatures, accepted specs, policy, waiver, pin, or full known-failing strict Octet gate.
