# Sponsored runtime placement constraint

- Change: `define-sponsored-runtime-grants`
- Task: optional runtime-service/job/CI sponsored placement constraint
- Started: `2026-05-07T01:57:42Z`
- Completed: `2026-05-07T01:59:02Z`

## Implemented

Added pure placement-admission types in `aspen-runtime-core`:

- `SponsoredPlacementSurface` for runtime service, job, and CI run placement surfaces;
- `SponsoredPlacementConstraint` carrying whether sponsorship is required and an optional admission request;
- `SponsoredPlacementError`;
- `admit_sponsored_placement`.

The helper is optional when sponsorship is not required, admits only accepted grants when present, and fails closed with `MissingRequiredGrant` when sponsorship is required but no accepted grant is attached.

## Verification

- `rustfmt crates/aspen-runtime-core/src/lib.rs`
- `CARGO_TARGET_DIR=target/agent cargo test -p aspen-runtime-core sponsored_placement_constraint --all-targets`

Result: placement constraint test passed.
