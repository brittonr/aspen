# Resume blocked scheduler runnables

## Why

F09 is an executed scheduler defect at source revision `fa1ced3e808861d8ce59f02a6fd6b13b655f5147`.
For one current-generation key, `Wake -> Block -> Wake` returns `DuplicateRunnable` on the final command.
Expected behavior is a bounded `Blocked -> Ready` transition for the same occurrence.
No other command resumes blocked work.

The affected core is `crates/molten-core/src/fabric_time/scheduler/mod.rs`, especially `wake`.
The current consumer is `ExtensionTimeContext` in `src/fabric_time/shell.rs`.
Its Wake precheck also charges resumed work as new work when the active limit is full.
That shell consequence has static evidence, not a separate executed counterexample.

## Proposed change

Add phase-aware Wake admission in the pure scheduler core.
Resume an existing blocked occurrence without another active slot or another record.
Apply the same ready-queue admission contract to Wake and Yield.
Preserve duplicate protection, generation fences, and state on rejection.

## Ownership and durable capability

Molten fabric-time maintainers own the core, shell integration, and regression tests.
The immediate outcome is resumable blocked work under existing admitted limits.
The durable capability is a repeatable phase-transition contract with core and adapter tests in normal repository paths.
Adoption uses the existing scheduler service. No new dependency is required.

## Evidence and scope

The audit reports `audit_blocked_runnable_can_wake_again` as an executed failing assertion against unchanged code.
Its core baseline reports 359 passing tests. The audit harness reports 41 passing controls and eight failing regression assertions.
Across the audit, eight findings have executed counterexamples and six have static evidence only.
This package copies the trigger and result into tracked Markdown. Ignored audit files are not acceptance receipts.
This planning pass ran no commands or tests.

This package does not authorize implementation or change accepted specs.
It does not claim global liveness, fairness, measured performance, production exposure, or repository acceptance.
