## Why

Molten actors, Wasm components, Steel orchestration, blob transfers, storage writes, remote sync, and distributed jobs can consume unbounded CPU, memory, mailbox space, storage, network, and policy resources unless governance is explicit. Deterministic replay also requires resource decisions to be recorded and reproducible.

## What Changes

- Define resource budgets, quotas, leases, and backpressure as admitted runtime policy.
- Add limits for CPU/fuel, actor turns, mailbox depth, dataspace assertion count, memory, blob bytes, storage bytes, remote fetches, effect calls, and trace volume.
- Require Wasmtime fuel/epoch or equivalent execution budgeting and Steel/native operation budgets.
- Add deterministic backpressure behavior for local queues and remote adapters.
- Emit receipts and traces for budget grants, consumption, throttling, denial, cancellation, and cleanup.
- Integrate budgets with effect manifests, handler profiles, job DAGs, remote sync, typed storage, and supervision.

## Impact

This prevents runaway actors and makes load behavior testable. The first milestone can add actor turn budgets, mailbox bounds, dataspace assertion bounds, and deterministic backpressure decisions in the local runtime.
