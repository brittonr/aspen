# Resource lifecycle count preflight

## Boundaries and decisions

`evaluate_placement` returns `PlacementDecision` rather than `Result`. Test
`request.constraints.len()` against its sibling's source-owned
`MAX_CONSTRAINTS = 64` before allocating diagnostics or examining values.
One-past-limit requests return `decision = "deny"`, no target or quota,
and one `"constraint count N exceeds maximum 64"` diagnostic, matching
the existing placement-fit count error wording. Exactly 64 continue through
the existing preferred/required/anti-affinity and taint logic. Do not
delegate to `evaluate_placement_fit`: it validates unrelated request fields
and would change admitted-input behavior and error precedence.

`validate_reconcile_completion` returns `ReconcileCompletionDecision`.
Check `input.required_effect_intents.len()` against existing
`MAX_EFFECT_INTENTS = 128` at entry, returning `pass = false` with one
`"required effect intent count N exceeds maximum 128"` diagnostic for
oversized input. For admitted inputs, retain the prior stale generation,
missing plan, per-required-effect, and missing status diagnostics in their
original order; cap before reserving/forming any count-dependent
diagnostics. The check is constant time and does not clone caller strings.

## Separate missing contract

The separate `explicit_target_properties` input has neither an owner-selected
cardinality cap nor a byte-size contract; matching all properties and
cloning taint values can still be proportional to that caller-controlled
input. Do not assert this owner fixed that path or invent a property
count/byte policy while repairing the two identified X priorities.

## Verification

Reproduce both one-past-bound cases before changing source. Prove exactly
64 preferred placement constraints still pass and an admitted unsatisfied
required constraint retains its denial; prove exactly 128 receipt-backed
effects pass and admitted stale/missing-effect diagnostics retain order.
Check both one-past bounds deny with a single source-owned count diagnostic,
then exercise a compiled `molten::supplies` consumer, scoped formatting,
Clippy, library Octet and strict Cairn in the isolated owner and 64-package
composition. Do not claim warning-only Octet proves a full strict gate.
