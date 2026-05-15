## Phase 1: Storage inventory and model

- [ ] [serial] Inventory current KV/table/blob/secret call sites that hand-roll schema, key, or receipt behavior and pick the first implementation slice.
- [ ] [depends:inventory] Define typed resource handle fields, schema/type hash strategy, codec versioning, and backend mapping for `Cell<T>` and `OrderedTable<K,V>`.

## Phase 2: Facade implementation

- [ ] [depends:model] Implement typed `Cell<T>` create/open/read/write validation with schema mismatch errors.
- [ ] [depends:cell] Implement typed `OrderedTable<K,V>` bounded key/range operations and reject unbounded scans.
- [ ] [depends:ordered-table] Implement transaction/batch-read facade behavior for the selected backend boundary.

## Phase 3: Receipts and validation

- [ ] [depends:facade] Add bounded, redacted typed-storage receipts and diagnostics.
- [ ] [depends:receipts] Add positive tests plus negative tests for schema mismatch, unbounded range, malformed codec metadata, and secret redaction.
- [ ] [depends:tests] Update docs and run focused storage tests, strict OpenSpec validation, and `git diff --check`.
