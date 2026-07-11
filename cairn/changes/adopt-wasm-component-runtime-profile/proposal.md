## Why

Molten already executes reviewed core WebAssembly modules with Wasmtime, deterministic fuel, explicit memory limits, import inspection, deny-by-default hostcalls, and canonical Preserves input/output receipts. That boundary still uses a custom pointer-and-memory ABI, treats WIT as inspection metadata rather than the executable contract, and rejects components instead of admitting a versioned component profile.

A shared WebAssembly Component Model profile is needed before ordinary plugins, system extensions, Kamacite adapters, or future Animus hosts independently invent incompatible Wasm ABIs or weaken Molten's deterministic and authority boundaries.

## What Changes

- Define a versioned Molten Wasm component runtime profile with one pinned Wasmtime, wasm-tools, wit-bindgen, WASI, WIT package/world, and enabled-feature compatibility cohort.
- Use WIT for the outer executable ABI while retaining canonical Preserves bytes and schemas as the authoritative actor, hostcall, and receipt payload representation.
- Extend deterministic execution admission to cover NaN canonicalization, relaxed SIMD posture, memory/table growth, fuel interruption, explicit imports, and deterministic host inputs.
- Keep WASI and host capabilities denied unless the component profile, Basalt/UCAN authority, policy, and resource admission all authorize a specific binding.
- Emit component inspection, instantiation, execution, hostcall, and migration receipts that bind exact component bytes, WIT identity, runtime configuration, toolchain cohort, inputs, outputs, and non-claims.
- Provide an explicit migration lane from the current `molten.wasm.abi.v1` core-module profile without silent fallback between core modules and components.

## Impact

- **Surfaces**: Wasm executor core and shell, plugin/system-extension execution profiles, WIT packages, Nickel runtime-profile configuration, Preserves receipts, fixtures, and operator readback.
- **Cross-stack inputs**: Mantle may later supply built/precompiled components, Octet may supply artifact checks, Valence may wrap component evidence, and Kamacite may supply adapter descriptors. Molten still owns runtime admission and execution semantics.
- **Safety**: WIT compatibility is not behavioral correctness; component validity is not authority; WASI virtualization is not a substitute for runtime linker admission; execution receipts do not prove application correctness.
- **Compatibility**: the existing core-module profile remains separately named during migration and is never selected as an implicit fallback for a component request.
