# Design: Wasm Preserves ABI

## ABI boundary

`molten.wasm.abi.v1` is a core-module ABI. Components and WASI remain disabled. The harness/runtime loads a reviewed module only after the existing Wasm inspection receipt validates module bytes, imports, WIT refs, allowed hostcalls, conformance refs, and sandbox settings.

Required exports:

- `memory`: linear memory used only through checked pointer/length ranges.
- `molten_alloc(len: i32) -> i32`: allocates guest memory for canonical input or hostcall response bytes.
- `molten_dealloc(ptr: i32, len: i32)`: releases guest memory previously returned by `molten_alloc` or by an operation entrypoint.
- `molten_hostcall_<operation>(input_ptr: i32, input_len: i32) -> i64`: executes an admitted operation and returns an output descriptor where high 32 bits are pointer and low 32 bits are length.

The runtime writes canonical Preserves bytes to guest memory only after bounds checks and reads output bytes only after validating the returned descriptor.

## Canonical envelopes

Operation input bytes are the canonical packed Preserves encoding of `<actor-input-v1 ...>`. Operation output bytes are the canonical packed Preserves encoding of `<actor-output-v1 ...>` or a stricter successor schema referenced by the ABI receipt. Output values are parsed, schema-checked, hash-bound, and then compared against the runtime-computed hostcall/admission evidence before any state change is committed.

Imported `molten:hostcall/<operation>` functions use the same byte ABI. The guest passes a canonical `<hostcall-request-v1 ...>` envelope and receives a canonical `<hostcall-decision-v1 ...>` or hostcall response envelope. The imported function is still only a request path: the runtime shell performs policy, capability, budget, effect-log, and replay checks.

## Deterministic bounds

The ABI has explicit byte limits:

- max actor input bytes;
- max actor output bytes;
- max hostcall request bytes;
- max hostcall response bytes;
- max allocated memory bytes;
- fuel limit and fuel remaining;
- maximum number of hostcalls per operation.

All limits are part of the sandbox/config ref and are copied into the Wasm execution receipt. Exhaustion, allocation failure, invalid pointers, non-canonical bytes, invalid schemas, or extra hostcalls fail closed before runtime commit.

## Evidence

`<wasm-execution-receipt-v1 ...>` is extended or superseded to include:

- ABI schema id/ref;
- module ref and inspection receipt ref;
- sandbox config ref;
- operation entrypoint;
- actor input ref;
- actor output ref;
- each hostcall request/decision/response ref;
- fuel limit/remaining;
- memory/table/byte limits;
- checks for no-WASI, allowed imports, bounds, canonical Preserves parsing, and replay binding.

Replay validates the receipt by re-running the same module over the same canonical inputs and comparing every output and hostcall ref exactly.
