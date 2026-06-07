## Why

The job DAG Iroh worker protocol exists as a core runtime path, but operators still need an explicit CLI workflow for constructing worker requests and running the deterministic local-gossip worker harness. Without this, remote worker evidence remains hidden behind tests and cannot be chained with sync/admission/execution receipts from scripts.

## What Changes

- Add CLI support for canonical `job-worker-request-v1` records from admission and execution request artifacts.
- Add a deterministic local-gossip worker runner that builds a worker envelope, publishes/delivers it through the remote dataspace transport, records a delivery log, executes on target roots, and writes indexed worker evidence.
- Extend job receipt summaries/status output to include worker receipts and results.
- Document the worker UX and preserve the existing trust boundary: transport and CLI invocation are evidence only, not authority.

## Impact

Operators can now exercise the first remote-shaped worker flow end-to-end from the CLI while keeping sync, admission, authority, resource, delivery, and replay evidence explicit.
