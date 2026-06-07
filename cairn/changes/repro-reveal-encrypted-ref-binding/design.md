## Context

`redacted-repro-export-profiles` introduced `encrypted-private` bundles that replace sensitive records with validated `<encrypted-ref-v1 ...>` placeholders. Unpack is intentionally private and requires passing reveal receipts, but matching reveal receipts via `secret_ref` or `commitment_ref` leaves room for stale or replayed receipts to appear applicable.

## Goals

- Bind reveal receipts to the exact encrypted-ref id they authorize.
- Treat the encrypted-ref binding as the only authorization key for encrypted-private repro unpack.
- Fail closed on missing bindings, denied receipts, stale bindings, unrelated encrypted refs, and partial coverage.
- Keep legacy/generic reveal receipts parseable outside repro unpack.
- Keep encrypted-private bundles `requires-reveal`, not pass-gate evidence.

## Non-Goals

- No new encryption primitive or plaintext storage format.
- No gate-preserving transform policy.
- No authority grant from reveal receipts beyond private repro unpack materialization.
- No network transport or key-management changes.

## Receipt Shape

Reveal receipts continue to use `reveal-receipt-v1` and gain an `encrypted-ref` field. The field is optional at parse time for legacy/generic confidentiality workflows, but repro unpack requires `Some(<encrypted-ref>)` and an `encrypted-ref-bound` check.

## Validation

`molten test repro unpack` constructs the expected encrypted-ref set from the parsed bundle. Each supplied passing reveal receipt must name exactly one encrypted ref in that set. Extra stale refs fail closed, and every expected ref must be covered before any files are materialized.

## Evidence Boundary

The direct binding is reveal/unpack evidence only. It does not satisfy gate-preserving repro verification, source gates, provenance admission, policy authority, resource grants, transport trust, or execution trust.
