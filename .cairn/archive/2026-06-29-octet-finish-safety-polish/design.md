## Context

The accepted Octet remediation model distinguishes configuration-clean evidence from source-remediated-zero evidence. Safety-polish warning families are small enough to handle directly, but they are close to core behavior and need tests rather than mechanical churn.

## Design

### Inventory first

Run the current no-disabled probe and classify remaining safety-polish findings into the active family set: borrowed argument types, platform-dependent casts, unbounded collection growth, boolean naming, unchecked division, ignored results, and nested conditionals.

### Remediation shape

Each remediation slice should use a pure deterministic helper where possible. The imperative shell should only parse inputs, call the helper, and render or persist the existing receipt/output. Prefer named bounds, checked conversions, explicit error propagation, bounded collection construction, and branch flattening with early returns.

### Evidence

Every slice records the before/after no-disabled warning movement for the touched family. If a finding cannot be safely removed in this change, the remediation docs must explain why it remains active rather than treating it as clean.

## Validation

Run the smallest focused test for the touched subsystem before and after the change, then run formatting, Clippy, and a no-disabled Octet probe. Logic-affecting helpers require both positive and negative tests.

## Non-goals

- Do not change public CLI syntax or receipt schemas.
- Do not add broad `allow` attributes or suppressions as burn-down progress.
- Do not combine unrelated import/path/size refactors unless required by the focused safety fix.
