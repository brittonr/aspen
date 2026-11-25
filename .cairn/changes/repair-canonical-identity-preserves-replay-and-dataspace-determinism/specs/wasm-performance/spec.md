# Wasm performance deltas

## ADDED Requirements

### Requirement: Wasm harness canonicalizes NaNs and enforces single-call limits synchronously
r[molten.wasm.harness_nan_canonicalization_and_sync_call_limit] The Wasm harness compiler MUST enable `cranelift_nan_canonicalization(true)` for deterministic floating-point playback and MUST enforce `require_single_call` synchronously inside the hostcall boundary before a second invocation executes.

#### Scenario: Second hostcall in single-call mode fails immediately
- GIVEN a Wasm module invoking a single-call host function twice
- WHEN the second invocation occurs
- THEN the hostcall trampoline MUST trap or deny synchronously before performing a second effect.
