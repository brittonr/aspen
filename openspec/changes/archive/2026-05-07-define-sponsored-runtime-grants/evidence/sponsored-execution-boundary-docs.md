# Sponsored execution boundary documentation

- Change: `define-sponsored-runtime-grants`
- Task: document sponsored execution boundary
- Started: `2026-05-07T02:05:05Z`
- Completed: `2026-05-07T02:06:33Z`

## Implemented

Added `docs/runtime-sponsorship.md`, covering:

- Aspen-owned resource authority, admission, quota, ledger, and receipt boundaries;
- external ownership of bilateral settlement, currency, payment, tax, and provider accounting;
- Nickel-authored policy contracts versus Rust-derived runtime DTO contracts;
- fail-closed sponsored admission behavior;
- signed/redacted usage receipt outcomes and secret-handling rules.

Updated `docs/runtime-applications.md` to link the sponsorship boundary from the runtime implementation slice and added the runtime-core focused test to the typed Nickel contract registry verification block.

## Verification

- `git diff --check`

Result: whitespace check passed during final staging for this task.
