## Why

Distributed runs can emit many receipts while still leaving ambiguity about whether node-local views converged. Reviewers need a canonical reconciliation gate that compares state, queue, ledger, chunk, receipt, and protocol refs across nodes instead of relying on logs or manual inspection.

## What Changes

- Add a cross-node reconciliation gate for multinode simulation, local multiprocess, VM, and soak evidence where applicable.
- Bind per-node state refs, ledger refs, queue refs, chunk manifest refs, control receipt refs, ack refs, and expected equivalence classes.
- Add positive coverage for converged nodes and negative coverage for missing refs, stale refs, wrong topology, duplicate commit, divergent queues, and diagnostic-log-only reconciliation.
- Document which refs must match exactly and which are allowed to differ only when declared as variance.

## Impact

A distributed run cannot claim pass evidence merely because each node produced some local receipts. The gate makes convergence, divergence, and declared variance explicit.