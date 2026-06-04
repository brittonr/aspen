# Change: sealed-repro-redaction-preflight

## Why

Sealed repro bundles are portable pass artifacts. Before they are exported, verified, unpacked, or gated, they need a fail-closed confidentiality rail so pass evidence cannot become an accidental secret exfiltration mechanism.

## What

- Add canonical redaction policy evidence to sealed report repro bundles.
- Add `<redaction-gate-v1 ...>` evidence bound to report and suite refs.
- Fail closed on sensitive Preserves record markers such as `<secret ...>`, `<confidential ...>`, `<credential ...>`, `<private ...>`, and unvalidated `<encrypted-ref ...>`.
- Require sealed bundle parse/gate/verify/unpack to recompute redaction evidence.
- Reject unsealed report bundles as pass evidence because they lack redaction preflight.

## Impact

Normal sealed two-actor bundles continue to pass. Suites/reports containing sensitive marker records can still run and validate locally, but sealed repro export refuses them until explicit redaction/encryption support exists.
