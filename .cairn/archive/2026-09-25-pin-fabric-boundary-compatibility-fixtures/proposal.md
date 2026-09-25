# Proposal: Pin the fabric boundary compatibility fixtures

## Why

The accepted requirement `molten.modularity.fabric_boundary.compatibility` (`.cairn/specs/project/spec.md:1313-1322`)
says that boundary migration MUST preserve canonical Preserves values, transition refs, receipt meanings, and live or
simulation behavior. Its scenario id `molten.modularity.fabric_boundary.compatibility.fixtures` compares the migrated
path against an accepted pre-migration transition and receipt fixture.

The introducing change, archive `2026-08-24-separate-fabric-ports-and-adapters`, recorded only a one-time rerun of
the focused suites before and after the migration commit `3de348149`. It pinned no fixture and added no permanent
test. Both ids therefore have no evidence marker. They are two of the three `unexpected_missing` ids that fail the
`inherited-tracey-debt` guard.

## What Changes

- Generate canonical fixtures once, at the pre-migration commit `3de348149^`, with a throwaway generator. The
  fixtures cover the membership profile and view, a role-assignment transition, the time profile, the transport
  profile and a registration transition, and the durable-state profile and an append transition. The generator
  source, the commit, and the inputs are recorded in `evidence/`.
- Check in the fixtures (`tests/fixtures/fabric-boundary/*.preserves` plus `refs.tsv`) and the shared explicit inputs
  (`tests/fabricboundarycompat/{inputs,ports,cases}.rs`).
- Add `tests/fabricboundarycompat.rs` (Result-returning tests, no `expect` or `unwrap`, and no lint allows):
  - Positive: equal explicit inputs give byte-equal canonical values and equal refs against the fixtures.
  - Negative: a one-field input mutation changes each ref, and a tampered or truncated fixture fails strict decode.
- Add impl markers for `molten.modularity.fabric_boundary.compatibility` on the eight covered canonical projections, and
  verify markers for both ids on the tests.

Accepted specifications do not change. The change implements accepted requirement text as written, so the
`no-spec-delta` profile applies. The one behavior effect is new tests and fixtures; no production code path
changes apart from comment markers.

## Impact

- **Files**: `tests/fabricboundarycompat.rs`, `tests/fabricboundarycompat/{inputs,ports,cases}.rs`,
  `tests/fixtures/fabric-boundary/*`, and comment-only marker lines in `src/fabric_{membership,time,transport,durability}/canonical.rs`.
- **Testing**: the focused test, pinned Octet root and lib with a zero per-lint delta, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, and `nix build .#checks.x86_64-linux.inherited-tracey-debt` (unexpected_missing should
  drop from 3 to 1).

## Out of Scope

- `molten.authority.nominal_references.octet.guard` (an owner decision between an Octet bump and a reword).
- Rewording the scenario's "pre-migration" wording. Generation at the pre-migration commit makes a reword
  unnecessary.
- Fixtures for projections outside the eight covered here (failure observations, placement, time events,
  recovery decisions).
