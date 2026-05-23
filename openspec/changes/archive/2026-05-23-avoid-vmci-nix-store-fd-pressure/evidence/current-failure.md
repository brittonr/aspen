# Current Failure Evidence

## Latest medium after selective input path rewrite

- Command: `nix run .#dogfood-local-vmci-medium`
- Process/session: `proc_877f0ed756ea`
- Log: `target/runtime-proof/vmci-rerun/medium-after-selective-input-path-rewrite-20260520T160845Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260520T162316Z.json`
- Result: exit code 1
- Proven passed boundaries:
  - VMCI local startup/readiness observed
  - source snapshot pushed to Forge
  - tracked WIP overlay applied (`files=45`)
  - CI pipeline discovered
  - `format-check` passed
  - `build-cli` reached `command_started`
  - workspace materialization completed before command start
- Failed boundary:

```text
copying path '/nix/store/i2gsp87gqp16whm9mw0ybk9n84zir01x-source' from 'https://cache.nixos.org'...
error: chmod ".../adoptopenjdk-icedtea-web/patches": Too many open files in system
unpacking 'github:NixOS/nixpkgs/b86751bc4085f48661017fa226dee99fab6c651b?...' into the Git cache...
```

## Interpretation

This failure occurs after executor command start inside guest Nix source/store handling. It is not VMCI startup, direct-route retention, Forge/ListRepos, source blob propagation, workspace setup, timeout finalization, Octet `HEAD` refresh, or `narHash` mismatch. The current root cause class is VMCI Nix source/store materialization FD pressure: guest Nix still handles a large `nixpkgs` source tree through a boundary that exhausts system file handles.

## Phase 1 classification implementation

Implemented diagnostic class `nix_source_store_fd_pressure` in `crates/aspen-dogfood/src/vmci_diagnostics.rs`.

Focused evidence:

- `classifies_nix_source_store_fd_pressure_after_command_start` feeds the latest medium stderr shape and requires:
  - boundary `executor_started`
  - failure class `NixSourceStoreFdPressure`
  - evidence `workspace_materialized`, `executor_started`, and `nix_source_store_fd_pressure`
  - post-registration classification
- `redacts_nix_store_source_subpaths_but_keeps_source_handle` keeps the bounded `/nix/store/<hash>-source` handle while redacting deep source subpaths and secret flags.
- Focused test command passed: `cargo test -p aspen-dogfood vmci_diagnostics -- --nocapture` (16 passed).


## Phase 2 VMCI-safe local store change

Audit result: the CI worker guest still mounts the host store as `/nix/.ro-store` via virtiofs and uses an overlay-backed `/nix/store` with tmpfs upper. Keeping `nixpkgs` fetcher-locked avoids path-rewriting it to a host `/nix/store` input, but the actual `nix build` still fetches/copies/chmod-walks public source paths through the overlay store. Because the overlay lower layer is the host virtiofs store, large source trees can still re-enter host `virtiofsd` and exhaust open files.

Implemented strategy: VMCI workers set `ASPEN_CI_NIX_LOCAL_STORE_ROOT=/tmp/aspen-ci-nix-store`; Nix command construction injects:

```text
--store local?root=/tmp/aspen-ci-nix-store --option min-free 0 --option max-free 0
```

That keeps command-fetched public inputs and build outputs in a guest-local tmpfs-backed Nix store instead of the overlay/virtiofs `/nix/store`. Selective private/offline rewrite remains narrow: `tigerstyle`/Octet and `ucan-src` are path-rewritten with preserved `original` and correct `narHash`; broad public inputs such as `nixpkgs` stay fetcher-locked.

Focused tests passed:

- `cargo test -p aspen-ci-executor-shell vmci_local_store_flags -- --nocapture`
- `cargo test -p aspen-ci-executor-shell local_executor -- --nocapture`

## Follow-up medium after guest-local store and UCAN rewrite

- Command: `nix run .#dogfood-local-vmci-medium`
- Process/session: `proc_023fbe25dad4`
- Log: `target/runtime-proof/vmci-rerun/medium-after-ucan-onixresearch-20260520T200715Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260520T203346Z.json`
- Result: exit code 1
- Proven passed boundaries:
  - VMCI local startup/readiness observed
  - source snapshot pushed to Forge
  - CI pipeline discovered
  - `format-check` passed
  - `build-cli` reached real Nix command execution in the guest-local store
  - no `Too many open files in system`
  - no UCAN SSH host-key failure
  - no GitHub `HEAD` fetch failure
- New failed boundary: `build-cli` reported `executor_timeout` while Nix was still actively fetching/building crate derivations in `/tmp/aspen-ci-nix-store` (for example `cranelift-*` and `crc*`).

