## Why

Unison abilities are useful prior art because they make effects visible in types instead of hiding them in ambient runtime behavior. Molten should adapt the visibility principle, not Unison's effect system: executable artifacts declare effect manifests, and local policy admits concrete handler profiles before any side effect occurs.

This keeps Wasm, Steel, native adapter, transcript, and job execution paths explicit about the operations they may request while preserving Molten's capability, policy, resource, replay, and evidence boundaries.

## What Changes

- Strengthen `effect-manifest-v1` as the canonical declaration of effect ids, operations, schemas, resources, handler profile compatibility, and evidence refs.
- Require handler profile admission receipts before execution uses production, local, chaos, profiling, or replay handlers.
- Deny undeclared effects and profile mismatches before side effects.
- Bind handler profile selection into replay, transcript, evaluation-cache, and remote execution evidence.

## Impact

- **Files**: runtime-spine/effects, executor adapters, transcript runner, evaluation cache, job DAG, remote execution receipts.
- **Testing**: positive fixtures for declared effects and admitted handler profiles; negative fixtures for undeclared effects, wrong schemas, stale profile receipts, missing capabilities, and nondeterministic replay profile changes.
- **Security**: effect declarations expose what may happen but do not grant authority. Capabilities, policy, resource, provenance, and source-gate evidence still gate execution.