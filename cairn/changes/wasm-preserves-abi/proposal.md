# Change: wasm-preserves-abi

## Why

The first Wasmtime executor path proves that reviewed Wasm actors can be instantiated without WASI, bounded by fuel/memory, and constrained to declared `molten:hostcall/*` imports. It is still only a skeletal hostcall shell: exported functions take no canonical input bytes and return no canonical output bytes. To make Wasm a real executor path, Molten needs a deterministic Preserves ABI that carries the same actor-input, hostcall request/decision, and actor-output envelopes used by native execution.

## What

- Define `molten.wasm.abi.v1` for reviewed core Wasm actors.
- Require exported memory plus explicit allocation/deallocation entrypoints for moving canonical Preserves bytes across the guest boundary.
- Invoke operation entrypoints with canonical actor-input envelope bytes and read canonical actor-output envelope bytes back from guest memory.
- Upgrade imported `molten:hostcall/*` functions to exchange canonical Preserves request/response bytes through the same pointer/length ABI.
- Bind ABI schema refs, input/output refs, hostcall refs, fuel, memory bounds, and byte-size limits into Wasm execution receipts.
- Add negative validation for missing memory/allocator exports, out-of-bounds ranges, oversized outputs, invalid Preserves bytes, fuel exhaustion, and output tampering.

## Impact

Reviewed Wasm actors become capable of real deterministic actor logic while preserving the same evidence, replay, and gate rails as native hostcall actors. WASI, component-model execution, and ambient imports remain fail-closed until separately specified.
