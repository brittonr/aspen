# Coordination Tiger Style Phase 2 Notes

## Baseline Inventory

Baseline command:

```bash
scripts/tigerstyle-check.sh -p aspen-coordination -- --lib
```

Baseline findings from `evidence/aspen-coordination-baseline.txt`:

- `ambient_clock`: 21 warnings
- `compound_assertion`: 2 warnings
- `no_panic`: 4 warnings

### Ambient-clock findings by file

Boundary-owned timeout or measurement reads:

- `crates/aspen-coordination/src/barrier.rs` — 4
- `crates/aspen-coordination/src/lock.rs` — 2
- `crates/aspen-coordination/src/queue/dequeue.rs` — 2
- `crates/aspen-coordination/src/rate_limiter.rs` — 2
- `crates/aspen-coordination/src/rwlock/acquisition.rs` — 4
- `crates/aspen-coordination/src/semaphore.rs` — 2
- `crates/aspen-coordination/src/worker_strategies.rs` — 5

Disposition: isolate these clock reads into one explicit shell boundary helper,
`crates/aspen-coordination/src/runtime_clock.rs`, instead of scattering raw
`Instant::now()` calls across coordination code.

### Compound-assertion findings

- `crates/aspen-coordination/src/verified/barrier.rs` — 2 assertions in
  `prop_initial_phase_matches_ready`

Disposition: split them into direct single-condition assertions.

### Panic-family findings

- `crates/aspen-coordination/src/barrier.rs` — 1 `unreachable!()` in the create
  path for `EnterResult::InLeavingPhase`
- `crates/aspen-coordination/src/lockset.rs` — 1 test panic for expected
  contention
- `crates/aspen-coordination/src/rate_limiter.rs` — 2 test/helper panics

Disposition:

- replace the barrier `unreachable!()` with `debug_assert!()` plus retry
- replace test panic branches with `expect_err(...)` or explicit error returns

## Cleanup Applied

- added `crates/aspen-coordination/src/runtime_clock.rs` to own timeout and
  measurement boundaries
- rewired barrier, lock, queue dequeue, rate limiter, rwlock acquisition,
  semaphore, and worker strategies to use that helper
- replaced the barrier create-path `unreachable!()` with a defensive
  `debug_assert!()` path
- removed panic-based test/helper branches in `lockset.rs` and `rate_limiter.rs`
- simplified the verified barrier property assertions to single-condition
  checks
- promoted `aspen-coordination` into the default package scope of
  `scripts/tigerstyle-check.sh`

## Post-cleanup Result

Targeted rerun command:

```bash
scripts/tigerstyle-check.sh -p aspen-coordination -- --lib
```

Result from `evidence/aspen-coordination-after.txt`:

- no coordination starter-lint findings remain
- only Dylint/toolchain infrastructure warnings remain (`resolver` and
  `patch ... was not used in the crate graph`)

Default pilot command:

```bash
scripts/tigerstyle-check.sh
```

Result from `evidence/aspen-default-pilot.txt`:

- default pilot now includes `aspen-coordination`
- starter-lint run completes with the expanded default scope
