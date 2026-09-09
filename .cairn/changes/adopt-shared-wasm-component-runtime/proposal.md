# Proposal: Adopt the shared Wasm component runtime

## Why

Molten owns a hardened Wasmtime component shell, but the reusable runtime mechanism remains embedded in the product. The first `molten.wasm.component.v1` cohort admits no imports, and current evidence proves profile behavior without proving that ordinary product actors use the shared runtime.

The active `extract-hardened-wasmtime-runtime` change moves product-neutral Wasmtime admission and execution into a focused component. Molten needs a separate consumer change for parity, cutover, explicit product hostcalls, and real actor and system-extension use.

## What Changes

- Pin one completed immutable `hardened-wasmtime-runtime` revision and the published Mantle consumer verifier. r[molten.wasm_component_adoption.prerequisite]
- Dual-run the local and shared runtime paths over the existing positive and negative component corpus before cutover. r[molten.wasm_component_adoption.parity]
- Move Wasmtime feature, resource, world, import, export, and normalized-outcome mechanism to the shared runtime adapter. r[molten.wasm_component_adoption.cutover]
- Add one versioned Molten component profile with an exact product-owned effect-request hostcall and no ambient WASI. r[molten.wasm_component_adoption.hostcalls]
- Run one ordinary actor and one system extension through non-test product composition roots. r[molten.wasm_component_adoption.consumers]
- Preserve canonical Preserves, Basalt/UCAN, resource, replay, receipt, and effect authority in Molten. r[molten.wasm_component_adoption.boundary]
- Keep a coupled rollback until parity, product fixtures, and lifecycle gates pass. r[molten.wasm_component_adoption.rollback] r[molten.wasm_component_adoption.validation]

## Impact

- **Shared runtime**: owns product-neutral Wasmtime planning, linking mechanics, limits, and normalized outcomes.
- **Molten core**: owns actor, extension, effect, replay, receipt, and policy meaning.
- **Molten shell**: supplies exact product host adapters and executes approved effect plans.
- **Dependencies**: requires completed shared-runtime and Mantle-verifier publications plus matching Mantle materialization bundles.

## Out of Scope

- Ambient WASI, unrestricted filesystem or network imports, automatic fallback, or generic plugin discovery.
- Moving Molten effect semantics, capability authority, receipt schemas, or actor lifecycle into the shared runtime.
- Claiming Wasmtime, component, actor, extension, or whole-system correctness.

## Affected Specs

- `wasm-component-runtime-adoption`: prerequisite, parity, cutover, hostcalls, product consumers, rollback, and validation.
