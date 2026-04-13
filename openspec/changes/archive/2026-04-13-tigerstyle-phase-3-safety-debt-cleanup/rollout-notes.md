# Ignored-Result Cleanup Notes

## Rollout Order

Phase 3 adopts the deferred safety families in this order:

1. `ignored_result`
2. `no_unwrap`
3. `no_panic`
4. `unchecked_narrowing`
5. `unbounded_loop`

Reasoning:

- `ignored_result` hides storage, coordination, and cleanup failures silently.
- `no_unwrap` is next because panic-on-error is still a direct production hazard, but its call sites need a larger API-by-API cleanup pass.
- `no_panic` stays behind `no_unwrap` because many remaining panic-family sites are test-only or deliberate invariants.
- `unchecked_narrowing` and `unbounded_loop` still need more false-positive triage before they belong in the pilot path.

## Pilot-Scope Inventory For `ignored_result`

Current pilot scope from `scripts/tigerstyle-check.sh`:

- `aspen-time`
- `aspen-hlc`
- `aspen-core`
- `aspen-coordination`

Inventory for this family:

| Crate | Inventory | Disposition |
|------|-----------|-------------|
| `aspen-time` | no lib-target `ignored_result` sites; one remaining raw `let _ =` is inside a test-only saturation probe | safe to promote now |
| `aspen-hlc` | no `ignored_result` sites in the lib target | safe to promote now |
| `aspen-core` | no lib-target `ignored_result` sites; remaining raw `let _ =` lines are test-only simulation cleanup | safe to promote now |
| `aspen-coordination` | 26 production `let _ = ...` removals across counter, election, queue, registry, rwlock, and worker coordination modules | required cleanup before promotion |

Source artifacts:

- `evidence/ignored-result-cleanup-counts.txt`
- `evidence/ignored-result-remaining-rg.txt`

## Highest-Risk Findings Addressed In `aspen-coordination`

### Counter buffering

- `crates/aspen-coordination/src/counter.rs`
- Background flush tasks used `let _ = c.add(to_flush).await;` after swapping local state to zero.
- Failure lost buffered increments silently.
- Fix: restore the local count on flush failure, log the error, and cover both background and explicit `flush()` paths with tests.

### Leadership state publication and shutdown

- `crates/aspen-coordination/src/election.rs`
- `crates/aspen-coordination/src/worker_coordinator/failover.rs`
- Watch-state sends and task joins were discarded.
- Fix: stop election loops when receivers disappear and log join failures instead of hiding them.

### Queue cleanup and dedup bookkeeping

- `crates/aspen-coordination/src/queue/create_delete.rs`
- `crates/aspen-coordination/src/queue/dequeue.rs`
- `crates/aspen-coordination/src/queue/enqueue.rs`
- `crates/aspen-coordination/src/queue/helpers.rs`
- Queue delete, DLQ cleanup, pending-item restore, and dedup-entry writes all had silent failure paths.
- Fix: propagate hard-delete failures where the caller can still react, log best-effort cleanup failures, and roll back pending claims when ready-queue deletion fails after a successful claim CAS.

### Registry, RW lock, and work-stealing cleanup

- `crates/aspen-coordination/src/registry/discovery.rs`
- `crates/aspen-coordination/src/registry/helpers.rs`
- `crates/aspen-coordination/src/rwlock/acquisition.rs`
- `crates/aspen-coordination/src/worker_coordinator/work_stealing.rs`
- Expired-instance cleanup, pending-writer cleanup, and steal-hint deletion dropped errors.
- Fix: log best-effort registry cleanup failures, warn on RW-lock pending-writer cleanup failures, and propagate steal-hint deletion failures to the caller/background monitor.

## Post-Cleanup Result

Targeted rerun:

```bash
scripts/tigerstyle-check.sh -p aspen-coordination -- --lib
```

Result:

- `aspen-coordination` now passes the targeted Tiger Style run with `ignored_result` enabled.
- The saved transcript has no `ignored_result` diagnostics.

Default pilot rerun:

```bash
scripts/tigerstyle-check.sh
```

Result:

- Default pilot now advertises `ignored_result` alongside `ambient_clock`, `compound_assertion`, and `contradictory_time`.
- Remaining pilot warnings are the pre-existing `ambient_clock` findings in `aspen-core/src/simulation.rs`; this slice did not add any new default-pilot noise.

Validation artifacts:

- `evidence/ignored-result-after.txt`
- `evidence/ignored-result-default-pilot.txt`
- `evidence/cargo-test-aspen-coordination-lib.txt`
- `evidence/implementation-diff.txt`

## No-Unwrap First Pass: Pilot-Scope Inventory

Scope for this first pass:

- `crates/aspen-time/src`
- `crates/aspen-hlc/src`
- `crates/aspen-core/src`
- `crates/aspen-coordination/src`
- lib-target code only; separate test-only and doc/comment-only occurrences before touching behavior

Inventory command artifacts:

- `evidence/no-unwrap-raw-rg.txt`
- `evidence/no-unwrap-pilot-inventory.txt`

First-pass split:

| Crate | Raw `.unwrap()` / `.expect()` matches | Production lib-target matches | Test-only matches | Doc/comment-only matches |
|------|--------------------------------------:|------------------------------:|------------------:|-------------------------:|
| `aspen-time` | 0 | 0 | 0 | 0 |
| `aspen-hlc` | 6 | 0 | 6 | 0 |
| `aspen-core` | 173 | 0 | 173 | 0 |
| `aspen-coordination` | 248 | 0 | 248 | 0 |

What this means:

- first-pass pilot lib targets have **zero production `no_unwrap` call sites** after filtering out tests and doc/comment noise
- the highest-risk production panic cleanup in this scope is therefore empty
- next step is safe: promote `no_unwrap` into the pilot path for these lib targets and save rerun transcripts

Noise cleanup applied during inventory:

- rewrote `aspen-coordination` doc examples to avoid `.unwrap()` in the published usage snippets
- rewrote `aspen-time` and `aspen-hlc` comments so future raw inventories are not inflated by comment-only matches

## No-Unwrap Promotion Result

Promotion changes:

- `scripts/tigerstyle-check.sh` now advertises `no_unwrap` in the default pilot lint set
- pilot crate attrs now enable `#![warn(no_unwrap)]` in:
  - `crates/aspen-time/src/lib.rs`
  - `crates/aspen-hlc/src/lib.rs`
  - `crates/aspen-core/src/lib.rs`
  - `crates/aspen-coordination/src/lib.rs`

Targeted rerun:

```bash
scripts/tigerstyle-check.sh -p aspen-coordination -- --lib
```

Result:

- clean for pilot lints after promotion
- no `no_unwrap` findings
- no coordination-local pilot warnings remain

Default pilot rerun:

```bash
scripts/tigerstyle-check.sh
```

Result:

- still no `no_unwrap` findings in the pilot scope
- remaining warnings are unchanged `ambient_clock` findings in `aspen-time` and `aspen-core`
- phase-3 promotion work did not add new pilot noise

Task decision:

- the last phase-3 task now meets its criteria
- repeating the pass for `no_unwrap` succeeded after `ignored_result` was bounded
- if we want the change to complete, checking the final task is justified

Validation artifacts:

- `evidence/no-unwrap-after.txt`
- `evidence/no-unwrap-default-pilot.txt`
