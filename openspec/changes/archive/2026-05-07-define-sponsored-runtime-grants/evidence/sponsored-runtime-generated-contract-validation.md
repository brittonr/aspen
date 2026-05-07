# Sponsored runtime generated-contract freshness and receipt validation

- Change: `define-sponsored-runtime-grants`
- Task: generated-contract freshness, Rust serialization round-trip, and Nickel receipt fixtures
- Started: `2026-05-07T01:53:16Z`
- Completed: `2026-05-07T01:56:48Z`

## Implemented

Updated generated Nickel contract tooling to:

- export generated contracts through non-recursive aliases;
- honor Rust `serde(rename_all = "kebab-case")` annotations;
- map unsigned Rust numeric fields to a non-negative Nickel contract.

Added runtime-core serialization round-trip coverage for:

- `SponsoredRuntimeGrant`
- `SponsoredQuotaLedger`
- `SponsoredUsageReceipt`

Expanded typed Nickel fixtures for generated usage receipts:

- valid receipt fixture;
- malformed outcome rejection;
- missing required field rejection;
- out-of-range negative unsigned resource rejection;
- unknown field rejection;
- secret-bearing redacted field rejection.

## Verification

- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core sponsored_rust_derived_dtos_serialize_roundtrip --all-targets`
- `python3 scripts/generate-typed-nickel-contracts.py --check`
- `python3 scripts/check-typed-nickel-contract-fixtures.py`
- `python3 scripts/check-typed-nickel-contract-registry.py`

Result: Rust round-trip test passed; generated contracts fresh for 5 files; fixture checker passed 16 typechecks, 10 positive exports, and 12 negative exports; registry checker passed 16 families.
