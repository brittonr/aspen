# Sponsored runtime Rust-derived contract registration

- Change: `define-sponsored-runtime-grants`
- Task: register sponsored grant, quota ledger, and usage receipt DTOs as Rust-derived Nickel contract families
- Started: `2026-05-07T01:49:56Z`
- Completed: `2026-05-07T01:52:06Z`

## Implemented

Registered Rust-derived generated contract families for:

- `sponsored-runtime-grant` → `schemas/sponsored-runtime-grant.ncl`
- `sponsored-quota-ledger` → `schemas/sponsored-quota-ledger.ncl`
- `sponsored-usage-receipt` → `schemas/sponsored-usage-receipt.ncl`

Updated `scripts/generate-typed-nickel-contracts.py` to generate those schemas from `crates/aspen-runtime-core/src/lib.rs` DTOs.

Updated typed Nickel registry docs, machine-readable registry, registry checker, and fixture checker coverage.

## Verification

- `python3 scripts/generate-typed-nickel-contracts.py --check`
- `python3 scripts/check-typed-nickel-contract-fixtures.py`
- `python3 scripts/check-typed-nickel-contract-registry.py`

Result: generated contracts fresh for 5 files; fixture checker passed 16 typechecks; registry checker passed 16 families.
