# Runtime Spine Delta: Wasm ABI Negative Hardening

### Requirement: Wasm ABI input refs are recomputed during validation
r[molten.runtime.wasm_abi_negative_hardening.input_ref] Report validation MUST reject a Wasm execution receipt whose `input-ref` does not match the canonical actor-input envelope for the corresponding step.

#### Scenario: Tampered ABI input ref fails closed
- GIVEN a report containing a `molten.wasm.abi.v1` execution receipt
- WHEN the receipt input ref is changed
- THEN report validation fails before gate acceptance

### Requirement: Guest descriptors are bounded and checked
r[molten.runtime.wasm_abi_negative_hardening.descriptors] The Wasm shell MUST reject output descriptors that point outside guest memory or exceed deterministic ABI byte limits.

#### Scenario: Out-of-bounds output descriptor is rejected
- GIVEN a reviewed Wasm actor returns an output pointer/length outside exported memory
- WHEN execution reads guest output bytes
- THEN execution fails closed with no output evidence accepted

#### Scenario: Oversized output descriptor is rejected
- GIVEN a reviewed Wasm actor returns an output descriptor larger than the ABI output limit
- WHEN execution validates the descriptor
- THEN execution fails closed before reading or accepting output bytes

### Requirement: Hostcall bytes remain canonical Preserves
r[molten.runtime.wasm_abi_negative_hardening.hostcall_bytes] Hostcall imports exposed to Wasm actors MUST reject non-canonical or malformed Preserves request bytes.

#### Scenario: Invalid hostcall bytes trap the actor
- GIVEN a Wasm actor calls a declared `molten:hostcall/*` import with invalid bytes
- WHEN the hostcall import validates the request
- THEN execution fails closed and no hostcall decision is accepted

### Requirement: Fuel exhaustion is deterministic evidence
r[molten.runtime.wasm_abi_negative_hardening.fuel] Wasm execution MUST fail closed when deterministic fuel is exhausted.

#### Scenario: Infinite guest loop is rejected
- GIVEN a reviewed Wasm actor enters a loop that exceeds the fuel budget
- WHEN Wasmtime traps for fuel exhaustion
- THEN the harness reports deterministic executor failure instead of accepting partial output
