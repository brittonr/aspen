# Tasks: Own remote assertions per session

## Ownership rule

- [ ] [serial] Record the current owner assignment for delivered remote assertions and the session state available at apply time. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Apply remote assertions with the receiving session scope as owner and record the owning session ref in the applied assertion record. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Deny an envelope whose declared owner is unknown or belongs to a closed session, before any staging. r[molten.runtime_spine.remote_assertion_ownership]

## Cleanup

- [ ] [serial] Run owner-scope cleanup for the session scope on session close and disconnect, so assertions, observers, and messages retract together. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Keep replayed deliveries for a closed session diagnostic, with no state resurrection. r[molten.runtime_spine.remote_assertion_ownership]

## Validation

- [ ] [parallel] Add positive tests: a delivered assertion is visible while the session lives, disconnect retracts it, and an observer receives the retraction. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [parallel] Add negative tests: a late delivery to a closed session denies before staging, an unknown declared owner denies, and a replay for a closed session does not resurrect assertions. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Re-record any recorded delivery fixtures that reuse a closed session identity and list the moved fixtures. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for remote runtime edits. r[molten.runtime_spine.remote_assertion_ownership]
- [ ] [serial] Document the ownership rule, the reconnect rule, and the no-delivery-completeness non-claim. r[molten.runtime_spine.remote_assertion_ownership]
