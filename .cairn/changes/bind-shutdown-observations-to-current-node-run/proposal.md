## Why

F14 identifies historical shutdown success as false current state at revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.

Dispatch shutdown request R, restart the node, then submit exact request R again. Restart removes the shutdown file but retains the old dispatch receipt. Duplicate dispatch returns that receipt without shutdown effects. The control loop sees a passing shutdown result, sets `has_stopped=true`, and exits with the current active lock intact.

Expected behavior suppresses the duplicate but does not report the current node as stopped. This finding is static restart and duplicate-path analysis only. The audit reports eight executed bugs and six static findings. F14 belongs to the static group. No live restart reproduction ran.

## What Changes

- Separate duplicate suppression from current stopped observations.
- Bind stopped observations to the current run and its observed lifecycle effects.
- Preserve historical request identity and suppress automatic reexecution.
- Add normal repository restart, duplicate, and current-shutdown regressions.

## Impact

- Current consumer: the node-control loop and service lifecycle readback.
- Maintenance owner: Molten node-runtime maintainers with node-host maintainers for lifecycle observations.
- Source scope: `src/node/parts/daemon/p027/body.rs`, `p018/body.rs`, and `p036/body.rs`.
- Spec delta: package-local `node-runtime` requirements.
- Durable capability: current-run lifecycle classification independent of historical duplicate results.
- Repeatability evidence: planned restart-sequence tests and pure observation tests.

## Non-goals and authority

This plan grants no implementation permission. It does not authorize replay of historical shutdown requests, claim process termination from receipt presence, or prove crash recovery. F01 separately owns admission before shutdown effects. Historical receipts remain historical evidence, not current authority or state.
