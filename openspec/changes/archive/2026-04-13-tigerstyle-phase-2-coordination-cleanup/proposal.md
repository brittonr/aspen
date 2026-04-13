# Promote Aspen Coordination Into Tigerstyle Phase 2

## Why

`crates/aspen-coordination` is still too noisy for Aspen's phase-1 tigerstyle
pilot. A focused lib-only run via `scripts/tigerstyle-check.sh -p
aspen-coordination -- --lib` produced 21 `ambient_clock` warnings, 2
`compound_assertion` warnings, and 4 `no_panic` warnings. That is real signal,
but it is enough noise that the coordination crate was excluded from the
default pilot package set.

## What Changes

- classify the current coordination findings into boundary-owned clock reads,
  required refactors, and assertion cleanup
- replace or isolate production `panic!` / `unreachable!` sites that block the
  rollout
- split the compound assertions in `src/verified/barrier.rs`
- rerun the tigerstyle coordination check and add the crate to the default
  pilot once the finding set is acceptable

## Evidence

- `barrier.rs`, `lock.rs`, `queue/dequeue.rs`, `rate_limiter.rs`,
  `rwlock/acquisition.rs`, `semaphore.rs`, and `worker_strategies.rs` account
  for the ambient-clock findings
- `src/verified/barrier.rs` has the current compound assertions
- `barrier.rs`, `lockset.rs`, and `rate_limiter.rs` contain the production
  panic-family warnings surfaced by the phase-1 spot check
