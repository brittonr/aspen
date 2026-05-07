# Sponsored runtime fail-closed admission tests

- Change: `define-sponsored-runtime-grants`
- Task: fail-closed admission tests
- Started: `2026-05-07T01:37:39Z`
- Completed: `2026-05-07T01:39:14Z`

## Implemented

Added pure admission contracts in `aspen-runtime-core`:

- `SponsoredProviderPolicy`
- `SponsoredAdmissionRequest`
- `SponsoredAdmissionError`
- `admit_sponsored_request`

Added tests proving sponsored admission fails closed for:

- missing sponsor/beneficiary/provider principal proof;
- expired grants;
- revoked grants;
- provider-principal rejection;
- unsupported settlement tags;
- secret-bearing settlement references;
- quota exhaustion;
- isolation mismatch;
- workload principal mismatch;
- service principal mismatch.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core --all-targets`

Result: 13 runtime-core tests passed.
