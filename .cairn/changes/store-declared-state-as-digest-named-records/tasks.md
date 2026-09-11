# Tasks: Store declared state as digest-named records

## Consumer

- [ ] [serial] Name the admitted declared-state store that needs multi-writer records, or record the decision to close this package with no implementation. r[molten.durable_state_ports.digest_named_records]

## Layout

- [ ] [serial] Implement the record layout: one file per declared fact, named by the BLAKE3 digest of the canonical record bytes, under a capability-rooted store directory. r[molten.durable_state_ports.digest_named_records]
- [ ] [serial] Implement the reader: enumerate, parse, verify name against content, reject mismatches, and merge as an ordered union. r[molten.durable_state_ports.digest_named_records]

## Validation

- [ ] [parallel] Add positive tests: two writers add records concurrently and both survive; removing one record retracts exactly one fact; duplicate content collapses to one record. r[molten.durable_state_ports.digest_named_records]
- [ ] [parallel] Add negative tests: a malformed record is rejected; a name that does not match its content is rejected; an unwritable or missing store directory fails closed; an over-bound record is rejected. r[molten.durable_state_ports.digest_named_records]
- [ ] [serial] Run focused store tests plus Clippy before and after the change, then the workspace checks the repository requires. r[molten.durable_state_ports.digest_named_records]
- [ ] [serial] Document the layout, the merge rule, and the declared-interest non-claim. r[molten.durable_state_ports.digest_named_records]
