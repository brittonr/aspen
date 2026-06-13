# Tasks: wasm-abi-negative-hardening

- [x] [serial] r[molten.runtime.wasm_abi_negative_hardening.input_ref] Validate Wasm ABI execution receipts against the recomputed canonical actor-input ref.
- [x] [serial] r[molten.runtime.wasm_abi_negative_hardening.descriptors] Add fail-closed coverage for out-of-bounds and oversized output descriptors.
- [x] [serial] r[molten.runtime.wasm_abi_negative_hardening.hostcall_bytes] Add fail-closed coverage for invalid canonical Preserves hostcall request bytes.
- [x] [serial] r[molten.runtime.wasm_abi_negative_hardening.fuel] Add fail-closed coverage for deterministic Wasmtime fuel exhaustion.
- [x] [parallel] r[molten.runtime.wasm_abi_negative_hardening.input_ref] Add replay/validation coverage for tampered ABI input/output refs.
