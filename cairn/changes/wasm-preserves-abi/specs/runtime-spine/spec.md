# Runtime Spine Delta: Wasm Preserves ABI

### Requirement: Wasm actors use a canonical Preserves ABI
r[molten.runtime.wasm_preserves_abi.schema] Reviewed Wasm actors MUST exchange actor input, hostcall, and actor output data as canonical Preserves bytes under `molten.wasm.abi.v1`.

#### Scenario: Operation receives canonical actor input
- GIVEN a reviewed Wasm actor with valid inspection and executor preflight receipts
- WHEN an admitted operation runs
- THEN the runtime writes the canonical `<actor-input-v1 ...>` envelope into guest memory
- AND the operation entrypoint is invoked with the checked pointer and length

#### Scenario: Operation returns canonical actor output
- GIVEN a Wasm operation has completed
- WHEN the entrypoint returns an output descriptor
- THEN the runtime reads the referenced bytes only after bounds checks
- AND the bytes must parse as the expected canonical actor-output schema
- AND the output ref is recorded in the execution receipt

### Requirement: Guest memory crossing is explicit and bounded
r[molten.runtime.wasm_preserves_abi.memory] The Wasm ABI MUST require exported memory and checked allocation/deallocation entrypoints before bytes can cross the guest boundary.

#### Scenario: Missing memory export fails closed
- GIVEN a reviewed Wasm module without an exported `memory`
- WHEN execution attempts to pass Preserves bytes to the guest
- THEN execution fails closed before runtime state changes

#### Scenario: Out-of-bounds output descriptor is rejected
- GIVEN a Wasm entrypoint returns a pointer/length pair outside guest memory bounds
- WHEN the runtime reads actor output bytes
- THEN execution fails closed and no actor output is accepted

### Requirement: Hostcall imports exchange canonical envelopes
r[molten.runtime.wasm_preserves_abi.hostcalls] Imported `molten:hostcall/*` functions MUST accept canonical hostcall request bytes and return canonical hostcall decision or response bytes.

#### Scenario: Hostcall request is admitted by the shell
- GIVEN a Wasm actor calls `molten:hostcall/send` with a canonical request envelope
- WHEN the hostcall import runs
- THEN the runtime validates the request schema
- AND admission binds the decision to policy, capability, budget, actor, turn, and input refs
- AND the decision/ref is returned through the ABI

#### Scenario: Invalid hostcall bytes are rejected
- GIVEN a Wasm actor calls a hostcall import with non-canonical or invalid Preserves bytes
- WHEN the runtime decodes the request
- THEN execution fails closed before any effect or dataspace commit

### Requirement: Execution receipts bind ABI inputs and outputs
r[molten.runtime.wasm_preserves_abi.receipts] Wasm execution receipts MUST bind ABI schema refs, input refs, output refs, hostcall refs, fuel usage, memory limits, and byte-size limits.

#### Scenario: Replay detects tampered output
- GIVEN a report with a modified Wasm actor-output envelope
- WHEN replay recomputes Wasm execution
- THEN replay reports a Wasm execution divergence

#### Scenario: Oversized output exhausts deterministic budget
- GIVEN a Wasm actor returns more bytes than the configured ABI output limit
- WHEN execution validates the output descriptor
- THEN execution fails closed with a resource diagnostic

### Requirement: ABI conformance matches native executor behavior
r[molten.runtime.wasm_preserves_abi.conformance] Native and Wasm actors with the same executor conformance profile SHOULD produce identical canonical actor-output behavior for the same admitted inputs.

#### Scenario: Native and Wasm outputs match
- GIVEN a native actor and a reviewed Wasm actor with the same allowed hostcall profile
- WHEN both receive the same canonical actor input under the conformance suite
- THEN their canonical actor-output refs match
- AND the final runtime state hashes match
