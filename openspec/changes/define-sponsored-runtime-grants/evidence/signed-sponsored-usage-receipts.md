# Signed sponsored usage receipts

- Change: `define-sponsored-runtime-grants`
- Task: signed, redacted usage receipts for sponsored runtime paths
- Started: `2026-05-07T01:59:39Z`
- Completed: `2026-05-07T02:04:20Z`

## Implemented

Added pure receipt helpers in `aspen-runtime-core`:

- `SignedSponsoredUsageReceipt` with redacted `signature_ref`;
- `SponsoredUsageReceiptInput`;
- `signed_sponsored_usage_receipt`.

Tests cover all required sponsored execution receipt paths: `started`, `reserved`, `consumed`, `completed`, `failed`, and `revocation-denied`. Each generated receipt keeps settlement refs, diagnostics, and signature refs redacted. A negative assertion proves secret-bearing signature refs are detected.

Updated Rust-derived Nickel generation to include `SignedSponsoredUsageReceipt` in `schemas/sponsored-usage-receipt.ncl`.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `python3 scripts/generate-typed-nickel-contracts.py --write`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core signed_sponsored_usage_receipts --all-targets`
- `python3 scripts/generate-typed-nickel-contracts.py --check`
- `python3 scripts/check-typed-nickel-contract-fixtures.py`

Result: all commands passed.
