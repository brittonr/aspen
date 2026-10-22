# Tasks: Fence runtime-local work across restarts

## Conformance first

- [x] [serial] Add the restart-fencing conformance trace (start A, create pending runtime-local work, restart without implementation change, deliver A's delayed timer, callback completion, and failure notification) and record whether existing lifecycle-sequence propagation rejects every delivery. r[molten.system_extension.incarnation_fencing]
- [x] [parallel] Retain the trace as a permanent regression with the observed verdict encoded in its assertions. r[molten.system_extension.incarnation_fencing]

## Identity model

- [x] [serial] If (and only if) the conformance trace found an accepted stale delivery, add the runtime incarnation to the lifecycle state, increment it on every restart-entry transition, and bind `CallbackEvent` and `TypedEffectRequest` to generation plus incarnation with combined validation and a typed stale-incarnation issue. r[molten.system_extension.incarnation_fencing]
- [x] [parallel] Add negative fixtures for stale-incarnation timer delivery, stale callback completion, stale failure notification, and future-incarnation rejection. r[molten.system_extension.incarnation_fencing]
- [x] [parallel] Update every `CallbackEvent` and `TypedEffectRequest` constructor in fixtures and tests to carry the incarnation when the field is added. r[molten.system_extension.incarnation_fencing]

## Durable readmission

- [x] [serial] Specify and implement explicit readmission of durable logical work after restart: fresh delivery claim bound to the current incarnation, rejection of runtime-local completions whose incarnation predates the claim. r[molten.system_extension.durable_readmission]
- [x] [parallel] Add the mixed-up negative cases: an old callback presented as durable work, and durable work treated as a live callback, both rejected with typed issues. r[molten.system_extension.durable_readmission]

## Validation and closeout

- [x] [serial] Run focused system-extension and addressable-actor tests, formatting, Clippy, Octet, Cairn validation, and the proposal, design, and tasks gates. r[molten.system_extension.incarnation_fencing] r[molten.system_extension.durable_readmission]
- [x] [serial] Retain the delivery-claim, placement-fencing, and no-whole-system-correctness non-claims before sync or archive. r[molten.system_extension.incarnation_fencing] r[molten.system_extension.durable_readmission]
