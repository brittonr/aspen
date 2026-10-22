# Tasks: Store declared state as digest-named records

## Decision

- [x] [serial] Name the admitted declared-state store that needs multi-writer records, or record the decision to close this package with no implementation. r[molten.durable_state_ports.digest_named_records]
      The decision is recorded: no admitted store of declared-interest records needs multi-writer records today, so the
      package closes with the decision and no implementation. The consumer census is in
      `evidence/2026-10-21-decision.md`.

## Layout

- [x] [serial] Record the layout that applies when a consumer appears: one file per declared fact, named by the BLAKE3 digest of the canonical record bytes, under a capability-rooted store directory. r[molten.durable_state_ports.digest_named_records]
      No admitted store needs the layout, so no code implements it. The design in this package applies unchanged when
      a consumer appears.
- [x] [serial] Record the reader rule that applies when a consumer appears: enumerate, parse, verify the name against the content, reject mismatches, and merge as an ordered union. r[molten.durable_state_ports.digest_named_records]
      No admitted store needs the layout, so no code implements it. The rule is stated with the layout.

## Validation

- [x] [parallel] Record the positive fixtures the layout carries: two writers add records concurrently and both survive; removing one record retracts exactly one fact; duplicate content collapses to one record. r[molten.durable_state_ports.digest_named_records]
      No store implements the layout, so there is no behaviour to test. The scenarios stay in the accepted requirement
      as the behaviour a future consumer must satisfy.
- [x] [parallel] Record the negative fixtures the layout carries: a malformed record is rejected; a name that does not match its content is rejected; an unwritable or missing store directory fails closed; an over-bound record is rejected. r[molten.durable_state_ports.digest_named_records]
      No store implements the layout, so there is no behaviour to test. The scenarios stay in the accepted requirement
      as the behaviour a future consumer must satisfy.
- [x] [serial] Run the repository gates that apply to a decision-only package. r[molten.durable_state_ports.digest_named_records]
      Nothing changed in the code, so no store test or Clippy run applies to this package. The repository gates for the
      change itself ran on the decision record: formatting, Clippy, Octet, the strict Cairn validation, and the
      inherited-debt guard.
- [x] [serial] Document the layout, the merge rule, and the declared-interest non-claim. r[molten.durable_state_ports.digest_named_records]
      The decision record carries the consumer census, the layout and merge rule, the decision, and the non-claims.
