# Logical bug audit: 2026-09-08

## Scope and Evidence

The audit reviewed checkout `fa1ced3e808861d8ce59f02a6fd6b13b655f5147` with unrelated staged changes present.
Product code and those staged changes remained unchanged during the audit.

The existing `molten-core` suite passed 359 tests.
A separate harness linked the unchanged core and ran 49 cases: 41 existing controls passed and eight new regression assertions failed.
The remaining six findings have source-level evidence, not executed reproductions.
No live deletion, shutdown, migration, or crash injection ran.

The original scratch evidence is under `target/logical-audit/` and is not a durable prerequisite for any package.
Each change proposal and design retains its source locations, triggering sequence, evidence class, owner, and required regression cases.
The packages are plans only. No implementation task is complete, and no fix, archive, release, or push is authorized by their creation.

## Findings and Changes

| Finding | Severity | Evidence | Planned change |
|---|---|---|---|
| F01: denied shutdown changes node lifecycle state | High | Source | [fix-node-shutdown-admission](../../.cairn/changes/fix-node-shutdown-admission/proposal.md) |
| F02: normal startup supplies synthetic gate evidence | High | Source | [require-node-startup-source-gate-evidence](../../.cairn/changes/require-node-startup-source-gate-evidence/proposal.md) |
| F03: dedup commit precedes unrecoverable ingress enqueue | High | Source | [recover-ingress-enqueue-after-dedup](../../.cairn/changes/recover-ingress-enqueue-after-dedup/proposal.md) |
| F04: historical success suppresses repair after replica loss | High | Core reproduction | [repair-lost-replicas-after-prior-success](../../.cairn/changes/repair-lost-replicas-after-prior-success/proposal.md) |
| F05: cleanup removes the only replica in a required domain | High | Core plan reproduction | [preserve-replica-fault-domains-during-cleanup](../../.cairn/changes/preserve-replica-fault-domains-during-cleanup/proposal.md) |
| F06: merge preparation bypasses migration admission | High | Source | [admit-world-merge-migrations-before-execution](../../.cairn/changes/admit-world-merge-migrations-before-execution/proposal.md) |
| F07: missing targets disappear from under-replication status | Medium | Core reproduction | [preserve-under-replicated-status-without-targets](../../.cairn/changes/preserve-under-replicated-status-without-targets/proposal.md) |
| F08: duplicate claim returns a later consumer token | Medium | Core reproduction | [bind-delivery-duplicate-results-to-original-claims](../../.cairn/changes/bind-delivery-duplicate-results-to-original-claims/proposal.md) |
| F09: blocked runnable cannot resume | Medium | Core reproduction | [resume-blocked-scheduler-runnables](../../.cairn/changes/resume-blocked-scheduler-runnables/proposal.md) |
| F10: yield exceeds queue depth | Medium | Core reproduction | [enforce-scheduler-queue-bounds-on-yield](../../.cairn/changes/enforce-scheduler-queue-bounds-on-yield/proposal.md) |
| F11: terminal runnable records accumulate without a bound | Medium | Core reproduction | [bound-terminal-scheduler-retention](../../.cairn/changes/bound-terminal-scheduler-retention/proposal.md) |
| F12: exponential retry wraps to zero | Medium | Core reproduction | [saturate-exponential-retry-arithmetic](../../.cairn/changes/saturate-exponential-retry-arithmetic/proposal.md) |
| F13: merge publication confuses absent and generated roots | Medium | Source | [distinguish-absent-world-merge-roots](../../.cairn/changes/distinguish-absent-world-merge-roots/proposal.md) |
| F14: historical shutdown success becomes a current stopped observation | Medium | Source | [bind-shutdown-observations-to-current-node-run](../../.cairn/changes/bind-shutdown-observations-to-current-node-run/proposal.md) |

## Integration Groups

- Node lifecycle: F01 and F14 share dispatch and stopped-state observations. F02 owns startup gate evidence separately.
- Durable ingress: F03 shares storage-fault vocabulary with `recover-from-storage-faults`, but does not expand into consensus repair.
- Content replication: F04, F05, and F07 need a combined current-inventory, action, and status corpus after their separate corrections.
- Coordination delivery: F08 preserves existing owner and completion-authority checks. No authority bypass was reproduced.
- Scheduler: F09 and F10 must share ready-queue admission. F11 must bound retention without enabling stale-key resurrection.
- Retry: F12 affects the generic exponential planner. The current fixed-delay delivery profile is not evidence of exposure.
- World merge: F06 and F13 need a combined preparation and publication corpus without circular prerequisites.

These groups describe integration order, not completion dependencies or permission to implement.

## Required Completion Evidence

Each package requires a fresh relevant test baseline before core changes, ordinary repository regression tests, and positive and negative adapter cases.
Each requires exact compatibility and receipt decisions, documentation, focused strict Octet, Clippy, workspace and relevant Nix checks, and Cairn gates.
A blocker remains explicit rather than becoming a skipped check or false passing receipt.

A source finding must receive an executed public-path regression before a fix claims behavioral completion.
An unsafe cleanup plan does not prove live deletion. A wrong duplicate response does not prove an authority bypass.
A passing core test does not prove whole-system correctness, remote exploitability, release readiness, or every optional runtime profile.

## Lifecycle Tool Compatibility

The installed Cairn rejects the repository policy because its initial workflow registry lacks `outcome-machine`.
The repository pre-push convention supplies the policy from the sibling Cairn owner explicitly.
Planning validation uses `--policy ../cairn/cairn-policy/generated/cairn-policy.json` without modifying the repository policy.
The baseline owner-policy BLAKE3 is `7a67802377005c18ebded42899e7f564f5d87087b6c14ca70464b35886f340a9`.
This local validation route is not a new product dependency or a policy migration.
