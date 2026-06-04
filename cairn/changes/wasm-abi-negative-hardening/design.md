# Design: wasm-abi-negative-hardening

## Boundaries

Nickel/Basalt remain responsible for static executor fixture validation. The Wasmtime shell is responsible for deterministic guest memory checks at execution time. Guest bytes crossing the ABI are canonical Preserves values and are never accepted by string inspection alone.

## Receipt binding

The Wasm execution receipt continues to carry `molten.wasm.abi.v1`, `input-ref`, `output-ref`, and `output-bytes`. Validation recomputes the expected actor-input envelope ref from the suite step and checks that receipt field before gate acceptance. Output bytes are bounded and output refs remain replay-bound by deterministic re-execution.

## Negative coverage

Hardening tests cover:

- output descriptors outside exported memory;
- output descriptors larger than the deterministic ABI limit;
- invalid canonical Preserves output bytes;
- invalid canonical Preserves hostcall request bytes;
- fuel exhaustion/traps under Wasmtime fuel;
- replay tampering of ABI refs.

## Non-goals

This does not add WASI, component-model execution, async hostcalls, or ambient filesystem/network access.
