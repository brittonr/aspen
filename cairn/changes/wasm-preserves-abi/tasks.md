# Tasks: wasm-preserves-abi

- [x] [serial] r[molten.runtime.wasm_preserves_abi.schema] Define the `molten.wasm.abi.v1` schema, required exports, pointer/length descriptor encoding, and canonical input/output envelope refs.
- [x] [serial] r[molten.runtime.wasm_preserves_abi.memory] Implement checked guest memory writes/reads using exported `memory`, `molten_alloc`, and `molten_dealloc`.
- [x] [serial] r[molten.runtime.wasm_preserves_abi.entrypoints] Invoke `molten_hostcall_<operation>(ptr,len)` with canonical actor-input bytes and parse canonical actor-output bytes.
- [x] [serial] r[molten.runtime.wasm_preserves_abi.hostcalls] Upgrade imported `molten:hostcall/*` functions to exchange canonical Preserves request/response bytes through the ABI.
- [x] [serial] r[molten.runtime.wasm_preserves_abi.receipts] Bind ABI refs, input/output refs, hostcall refs, fuel, memory, and byte limits into Wasm execution receipts and gate checks.
- [x] [parallel] r[molten.runtime.wasm_preserves_abi.conformance] Extend executor conformance suites so native and Wasm actors produce identical canonical outputs for the same inputs.
- [x] [parallel] r[molten.runtime.wasm_preserves_abi.tests] Add negative tests for missing exports, out-of-bounds descriptors, oversized output, invalid Preserves output, fuel exhaustion, undeclared hostcalls, and tampered output refs.
