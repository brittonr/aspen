## Why

Job worker requests can now be built and executed through recorded local gossip, but operators still lack a coordination-backed scheduling path that proves FIFO queue admission, worker claim, lease/fencing, duplicate operation replay, and stale-token denial before worker side effects.

## What Changes

- Add a job-specific scheduled local worker command that enqueues a worker request through the coordination queue, replays the enqueue operation id, dequeues it for a worker session, acquires a coordination lock/fencing token, executes the recorded local-gossip worker only while the lease token is current, and releases the lease afterward.
- Emit a canonical job worker schedule receipt binding the worker request, queue key, lease key, coordination apply report, queue/claim/lease/release receipts, fencing token, worker receipt, and worker result.
- Extend job ledger/status/receipt UX to classify and summarize schedule receipts.
- Cover pass, duplicate enqueue replay, stale-token denial before worker execution, and ledger-visible schedule evidence.

## Impact

Operators can exercise the first coordination-backed worker scheduling flow end-to-end without treating the queue, lease token, transport, or CLI wrapper as authority. Authority, policy, resource, source-gate, provenance, sync, admission, and execution request evidence remain explicit inputs.
