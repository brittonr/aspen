## Why

Molten still carries a source-remediated-zero caveat for lower-count Octet warning families that can indicate correctness, portability, or clarity issues. The previous safety-polish slice proved the workflow, but the remaining warnings still need focused remediation evidence before the category can stop being an active caveat.

This change finishes the safety-polish burn-down without hiding behavior changes inside broad source-shape refactors.

## What Changes

- Refresh the no-disabled Octet probe and inventory remaining safety-polish findings by lint family, file, and subsystem.
- Remediate safety-polish hotspots with explicit result handling, bounded collection construction, checked conversions/arithmetic, clear boolean names, and simpler control flow.
- Add positive and negative tests for every logic-affecting remediation.
- Record before/after warning movement and keep any intentionally deferred finding visible in remediation evidence.

## Impact

This is correctness-oriented cleanup. It should not change public CLI syntax, receipt schemas, canonical Preserves values, or source-gate semantics except by reducing or explicitly scoping safety-polish warnings.
