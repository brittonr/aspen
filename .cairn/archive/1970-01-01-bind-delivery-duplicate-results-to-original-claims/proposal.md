# Bind delivery duplicate results to original claims

## Why

F08 demonstrates a wrong duplicate response at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

The executed counterexample is `audit_duplicate_claim_does_not_return_another_consumers_token`. Queue `queue:delivery` uses service generation 7, consistency epoch 11, and engine epoch 13. The fixture enqueues item reference `a`, then consumer A claims it at logical tick 100 with operation reference `2`. References use the fixture BLAKE3 prefix and repeated hexadecimal digit encoding.

The visibility duration is 10 ticks and fixed retry delay is 5 ticks. An admitted expiry request with operation reference `3` expires A's token at tick 110. Consumer B claims the item at tick 115 with operation reference `4`. A repeats the exact original request at tick 100.

Expected: duplicate replay returns A's original result or no token if the original token is unavailable. Actual: `DuplicateReplay` identifies A's operation but includes B's current token.

The audit executed the core sequence. Static service review shows that duplicate transitions return without a new commit. Separate completion-owner and delegated-authority checks remain in force. This result does not establish an authority bypass.

## Proposed scope

Bind duplicate token reconstruction to the saved operation token, not the current item occupant. Preserve bounded history, exact-operation conflict checks, no-commit replay, and independent completion admission.

The current consumer is the coordination-delivery service. Molten coordination-delivery maintainers own the change. The durable capability is operation-bound duplicate responses with normal repository regression tests.

## Evidence and limits

The audit reports 359 passing core baseline tests. Its separate harness reports 41 passing controls and eight failing regression assertions. F08 is executed core evidence. Six other audit findings have static evidence only.

This package records the trigger without ignored scratch dependencies. No commands or gates ran during planning. This proposal grants no implementation permission. It does not prove authority bypass, exactly-once effects, global ordering, or release eligibility.
