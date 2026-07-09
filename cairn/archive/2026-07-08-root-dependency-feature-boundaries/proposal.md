## Why

The root crate currently pulls together heavy runtime, storage, transport, executor, policy, and stack dependencies. That makes even pure code compile with adapter stacks and obscures which dependencies are core laws versus optional integration surfaces.

## What Changes

- Classify root dependencies as core, codec, policy/evidence, runtime, adapter, CLI, test, or integration.
- Introduce feature or crate boundaries so pure core code builds without Iroh, Redb, Wasmtime, Steel execution, Nickel tooling, or stack integration crates.
- Preserve default developer behavior while adding focused minimal-build checks.

## Impact

Dependency direction becomes visible and enforceable. The project can move toward smaller crates without breaking existing default builds.
