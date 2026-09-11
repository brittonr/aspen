# Tasks: Enforce introduction before reference

## Baseline

- [ ] [serial] Inventory the reference-bearing fields on the remote dataspace path and the session bootstrap fields that establish known references. r[molten.runtime_spine.reference_introduction_rule]

## Rule

- [ ] [serial] Add the pure derivation of the introduced reference set from session bootstrap refs and live session assertions. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [serial] Add receiver-side denial before delivery for a message that carries an unintroduced reference, with the unknown ref named in the denial evidence. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [serial] Add sender-side refusal in the envelope build step for an unintroduced reference. r[molten.runtime_spine.reference_introduction_rule]

## Validation

- [ ] [parallel] Add positive tests: an assertion introduces a ref and a later message that carries it is admitted and applied through the turn boundary. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [parallel] Add negative tests: an unknown ref denies before delivery with no partial state, a ref whose introducing assertion retracted denies again, sender-side build refuses, and session close empties the introduced set. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [serial] Re-record any recorded delivery fixtures that carry unintroduced references and list the moved fixtures. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [serial] Run `cargo test -p molten` and focused Clippy before and after the change, then the workspace checks the repository requires for remote runtime edits. r[molten.runtime_spine.reference_introduction_rule]
- [ ] [serial] Document the rule and its non-claims in the remote dataspace and reference-harness docs. r[molten.runtime_spine.reference_introduction_rule]
