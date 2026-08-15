# Design: sealed repro verify/unpack

## Verify

`molten test repro verify refs.preserves` parses a sealed report repro bundle, rejects failure bundles and unsealed legacy bundles, validates the embedded report and gate receipt, recomputes deterministic replay, and recomputes the exact embedded report gate receipt. On success it emits `<repro-verify-receipt-v1 "molten.harness.repro-verify-receipt.v1" ...>` with bundle, report, suite, embedded gate receipt refs, and pass checks.

Failures are emitted as canonical `<harness-failure-v1 ...>` artifacts when `--failure-out` is supplied.

## Unpack

`molten test repro unpack refs.preserves --out DIR` first runs the same sealed verification path. If verification passes it writes:

- `refs.preserves` — the original sealed bundle;
- `report.preserves` — embedded report;
- `suite.preserves` — embedded suite;
- `gate-receipt.preserves` — embedded report gate receipt;
- `verify-receipt.preserves` — verification receipt;
- `summary.txt` — bundle summary;
- `commands.txt` — local validation/replay/gate/verify/unpack commands.

Unpack refuses failure bundles, unsealed bundles, and tampered bundles fail-closed.

## Compatibility

Existing sealed bundle gate checks continue to work. Older unsealed report repro bundles remain parseable as compatibility artifacts but do not satisfy the new sealed verify/unpack lifecycle.
