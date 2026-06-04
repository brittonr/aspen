# Runtime Spine Delta: Steel VM executor

### Requirement: Steel VM execution is review-gated
r[molten.runtime.steel_vm_executor.review] Steel actor execution MUST require a validated review receipt binding source ref, callable name, allowed hostcalls, engine profile, sandbox profile, and conformance refs.

#### Scenario: Missing review receipt remains fail-closed
- GIVEN a Steel actor registry entry without a matching review receipt
- WHEN the harness attempts to execute the actor
- THEN execution is rejected before side effects occur

#### Scenario: Callable mismatch is rejected
- GIVEN a Steel review receipt for callable `a`
- WHEN the actor fixture requests callable `b`
- THEN Steel execution fails closed before VM instantiation

### Requirement: Steel values cross the runtime boundary as Preserves
r[molten.runtime.steel_vm_executor.preserves_bridge] The Steel executor MUST receive actor inputs and return actor outputs through deterministic Preserves conversion.

#### Scenario: Actor input ref is preserved
- GIVEN a reviewed Steel callable receives an admitted step
- WHEN the runtime invokes the callable
- THEN the callable input corresponds to the canonical `<actor-input-v1 ...>` value
- AND the input ref is recorded in the Steel execution receipt

#### Scenario: Invalid output is rejected
- GIVEN a Steel callable returns a value that cannot encode as the expected canonical actor-output schema
- WHEN the runtime validates the return value
- THEN execution fails closed before commit

### Requirement: Steel hostcalls use the runtime shell
r[molten.runtime.steel_vm_executor.hostcalls] Steel hostcall primitives MUST build canonical hostcall request envelopes and submit them to the same admission/effect/replay rails used by native and Wasm actors.

#### Scenario: Steel send hostcall is admitted normally
- GIVEN a reviewed Steel callable invokes the send primitive
- WHEN the primitive submits a hostcall request
- THEN policy, capability, budget, and effect-log checks run before the send is accepted
- AND the hostcall request/decision refs are recorded

#### Scenario: Undeclared hostcall fails closed
- GIVEN a Steel callable invokes a hostcall not listed in its review receipt
- WHEN execution reaches that primitive
- THEN execution fails closed before side effects occur

### Requirement: Steel VM has no ambient authority
r[molten.runtime.steel_vm_executor.sandbox] The Steel executor MUST NOT expose filesystem, network, process, environment, clock, random, dynamic loading, or unreviewed module authority to reviewed callables.

#### Scenario: Ambient IO token is unavailable at runtime
- GIVEN reviewed Steel source attempts to access filesystem or network primitives
- WHEN the VM is instantiated
- THEN those primitives are absent or disabled
- AND any attempted access fails closed as executor-boundary evidence

### Requirement: Steel execution is resource bounded and replayable
r[molten.runtime.steel_vm_executor.receipts] Steel execution receipts MUST bind input/output/hostcall refs, review refs, sandbox refs, and deterministic resource limits/usage so replay can compare execution exactly.

#### Scenario: Replay detects Steel output tampering
- GIVEN a report whose Steel actor-output envelope was modified
- WHEN replay recomputes Steel execution
- THEN replay reports a Steel execution divergence

#### Scenario: Resource exhaustion prevents commit
- GIVEN a Steel callable exceeds its deterministic fuel or allocation budget
- WHEN execution reaches the resource limit
- THEN execution fails closed before any staged runtime state is committed
