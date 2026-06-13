## Why

Local job DAG execution, sync, and admission are close to remote execution but stop before a target worker actually runs admitted work over Iroh. After loopback execution is implemented, Molten needs a live remote-shaped worker protocol that preserves the same evidence contract: sync first, target admission, execution from target state only, recorded delivery, and deterministic replay or explicit non-replayable exclusion.

## What Changes

- Add canonical job worker request, assignment, execution, status, result, and worker receipt records carried over remote dataspace/Iroh envelopes.
- Require a passing loopback-compatible admission receipt before a worker starts any stage.
- Fetch/verify missing artifact/chunk closure through the existing Iroh/chunk/artifact sync path.
- Execute only within the target node state roots and bind stage receipts, resource consumption, authority receipts, and result refs.
- Record delivery/status/result logs for replay gates; live unrecorded worker runs remain non-deterministic diagnostics.

## Impact

This is the first real remote job execution rail. It turns the local job DAG into a distributed substrate while preserving the no-mobile-closure, target-admission, and receipt-backed safety model.
