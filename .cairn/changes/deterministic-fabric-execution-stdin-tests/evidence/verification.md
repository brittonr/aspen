# Verification: Make the fabric_execution stdin tests deterministic

Base: `integration/gate-fixes-20260924` `dca8f7135e09a51a35a961b160b6b71c05102c97`, which is 13 commits on
`origin/molten` `4e31cee55`. Branch `fix/fabric-execution-stdin-epipe-tests`, worktree
`/home/brittonr/git/aspen-worktrees/fabric-exec-epipe`, private `CARGO_TARGET_DIR`
`/home/brittonr/git/aspen-worktrees/fabric-exec-epipe-target`.

## Root cause (confirmed)

The pinning test printed the adapter's failure detail once, through a temporary `eprintln!` that was then removed:
`WriteStdin failed: Broken pipe (os error 32)`. The `bounded-exec` revision `29dac88` reports that failure for a
child that exits normally, and the adapter maps it to `UnknownAfterStart`.

## Repetition

`loop-200.sh` builds the lib test binary once and runs `fabric_execution::` 200 consecutive times. Every run executes
all 10 tests in the module.

| Mode | Log | Result |
|---|---|---|
| `--test-threads=1` | `loop-200-serial.log` | pass=200 fail=0 |
| default parallelism | `loop-200-parallel.log` | pass=200 fail=0 |

The same loops also passed 200/0 in both modes before `cargo fmt` rewrapped three statements.

## Gates

| Command | Exit | Result |
|---|---|---|
| `nix develop -c cargo fmt --check` | 0 | clean |
| `nix develop -c cargo clippy --workspace --all-targets -- -D warnings` | 0 | no warnings |
| `nix develop -c cargo test --workspace` | 0 | 2072 passed, 0 failed across all workspace test binaries and doctests |
| `nix build .#checks.x86_64-linux.nextest` | NEXTEST_EXIT | NEXTEST_RESULT |
| `cairn gate proposal\|design\|tasks` (explicit Cairn policy) | 0 | 0 issues each |
| `cairn validate --strict` (explicit Cairn policy) | 0 | valid, 0 issues |

Local `cargo nextest run` could not be used. It calls `cargo metadata --all-features`, and that command panics at
`cargo-util-schemas/src/core/package_id_spec.rs:248:40` on the feature-gated `rad://` package IDs. The same panic is
already recorded in the `add-addressable-actor-runtime-profile` archive. The hermetic Nix nextest check is the nextest
evidence.

## Follow-up probe (dev-only, not release evidence)

A throwaway detached worktree held these test changes. It used Cargo `--config patch` to point `bounded-exec` and
`bounded-exec-core` at the local bounded-exec `observe-unconsumed-input` commit
(`4e805c819011cab08676e58acd3455c6b5063695`, unpublished). The worktree was removed after the run.
`override-dev-only-summary.txt` records the output. Aspen compiled without source changes, and 9 of 10
`fabric_execution::` tests passed. As intended, the pinning test failed: the adapter now returns `Ok` with `Exited`,
`ExitPolicyAccepted`, and a receipt. The revision that adopts `InputDelivery` must revise this test.

A Nix `--override-input bounded-exec-src path:...` run is blocked by design. The build-plan staleness check rejects
the changed lock, and the flake asserts `bounded-exec-src.rev == boundedExecRevision`.
