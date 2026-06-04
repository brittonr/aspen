## Why

`job-dag-remote-sync` makes a target registry contain the canonical job and stage artifact closure, but synced artifacts alone must not grant authority to run. Before any loopback or networked remote execution, Molten needs a target-side admission step that proves the target has the exact closure, policy/capability/evidence refs, executable-stage artifacts, and resource envelope required for the selected job stages.

## What Changes

- Add target-side job admission request, plan, and receipt records.
- Verify synced job/stage closures from the target registry by canonical artifact refs.
- Reject missing, tampered, path-based, raw-closure, or non-artifact executable stage configs.
- Bind policy, capability, evidence, and resource refs explicitly before any target execution is possible.
- Produce advisory admission plans and pass/deny receipts without executing stages.
- Add local/loopback CLI surfaces for admission planning and admission verification.

## Impact

This is the safety gate between artifact sync and future remote execution. It keeps remote execution fail-closed while providing canonical evidence that a target peer can independently validate synced job artifacts and selected stage requirements.
