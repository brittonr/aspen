# Change: wasm-abi-negative-hardening

## Why

`molten.wasm.abi.v1` is now executable, but the negative/security rails need to be explicit as a separate hardening slice. Reviewed Wasm actors must fail closed for malformed guest memory descriptors, oversized canonical Preserves payloads, fuel exhaustion, invalid hostcall request bytes, and tampered ABI refs before their outputs can satisfy deterministic gates.

## What

- Expand Wasm ABI validation so execution receipts bind the expected canonical actor-input ref and bounded output byte count.
- Add negative suites for out-of-bounds output descriptors, oversized output descriptors, invalid hostcall bytes, fuel exhaustion, and tampered input/output refs.
- Keep WASI/component/ambient imports fail-closed and preserve no-WASI Wasmtime execution.

## Impact

Wasm executor pass evidence becomes less dependent on happy-path conformance and more robust against guest memory and transcript tampering. This remains core-module Wasmtime only; component model and WASI stay out of scope.
