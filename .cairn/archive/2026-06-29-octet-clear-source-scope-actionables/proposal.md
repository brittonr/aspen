## Why

Source-scope classification now distinguishes generated/remapped external rows from Molten-owned actionable rows. The remaining Molten-owned rows must be resolved or explicitly scoped before Molten can narrow the source-scope caveat without hiding source-gate debt.

This change clears the actionable source-scope rows while preserving the fail-closed treatment for unknown or unclassified findings.

## What Changes

- Re-run source-scope classification from the current no-disabled probe.
- Remediate the remaining Molten-owned actionable rows, currently centered on dogfood and node authority CLI surfaces.
- Preserve deterministic classification evidence for external/remapped findings.
- Keep unknown rows blocked until they are confidently classified or fixed.

## Impact

This reduces source-scope/tooling caveats without weakening Octet coverage. Public command syntax, receipt labels, canonical Preserves output, and source-gate failure semantics remain unchanged.
