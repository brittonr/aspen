## Why

Molten already executes reviewed no-WASI core modules through Wasmtime with explicit imports, deterministic fuel, store limits, canonical Preserves input/output, and `molten.wasm.abi.v1` receipts. SpaceWasm offers a narrower interpreter with deterministic instruction stepping, pause/out-of-fuel resumption, streaming decode, and explicit allocation failure. Those properties may improve deterministic turn segmentation and provide a second runtime observation for the existing core-module ABI.

SpaceWasm is not a Component Model runtime and is currently unreleased, version `0.0.0`, missing several Rust-emitted post-MVP features, and still developing its ground checker, performance, and continuous fuzzing. Molten therefore needs an explicit research profile rather than a replacement, automatic fallback, or production claim.

## What Changes

- Add a non-default `spacewasm-core-mvp-pilot` runtime profile separate from the existing Wasmtime core-module and planned component profiles.
- Require an exact, verified Mantle SpaceWasm reference bundle and compatible Octet/ChaosControl evidence before pilot execution can become evidence-bearing.
- Admit only core modules in the exact SpaceWasm/Wasmtime feature intersection with explicit import/export, memory/table, `memory.grow`, stack, allocation, and instruction bounds.
- Adapt the existing `molten.wasm.abi.v1` Preserves byte ABI and declared `molten:hostcall` imports through a thin SpaceWasm shell without WASI or ambient host authority.
- Schedule actor turns through bounded instruction segments and distinguish finish, deterministic trap, out-of-fuel yield, host pause, denial, and harness failure.
- Emit pilot inspection, instantiation, execution, yield/resume, hostcall, resource, and final-state receipts binding exact engine/source/profile/configuration identities.
- Prove replay only for exact admitted artifacts, inputs, recorded effects, segment plans, and runtime configuration; do not claim canonical interpreter-state serialization or migration.
- Keep the profile diagnostic/experimental until a later change explicitly promotes a reviewed cohort.

## Impact

- **Surfaces**: Wasm runtime-profile configuration, executor admission core, SpaceWasm shell adapter, hostcall ABI, deterministic scheduling, resource accounting, receipts, fixtures, replay, and operator status.
- **Dependencies**: consumes a Mantle reference bundle, Octet static artifact facts, ChaosControl differential evidence, and existing Molten authority/resource admission without transferring their claims.
- **Compatibility**: core Wasmtime, SpaceWasm pilot, and Component Model profiles remain explicitly named; no profile silently falls back to another.
- **Claims**: a passing pilot receipt proves only bounded execution under the exact profile. It does not prove SpaceWasm correctness, sandbox completeness, state migration, component compatibility, or production readiness.
