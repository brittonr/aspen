## Why

Cluster lifecycle tests currently inspect per-node receipts and rendered CLI output, but the cluster wrapper does not emit one canonical artifact that binds the entire run. Reviewers have to reconstruct manifest identity, node ordering, already-running decisions, stop order, and per-node receipt sets from multiple files and diagnostic text.

## What Changes

- Add a first-class `cluster-lifecycle-run-v1` receipt for `cluster init`, `cluster start`, `cluster status`, and `cluster stop` workflows.
- Bind manifest refs, node order, per-node lifecycle/control receipts, already-running decisions, stop ordering, diagnostics, and evidence-only caveats.
- Add positive coverage for a complete two-node lifecycle and negative coverage for missing or stale lifecycle evidence.
- Keep cluster planning and receipt validation in pure cores; keep filesystem and process execution in CLI/test shells.

## Impact

Cluster lifecycle evidence becomes easier to review and safer to use in later drift, VM, and release-readiness gates. The receipt remains local cluster-wrapper evidence only and does not grant authority, policy, provenance, source-gate, transport, retention, deployment, or production-readiness trust.
