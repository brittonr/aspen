## Why

WASM remains the right boundary for deterministic hooks, policies, and small service fragments, but Aspen needs an explicit host ABI and admission contract rather than treating plugins as the whole service architecture.

## What Changes

- Define WASM service/extension host admission.
- Specify ABI versioning, fuel/memory/time limits, deterministic host functions, and capability binding.
- Ensure WASM plugins remain bounded extensions unless explicitly wrapped as services.

## In Scope

- Active OpenSpec package for the WASM runtime service host implementation seam.
- Requirements, design constraints, implementation tasks, and verification plan.
- Integration with the existing runtime-host-loading and runtime-service-core direction.

## Out of Scope

- Moving all Forge or first-party services into WASM.
- Implementing a general marketplace.
- Allowing unbounded host functions.

## Verification

- `openspec validate implement-wasm-runtime-service-host --strict`
- Focused runtime-core or runner tests added by the implementation task.
- Docs/source-anchor tests where the change affects runtime architecture documentation.
- `git diff --check`
