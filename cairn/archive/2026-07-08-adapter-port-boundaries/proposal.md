## Why

Molten's runtime law says effects happen only through admitted adapters, but implementation boundaries can blur when pure decisions call directly into filesystem, ledger, transport, clock, or executor helpers. That makes replay harder to prove and makes domains harder to extract.

## What Changes

- Define explicit port interfaces or input/output records for storage, transport, execution, policy, clock/seed, and effect-log interactions.
- Move pure admission and planning logic behind deterministic functions that return planned adapter operations.
- Keep adapter implementations in thin shells that perform IO only after admission.
- Add positive and negative tests proving denied plans do not perform side effects.

## Impact

This change clarifies the functional-core / imperative-shell boundary for runtime and node behavior. It should strengthen deterministic replay evidence and reduce coupling between domain logic and adapter implementations.
