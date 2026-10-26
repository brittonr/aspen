# Standalone batch adoption verification — 2026-10-25

## Scope and exact inputs

- Adoption parent: `f3c088ab9a950eebe40e420e3544d7af3440648b`.
- Source: repository-root `aspen-orchestration-batch-implementation.zip`, BLAKE3
  `74e9b89aa577943ee4edee37ecdc1cce0ebfdae254989ef6b6ae893b2b626874`.
- Imported member prefix:
  `aspen-orchestration-full/overlay/pilots/orchestration/` (57 supplied files).
- Original reference base: `bb6f3830ee7327da9875ea85a8c8e25697eddc35`.
- Independent source review date recorded by the proposal: 2026-09-09.
- Standalone `Cargo.lock` BLAKE3:
  `8691713a157a3a85c51b8d29df9edb78c93facd0593100401d24662bbf7a47e0`.
- Exact adopted payload is the source committed with this record; the parent hash
  is not a claim that the parent already contains the pilot.
- `pilots/orchestration/source.ncl` records all repairs, AGPL licensing, ownership,
  toolchain and deployment profile. No root dependency or lockfile was changed.

## Observed verification

From `pilots/orchestration`, with the repository Nix shell:

```sh
TMPDIR=/home/brittonr/.cache/aspen-batch-20261025/tmp \
CARGO_TARGET_DIR=/home/brittonr/.cache/aspen-batch-20261025/target \
CARGO_BUILD_JOBS=6 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
CARGO_INCREMENTAL=0 CLIPPY_STRICT=1 \
nix develop /home/brittonr/git/OnixResearch/.worktrees/aspen-adopt-batch-20261025 \
  -c bash scripts/verify.sh
```

Final exit code: 0. The retained `verify.log` records:

- `rustc 1.98.0-nightly (31a9463c6 2026-05-25)` and
  `cargo 1.98.0-nightly (4d1f98451 2026-05-15)` from the pinned shell.
- Seven-crate manifest/dependency and lexical effect-boundary check: pass. This is
  lexical evidence, not a proof of semantic purity.
- `cargo fmt --all`: pass.
- `cargo test --workspace --all-targets --locked`: **62 passed, 0 failed**.
  Includes the held-directory synchronization regression, real Redb reopen/CAS,
  actual Wasmtime positive and deny cases, and the real two-worker CLI DAG,
  restart, idempotency, unenrolled-peer denial and cancellation case.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: pass.
- `cargo build --release -p molten-orchestrator-cli --locked`: pass.

The compiler cache reported an existing index I/O error and disabled caching.
It did not prevent these commands from completing.

## Actual operator surface

The release binary was exercised through `bash scripts/demo.sh` in the same pinned
shell, with `CARGO_TARGET_DIR` selecting the built executable. Final exit code: 0.
The script launched separate controller and worker processes, submitted tasks
`a -> b`, fetched the final content and asserted exact bytes `b"ok"`.

- Final phase: `Succeeded`; both tasks succeeded on attempt 1.
- Output object: `ed53edd46070bae2811316d2ecabd484248f9e1438b0c9efddd2ace71bf76370`.
- Fetched output: 2 bytes, exactly `ok`.
- Private run directory retained and inspected after command exit:
  `/home/brittonr/.cache/aspen-batch-20261025/tmp/nix-shell.xrZLe7/aspen-batch-demo.V4HO6Z`.
- Repository evidence: `verify-and-demo.log`, `demo-status.json`, `demo-job.json`,
  `demo-controller.log`, `demo-worker.log`. No private keys or databases were copied.

## Historical reference patch and mainline reconciliation

The unchanged installer generated and checked its patch at the exact original
base in a clean detached worktree. `git apply --check`, `git apply`, and all three
commands in `scripts/reference-tests.sh` passed: four lease-fencing regressions,
14 core simulation tests, six root library tests, three main tests and two CLI
integration tests — **29 passed, 0 failed**. See `reference-results.md`.

The installer was separately exercised against the moved adoption checkout and
refused it before patch generation; `moved-mainline.log` retains the denial.
The delivered installer has no `--check` flag: its default dry-run is the guarded
check mode. The task/scenario wording is corrected to that actual interface.

Current mainline already uses a job/owner/epoch-bound assignment token. The
historical command-shape patch is not reapplied or used to replace that API.

## Retained failed attempts

- `initial-verify.log`: 62 tests passed; strict Clippy found the oversized
  `Command::Schedule` variant. The repair borrows the transient request and removes
  a redundant clone instead of allocating a box.
- `durability-verify.log`: strict Clippy found four ignored
  `set_durability(Immediate)` results. Every result now propagates before table
  mutation or commit; no lint or durability requirement was weakened.
- `interrupted-verify.log`: tests, Clippy and the release build passed, but adding
  a license header to the running shell script disturbed its final read. The script
  itself was syntax-checked, frozen and rerun successfully. This failed invocation
  is not counted as the final successful verification.

The review's pre-repair compilation and `O_PATH`/`EBADF` observations were treated
as established input, not rerun to reconfirm them. The permanent held-root test
passes after repair; its create/sync path reaches the formerly broken operation.

## Boundary and additional checks

- Locked Cargo metadata confirms seven independent AGPL-licensed pilot packages
  using registry dependencies (`workspace-metadata.json`). Root metadata still
  lists only `molten`, `molten-core`, `molten-node-host`, `molten-release-policy`;
  the pilot is not enrolled (`root-workspace-metadata.json`).
- Root `cargo fmt --all --check`: pass.
- Bash syntax checks for all four supplied scripts: pass.
- `nickel typecheck pilots/orchestration/source.ncl`: pass.
- Architecture, operator docs and the per-port follow-on integration plan name
  `standalone-single-controller-import-free-component-batch-v1` and Molten
  contributors as the maintenance owner.
- Dispatcher, ALPN/fabric-port registries, system-tier manifests, root workspace,
  root dependencies and Basalt/Cairn production admission are intentionally unchanged.
  No root hermetic Nix build, VM result, cross-host result or production source-gate
  acceptance is claimed by this isolated pilot's execution.
- Changed documentation sections/files: balanced code fences and seven valid
  relative file links across the README, architecture, operator guide, pilot
  README, integration plan and review. Checks use raw tool output; anchored reader
  excerpts are not complete Markdown documents.
- Accepted-spec sync completed. The inherited-debt guard reports 2,962 requirements,
  1,028 referenced, 1,934 uncovered and zero dangling references. The uncovered list
  and its BLAKE3 remain unchanged; only observed metadata totals were regenerated
  from Nickel. See `tracey-guard.log`.

## Non-claims

Passing adoption evidence does not prove Wasmtime, component, worker, transport,
whole-system or cross-host correctness, production isolation, controller HA,
exactly-once external effects, or release eligibility. Operator policy, public
peer identity, output possession and passing logs do not grant Molten system
extension authority. Native integration remains a separate gated change.
