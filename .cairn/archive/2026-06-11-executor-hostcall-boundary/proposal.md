# Change: executor-hostcall-boundary

## Why

The harness recognizes native, Steel, Wasm, adapter-backed, and remote-proxy actor kinds. Non-native execution needs a shared executor boundary so actors cannot bypass policy, capabilities, budgets, replay, or Preserves evidence rails.

## What

- Define canonical hostcall envelopes for actor input, dataspace operations, effect requests, capability checks, and actor output.
- Add executor preflight receipts for Steel and Wasm adapters before enabling execution.
- Allow reviewed Steel hostcall actors only when source/callable review receipts and allowed-hostcall bindings validate.
- Allow reviewed Wasm hostcall actors only when module inspection receipts, WIT refs, import checks, allowed-hostcall bindings, and no-WASI Wasmtime execution receipts validate.
- Keep unsupported executor kinds fail-closed until their preflight receipts and hostcall conformance tests pass.
- Bind every hostcall to actor id, turn id, capability context ref, policy ref, budget ref, and trace/effect refs.
- Add conformance suites that compare native and non-native actors over the same Preserves envelope behavior.

## Impact

This unlocks implementation of Steel and Wasm actor execution without weakening the existing deterministic harness. It also defines the adapter contract for future plugin-host and remote-proxy work.
