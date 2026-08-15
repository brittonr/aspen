## Why

Once Molten has a deterministic Wasm component profile, runtime tuning will affect compilation latency, instantiation latency, steady-state execution, memory reservation, concurrency capacity, and startup behavior. Today there is no component-specific performance evidence contract that separates those phases or prevents a faster but differently configured run from being compared as if it were the same execution profile.

Molten needs bounded, reviewable performance evidence before adopting precompiled artifacts, pooling allocation, copy-on-write heap images, `InstancePre`, Wizer snapshots, or other startup/runtime optimizations.

## What Changes

- Add a pinned Sightglass benchmark suite for representative Mantle-materialized Molten component actors and system-extension callbacks.
- Require portable, Wizer, and precompiled benchmark inputs to arrive through exact Mantle materialization bundles; Molten measures and admits them but does not produce those build artifacts.
- Record compilation, instantiation, and execution measurements separately with benchmark, host, engine, profile, and sample identities.
- Define deterministic statistical comparison and regression classification without cross-host or cross-runtime overclaims.
- Admit Wasmtime precompiled components only when exact Mantle/Valence provenance, target, CPU-feature, runtime-configuration, and artifact identities match.
- Add separately reviewed profiles for pooling allocation, copy-on-write heap images, `InstancePre`, and bounded concurrency/backpressure.
- Admit Mantle-produced Wizer artifacts only after deterministic build inputs, denied/virtualized imports, repeated output identity checks, and pre/post artifact receipts pass.

## Impact

- **Surfaces**: Mantle materialization-bundle admission, benchmark fixtures, runtime profiles, performance receipt DTOs, AOT admission, component cache/startup paths, Nix checks, and operator reports.
- **Dependency**: this change depends on `adopt-wasm-component-runtime-profile`; optimization cannot define or weaken the component correctness, authority, or deterministic-execution profile.
- **Claims**: measurements are scoped to the declared host and benchmark configuration. They do not prove correctness, release eligibility, cross-machine performance, or superiority to another runtime.
