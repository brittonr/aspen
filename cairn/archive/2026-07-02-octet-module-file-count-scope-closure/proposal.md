## Why

Molten's Octet gate has been reduced to one configured disabled lint family: `module_file_count`. The previous source-scope burn-down classified residual no-disabled source-scope findings as remapped/generated external rows, but the checked-in gate still carries the disabled-lint caveat.

Release and admission evidence should not keep a broad disabled family once the remaining residue can be scoped away without hiding Molten-owned source. This change closes that gap by proving the current residue, removing the configured disable only if the source gate stays fail-closed for Molten-owned and unknown findings, and refreshing the documented evidence.

## What Changes

- Capture a fresh no-disabled Octet probe for the current source tree.
- Narrow source-gate scope or Octet configuration so `module_file_count` is not globally disabled for Molten evidence.
- Preserve fail-closed behavior for Molten-owned source and unknown source-scope findings.
- Update Octet evidence documentation after the caveat is removed or converted into an explicit external-scope decision.

## Impact

This is source-gate evidence cleanup, not a relaxation. The final state removes the last broad disabled-lint family from `dylint.toml` only with focused validation showing no Molten-owned source findings are hidden and strict source-gate receipts still bind object-corpus/fingerprint evidence.
