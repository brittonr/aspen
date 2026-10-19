# Preflight public resource lifecycle growth

## Why

The public `evaluate_placement` entry point processes caller-built constraints
without its sibling `evaluate_placement_fit` function's existing
`MAX_CONSTRAINTS = 64` guard. The public
`validate_reconcile_completion` entry point formats one diagnostic for each
missing required effect intent, although `MAX_EFFECT_INTENTS = 128` is
defined but unused at that boundary. Both inputs can bypass the bounded
resource lifecycle intake used by their owners.

## What Changes

- Deny more than 64 placement constraints before allocating diagnostic
  capacity, cloning taint values, or formatting constraint failures.
- Deny more than 128 required effect intents with a single completion
  decision diagnostic before any per-effect formatting; preserve the prior
  generation, plan, effect, and status diagnostic order for admitted input.
- Exercise both exact-bound and one-past-bound public API behavior with
  focused regressions and a separately compiled consumer.

## Impact

The canonical owner changes the resource placement and completion helper
bodies, focused tests, this change-local Cairn and the Octet handoff. The
current 64-package composition receives their equivalent bodies and tests
without changing other owners. Only tests call these public helpers within
this repository, so no CLI route is asserted. Explicit target-property
cardinality/byte size remains a separate missing owner contract: this change
does not invent a cap for it. No public signatures, policy, waiver, pin,
accepted spec, or full strict-ci gate change.
