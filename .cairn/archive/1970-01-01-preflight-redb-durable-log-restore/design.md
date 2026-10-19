# Redb durable log recovery budget preflight

## Authority and available bounded reader

`RedbDurableStateAdapter::open(root_path, profile: CanonicalDurableProfile, descriptor)` already receives the **current caller-selected** canonical profile and validates its `DurableStateProfile` and descriptor before database access. The accepted durable-port contract binds profile, namespace and quota, and the core `append_log` checks `profile.max_log_records`, `profile.max_operation_bytes` and the smaller of `profile.max_namespace_bytes` / `descriptor.quota_bytes` for each durable payload. `validate_durable_profile` admits positive owner-selected values; it does **not** impose a universal numeric cap. The same current profile is a per-instance restore budget, not a claim that every valid profile is small. Redb 4.1 `ReadableTableMetadata::len()` returns a `Result<u64>` count without decoding or owning record values; `ReadableTable::iter()` provides borrowed record guards. No new hardcoded persistence budget is required.

## Narrow source change

Pass `&profile.profile` into `load_state` from the validated `open`. Once the log table is open but before `table.iter`, compare `table.len()?` to `max_log_records` and return the existing `OperationLimitExceeded` under adapter restore denial when exceeded. For each admitted row, use the existing `ByteCursor` to borrow the encoded value-ref and payload without a `Vec` allocation. Bound borrowed value-ref bytes by the **existing** `MAX_DURABILITY_TEXT_BYTES` ceiling to avoid materializing enormous malformed metadata. Check payload length against `max_operation_bytes` and checked cumulative payload length against `min(profile.max_namespace_bytes, descriptor.quota_bytes)` before creating an owned `LogRecord`; return existing `ByteLimitExceeded` or `NamespaceQuotaExceeded` issues. Advance durable-byte accounting once and push only admitted records. Keep ordered/snapshot/effect loading and existing valid-log encoding, durability and sequence semantics unchanged.

The profile and descriptor are the trusted *current* caller authority at this API; historical persisted rows need not have been written under those same budgets. A caller that selects an impractically large positive profile still controls that budget, as before; no silent default can repair operator configuration. Oversized legacy log tables deny before loading row values; malformed/oversized row payloads deny before copies. Failed open performs no Redb writes beyond the existing table-initialization path.

## Regression and smoke

Persist two and three real Redb records under an earlier broader profile, reopen with a current two-record profile: exactly two restore unchanged; the third denies at the current count before any log decoding and leaves the legacy store intact. Likewise persisted eight- and nine-byte values test the current per-operation byte budget; cumulative 16/17-byte fixtures independently exercise a current profile namespace limit under a wider descriptor quota and a descriptor quota under a wider current profile. The count and operation-byte overbound branches failed before the fix. A disposable external `molten::fabric_durability::RedbDurableStateAdapter::open` consumer proves the public API, and the native-host journal restart test now observes restored filtered history, not just the latest record.

## Validation constraints

Run focused red-green persisted Redb tests and native-host restart, owner/composition fmt/Clippy, scoped nonincremental library Octet and strict Cairn. No accepted spec, policy, waiver, pin, new arbitrary numeric limit, unrelated journal API or full known-failing strict Octet retry is in scope.
