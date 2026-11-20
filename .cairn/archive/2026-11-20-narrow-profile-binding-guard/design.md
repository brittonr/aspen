## Context

The audit normalizer refuses to evaluate a closure when the `molten` resolve
node carries any root feature, because the reviewed release profile enables no
optional features. `cargo metadata` reports features enabled by *any* edge,
including the crate's own dev-dependency, so the self dev-dependency that enables
`test-fixtures` for the integration tests makes the rule unsatisfiable.

## Decisions

### 1. Review the exact dev-only set instead of disabling the check

**Choice:** accept a root feature set of exactly `{"test-fixtures"}`; deny every
other set.

**Rationale:** an allowlist keeps the guard closed. A new default feature, an
optional-feature union, or a second dev feature still denies and still needs
review; the only thing removed is the permanent false positive.

### 2. Keep the binding-level check untouched

**Choice:** the `binding.features == []` requirement stays.

**Rationale:** that check binds what the *capture* claims to scan; it is
independent of what `cargo metadata` reports for the workspace root, and it is
the control that stops an optional profile masquerading as the default.

## Risks / Trade-offs

- If `test-fixtures` ever stops being enabled by the self dev-dependency, the
  allowlist becomes stale rather than wrong: the check still denies any other
  feature set, and the control fixes the expected value in place.
