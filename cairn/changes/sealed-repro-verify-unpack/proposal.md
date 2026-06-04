# Change: sealed-repro-verify-unpack

## Why

Sealed repro bundles are first-class pass artifacts, but users need direct lifecycle tooling to verify a bundle and materialize its embedded report without treating `gate check` as the only inspection path.

## What

- Add `molten test repro verify refs.preserves`.
- Emit canonical `<repro-verify-receipt-v1 ...>` receipts for verified sealed report bundles.
- Add `molten test repro unpack refs.preserves --out DIR`.
- Unpack verified sealed bundles into report, suite, gate receipt, verify receipt, refs, summary, and commands files.
- Reject failure, tampered, and unsealed bundles fail-closed with canonical failure artifacts when requested.

## Impact

Sealed bundles gain an explicit verify/unpack lifecycle. Failure bundles remain diagnostic-only and unsealed legacy report bundles remain parseable but cannot satisfy sealed verify/unpack commands.
