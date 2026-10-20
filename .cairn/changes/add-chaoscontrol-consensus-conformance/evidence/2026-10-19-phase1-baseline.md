# ChaosControl conformance Phase 1 verification

## Scope

The pure Molten-side foundation for the ChaosControl SMR-chain cohort is
implemented at `src/fabric_consistency/chaoscontrol/`:

- `projection.rs` binds a canonical initial-state ref, derives the genesis and
  transition digests with the archived ChaosControl `smr-chain` framing, and
  projects bounded chain observations from contiguous committed applies.
- `ledger.rs` accounts a bounded observation stream, distinguishes lossless
  gaps from safety violations, and retains violations across later reports.
- `identity.rs` preserves one logical operation's client-session and sequence
  identity across acknowledged, definitely rejected, and indefinite outcomes.
- `parts/tests/p000/body.rs` and `p001/body.rs` cover deterministic projection,
  changed order, duplicate apply, rollback, lossless gaps, malformed refs,
  stale generation, operation identity, and indefinite outcomes.

The implementation was already present at HEAD before this verification record;
this record binds the current verification and the remaining Phase 2-5 scope.

## Validation

- `cargo test -p molten --lib chaoscontrol`: 23 tests passed, 0 failed.
  The filter also selected three related world snapshot/benchmark tests.
- The ChaosControl genesis and first-transition known-answer vectors match the
  archived ChaosControl framing.

## Remaining blockers

- Phase 2 KVM and guest tasks depend on ChaosControl's
  `add-protocol-observation-cohorts` contract, which is still active in the
  sibling repository (1/22 tasks), and on an admitted cross-process Molten
  consensus guest.
- No fault campaign, external bundle import, operator workflow, replay class,
  or release claim is established by this phase. The projection and ledger are
  pure in-memory logic only.
