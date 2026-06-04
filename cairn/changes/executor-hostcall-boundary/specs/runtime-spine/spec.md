# Runtime Spine Delta: executor hostcall boundary

### Requirement: Executors use canonical hostcall envelopes
r[molten.runtime.executor_hostcall_boundary.envelopes] Non-native executors MUST interact with the runtime only through canonical Preserves actor input, hostcall request/decision, and actor output envelopes.

#### Scenario: Steel actor requests a send hostcall
- GIVEN a Steel actor with valid executor preflight evidence
- WHEN it requests a send operation
- THEN the runtime records a canonical hostcall request
- AND admission binds the decision to policy, capability, budget, actor, and turn refs

#### Scenario: Reviewed Steel preflight binds source, callable, and hostcalls
- GIVEN a Steel actor with a reviewed source/callable fixture
- WHEN executor preflight evidence is emitted
- THEN it includes a Steel review receipt binding the source ref, callable name, and allowed hostcalls
- AND replay/validation rejects stale, missing, or tampered Steel review receipts

#### Scenario: Steel undeclared hostcall is rejected before effects
- GIVEN a Steel actor whose reviewed fixture allows only a subset of hostcalls
- WHEN a suite step requests an undeclared hostcall
- THEN execution fails closed before side effects occur

#### Scenario: Wasm preflight binds module, imports, WIT, and hostcalls
- GIVEN a Wasm actor with an explicit module/WIT/allowed-hostcall fixture
- WHEN executor preflight evidence is emitted
- THEN it includes a Wasm inspection receipt binding the module ref, inspected imports, WIT ref, and allowed hostcalls
- AND invalid modules, stale receipts, unlisted imports, or ambient/WASI imports are rejected before side effects occur

#### Scenario: Wasm undeclared hostcall is rejected before effects
- GIVEN a Wasm actor whose preflight allows only a subset of hostcalls
- WHEN a suite step requests an undeclared hostcall
- THEN execution fails closed before side effects occur

#### Scenario: Reviewed Wasm hostcall actor executes under Wasmtime
- GIVEN a reviewed Wasm actor with valid module/WIT/allowed-hostcall preflight evidence
- WHEN an admitted hostcall step runs
- THEN the harness instantiates the core module with Wasmtime without WASI
- AND only `molten:hostcall/*` imports declared by preflight are linked
- AND the actor must export the operation entrypoint used for that hostcall
- AND execution is bounded by deterministic fuel and memory limits
- AND a canonical Wasm execution receipt is recorded before runtime state changes

#### Scenario: Ambient IO attempt is rejected
- GIVEN a non-native executor attempts filesystem, network, clock, random, or process access outside declared hostcalls
- WHEN execution runs
- THEN the runtime fails closed and records an executor-boundary diagnostic

### Requirement: Executor preflight is mandatory
r[molten.runtime.executor_hostcall_boundary.shell_admission] Steel, Wasm, adapter-backed, and remote-proxy actor kinds MUST remain fail-closed until executor preflight receipts validate.

#### Scenario: Unsupported executor kind remains blocked
- GIVEN an actor registry containing a Wasm actor without Wasm preflight evidence
- WHEN a suite runs
- THEN execution is rejected before side effects occur

#### Scenario: Stale preflight receipt is rejected
- GIVEN an actor module changed after preflight
- WHEN execution runs with the stale preflight receipt
- THEN execution fails closed before the actor can emit hostcalls

### Requirement: Replay validates hostcalls
r[molten.runtime.executor_hostcall_boundary.conformance] Replay MUST compare hostcall requests, decisions, and outputs for non-native actors exactly.

#### Scenario: Cross-kind conformance profile binds identical hostcalls
- GIVEN native, reviewed Steel, and reviewed Wasm actors that request the same hostcall operations over identical Preserves inputs
- WHEN executor preflight evidence is emitted
- THEN each actor binds the same executor conformance suite ref for the shared hostcall profile
- AND deterministic runs over the same actor id and inputs produce the same final runtime state across actor kinds

#### Scenario: Hostcall replay divergence is reported
- GIVEN a report whose hostcall decision was tampered
- WHEN replay runs
- THEN replay emits a hostcall-decision divergence diagnostic
