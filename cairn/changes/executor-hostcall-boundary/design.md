# Design: executor hostcall boundary

## Boundary shape

All executors communicate through canonical Preserves hostcall envelopes. The executor never receives ambient access to the dataspace, filesystem, network, clocks, randomness, or process environment. It asks the runtime shell through hostcalls, and the shell admits or denies them through existing policy/capability/budget gates.

Candidate envelopes:

- `<actor-input-v1 ...>`: actor id, kind, turn id, message/assertion/demand value, context refs.
- `<hostcall-request-v1 ...>`: requested operation, target, value, capability request ref, budget debit.
- `<hostcall-decision-v1 ...>`: admitted/denied, authority ref, policy ref, budget ref, reason.
- `<actor-output-v1 ...>`: staged assertions/messages/effects plus deterministic trace refs.
- `<executor-preflight-v1 ...>`: executor kind, adapter version, sandbox config, conformance suite refs, checks.

## Steel

Steel execution is for reviewed dynamic predicates/trusted callables and actor adapters only after preflight. Steel code must not become an implicit policy backdoor. Source/module refs, reviewed callable receipts, and allowed hostcalls are explicit. The first local harness slice accepts reviewed Steel hostcall actor fixtures, emits source/callable/allowed-hostcall review receipts, rejects forbidden ambient IO source tokens, and still routes every requested operation through the canonical hostcall/admission shell.

## Wasm

Wasm/component execution uses deny-by-default WASI. Module/component refs, wasmparser inspection results, WIT interface refs, and sandbox config refs are part of preflight. The local harness accepts explicit Wasm module/WIT/allowed-hostcall fixtures, validates module bytes with `wasmparser`, records inspected imports, rejects ambient/WASI imports, and routes requested operations through the canonical hostcall/admission shell. Reviewed core Wasm hostcall actors now instantiate through Wasmtime with no WASI linker, deterministic fuel and memory limits, declared `molten:hostcall/*` imports only, required `molten_hostcall_<operation>` exports, exact hostcall-operation matching, and canonical `<wasm-execution-receipt-v1 ...>` evidence before runtime state changes. Component execution remains fail-closed until a component-specific adapter lands.

## Replay

Replay compares hostcall request/decision/output envelopes exactly. Non-deterministic hostcalls are represented as effect requests with recorded responses, just like current clock/random handling. Executor preflights bind deterministic conformance profile refs derived from the allowed hostcall set and canonical actor-input/request/decision/output envelope schemas; native, reviewed Steel, and reviewed Wasm actors with the same hostcall profile bind the same conformance ref and are covered by cross-kind suite tests over identical Preserves inputs.
