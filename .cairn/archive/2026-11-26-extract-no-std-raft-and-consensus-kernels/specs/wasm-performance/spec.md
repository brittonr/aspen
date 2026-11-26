# Wasm performance deltas

## ADDED Requirements

### Requirement: Wasm component admission and optimization policy live in a no_std kernel without Wasmtime
r[molten.wasm.no_std_wasm_policy_kernel] Wasm component import/export admission, profile admission, and optimization-profile selection MUST live in a `#![no_std]` + `alloc` crate (`molten-wasm-policy-core`) with zero `wasmtime` or filesystem dependencies.

#### Scenario: Wasm policy evaluates without Wasmtime or std
- GIVEN parsed component import descriptors and an optimization profile
- WHEN `molten-wasm-policy-core` evaluates admission under `#![no_std]`
- THEN it MUST return deterministic allow/deny decisions without linking `wasmtime` or reading the filesystem.
