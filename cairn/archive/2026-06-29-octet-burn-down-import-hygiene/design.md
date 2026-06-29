## Context

Import-hygiene warnings are broad and numerous, so they should be handled separately from logic changes. The safest pattern is to reduce local namespace fan-in while preserving module boundaries and public behavior.

## Design

### Hotspot workflow

Use the current no-disabled probe to pick one domain or module family. Replace lists of concrete imports with owner-module namespaces when the resulting call sites stay readable. Use narrow aliases only when full owner paths would create path-shape churn or obscure domain intent.

### Behavior preservation

Import cleanup must not alter command parsing, receipt rendering, canonical Preserves values, or fail-closed validation. Any required functional change must move to a different Cairn package or receive explicit tests in this package.

### Evidence

Record before/after `non_trait_imports` counts for the touched domain and the total probe. Keep remaining import warnings visible until the family is clean or explicitly scoped.

## Validation

Run focused tests for the touched domain, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and a no-disabled Octet probe.

## Non-goals

- Do not introduce public module renames only for import counts.
- Do not mix substantial path-shape or size-shape decomposition unless needed for compilation.
- Do not suppress import warnings with allow attributes as burn-down progress.