Interpretation: the guest-local store fix removed the VMCI/virtiofs FD-pressure failure. The remaining failure is a cold guest-local Nix store timeout, not a route/source/workspace/store-FD regression.

## Timeout tuning implementation

The VMCI medium rail now gives `build-cli` a wider command window for first-run guest-local Nix store population:

```text
build-cli timeout_secs = 3600
pipeline timeout_secs = 5400
app-level ASPEN_DOGFOOD_CI_TIMEOUT_SECS default = 5700
```

This preserves the medium rail's scope (format plus build-only, no clippy/nextest/deploy) while allowing the cold local store to finish or fail on a real build error instead of the historical 1800s executor timeout.

## Follow-up after timeout tuning

- Command: `nix run .#dogfood-local-vmci-medium`
- Process/session: `proc_24d329d1dc77`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260520T223632Z.json`
- Result: receipt build stage failed; process exit code was misleading because the tee wrapper did not preserve pipefail.
- Proven passed boundaries:
  - VMCI startup/source push/CI discovery/workspace materialization worked
  - `format-check` passed
  - `build-cli` reached guest Nix execution
  - `build-cli` carried `timeout_secs=3600`
  - no `Too many open files in system`, no UCAN SSH host-key failure, no GitHub `HEAD` fetch failure
- Timing clue: the dogfood build stage ended after about 936s while the job log still showed active Nix crate fetching/building around `cranelift-*`; no `command_timeout`, `executor_job_timeout`, `result_publish_enter`, or `result_published` marker was present in the captured failure log.

Interpretation: the latest failure was not the 3600s local executor command timeout firing. It exposed a shorter dogfood/rail wait-budget path around the running VMCI build, then the receipt classifier labeled it `executor_timeout` because the available failure detail showed active executor command output. The follow-up patch adds an in-process rail minimum so `vm-ci-medium` cannot silently run with the generic smoke/default CI wait budget even when invoked outside the Nix app wrapper or with stale/missing environment defaults.

## Follow-up after rail wait-budget guard

- Command: `nix run .#dogfood-local-vmci-medium`
- Process/session: `proc_7af1c2828444`
- Log pointer: `target/runtime-proof/vmci-rerun/latest-medium-after-rail-wait-guard.logpath`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260521T003611Z.json`
- Diagnostics: `target/runtime-proof/vmci-diagnostics/dogfood-20260521T003611Z/`
- Result: exit code 1 with `build-cli` -> `executor_timeout`
- Proven passed boundaries:
  - VMCI startup/source push/CI discovery/workspace materialization worked
  - `format-check` passed
  - `build-cli` reached guest Nix execution
  - `build-cli` carried `timeout_secs=3600`
  - no `Too many open files in system`, no UCAN SSH host-key failure, no GitHub `HEAD` fetch failure
- Timing clue: the build stage ended after 805s while guest Nix was active around `datafusion-*`. No decisive `command_timeout` or `executor_job_timeout` origin marker was captured.

Interpretation: the rail wait-budget guard alone was not sufficient. The failure remains a cold guest-local Nix store realization problem, or an internal timeout path still being collapsed to `executor_timeout` without enough origin evidence. The follow-up implementation splits VMCI medium into an explicit `cache-warm` stage (`build-cli-deps`, 2400s) followed by `build-cli` (3600s), widens the medium pipeline/app wait budget, and adds bounded timeout-origin markers with `elapsed_secs` and `origin` fields so the next receipt distinguishes direct command timeout from VMCI wrapper timeout.

## Successful medium after cache-warm split and shell local-store propagation

- Command: `nix run .#dogfood-local-vmci-medium`
- Process/session: `proc_610989a9d21b`
- Log: `target/runtime-proof/vmci-medium-20260523T001418Z.log`
- Receipt: `/home/brittonr/.cargo-target/aspen-dogfood-vmci-receipts/dogfood-20260523T002711Z.json`
- CI run: `ef06231a-3b09-4ac0-a749-7272ad97014b`
- Result: exit code 0
- Passed stages/jobs:
  - `check` / `format-check`
  - `cache-warm` / `build-cli-deps`
  - `build` / `build-cli`
- Receipt timing:
  - `start`: succeeded in 37.735s
  - `push`: succeeded in 22.174s
  - `build`: succeeded in 2158.205s
  - `stop`: succeeded in 2.256s

Interpretation: VMCI medium now passes with the guest-local Nix store strategy and explicit cache-warm stage. The latest proof did not reproduce VMCI Nix source/store FD pressure, UCAN SSH host-key failure, GitHub `HEAD` fetch failure, or hidden executor-timeout misclassification.
