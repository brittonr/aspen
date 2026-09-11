# Tasks: Coalesce equal-assertion notifications

## Behavior

- [ ] [serial] Record a baseline of the current notification behavior for duplicate assertions, duplicate owners, and owner-scope cleanup. r[molten.runtime_spine.assertion_notification_coalescing]
- [ ] [serial] Add the pure visibility decision for `Assert`, `Retract`, and `Observe` in `src/runtime/dataspace/state.rs`. r[molten.runtime_spine.assertion_notification_coalescing]

## Validation

- [ ] [parallel] Add positive tests: two owners assert one value and the observer sees one notification; one owner retracts and the value stays visible with no notification; the last owner retracts and one retraction arrives. r[molten.runtime_spine.assertion_notification_coalescing]
- [ ] [parallel] Add negative tests: distinct values notify separately; cleanup of one of two owners keeps the value visible; a retraction of a value that no owner holds fails closed before commit. r[molten.runtime_spine.assertion_notification_coalescing]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for runtime edits. r[molten.runtime_spine.assertion_notification_coalescing]
- [ ] [serial] Record the rule and its non-claims in the `docs/architecture.md` dataspace section. r[molten.runtime_spine.assertion_notification_coalescing]
