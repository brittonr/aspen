# Historical reference-scheduler execution

## Exact source and guard

- Clean detached worktree: `../.worktrees/aspen-batch-reference-20261025`.
- Base: `bb6f3830ee7327da9875ea85a8c8e25697eddc35`.
- Bundle BLAKE3: `74e9b89aa577943ee4edee37ecdc1cce0ebfdae254989ef6b6ae893b2b626874`.
- The unchanged delivered `apply.py` default dry-run returned `Validated; not applied`.
- `git apply --check` and `git apply` of that generated patch both exited 0.
- The delivered installer has no `--check` option; default dry-run is its check mode.
- Running the same installer against the current adoption checkout exited 1 with
  `Expected HEAD bb6f3830ee7327da9875ea85a8c8e25697eddc35`; see `moved-mainline.log`.
  No guard was removed and the historical patch was not applied to mainline.

## Executed commands

From the exact-base worktree, the current repository Nix development shell supplied
tools; the historical worktree retained its own pinned Cargo dependencies:

```sh
TMPDIR=/home/brittonr/.cache/aspen-batch-20261025/tmp \
CARGO_TARGET_DIR=/home/brittonr/.cache/aspen-batch-20261025/reference-target \
CARGO_BUILD_JOBS=4 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 \
nix develop /home/brittonr/git/OnixResearch/.worktrees/aspen-adopt-batch-20261025 \
  -c bash pilots/orchestration/scripts/reference-tests.sh
```

The script executed all three commands and exited 0:

| Command | Executed passing tests | Failed |
|---|---:|---:|
| `cargo test -p molten-core --test scheduler_lease_fencing` | 4 | 0 |
| `cargo test -p molten-core fabric_simulation` | 14 | 0 |
| `cargo test -p molten fabric_simulation` | 6 library, 3 main, 2 CLI integration | 0 |

The four delivered regressions accepted the current epoch once and rejected ABA
owner return, same-owner replacement with an old epoch, and zero/future epochs.
The remaining runs exercised reference transitions and the real CLI
run/replay/shrink/inspect/export path, including tampered-report denial.

This is the execution summary, not a byte-for-byte copy of compiler stdout. The
compiler cache reported an existing index I/O problem and disabled caching; all
commands above still completed successfully.

## Explicit mainline reconciliation

Current `DistributedSchedulerOperation::Complete` carries `assignment_token`, not
this older overlay's `lease_epoch` field. Current completion compares the supplied
token with `scheduler_assignment_token(job_id, owner, job.lease_epoch)` before
mutation and returns `StaleAssignmentToken` on mismatch. The token hashes the job,
owner and lease epoch with a domain separator. The existing contract is preserved;
this adoption does not replace it with a second completion API or claim that the
historical tests validate every current-mainline behavior.
