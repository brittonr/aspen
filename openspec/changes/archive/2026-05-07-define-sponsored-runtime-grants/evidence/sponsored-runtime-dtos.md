# Sponsored runtime grant DTOs

- Change: `define-sponsored-runtime-grants`
- Task: portable Rust-owned DTOs
- Started: `2026-05-07T01:34:06Z`
- Completed: `2026-05-07T01:35:20Z`

## Implemented

Added pure, serializable sponsorship DTOs to `crates/aspen-runtime-core/src/lib.rs`:

- `SponsoredPrincipalRef`, `SponsoredPrincipalRole`, node/plugin principal refs;
- `SponsoredRuntimeGrant`, `SponsoredGrantScope`, `SponsoredRevocationRef`;
- `SponsoredResourceLimits`, `SponsoredQuotaLedger`, `SponsoredQuotaReservation`, `SponsoredQuotaConsumption`;
- `SponsoredSettlementReference` with redacted/opaque value boundary;
- `SponsoredUsageReceipt` and `SponsoredReceiptOutcome`.

The model remains portable: no process/network/filesystem I/O, no cryptographic verification, no payment-rail interpretation, and no raw credential storage requirement.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo check -p aspen-runtime-core --all-targets`

Both passed.
