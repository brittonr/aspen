## ADDED Requirements

### Requirement: State-machine proof traces have a bounded contract
r[molten.testing.state_machine_proof.trace_contract] Molten MUST define a bounded proof trace step contract for state-machine evidence that binds before-state refs, transition or command refs, after-state refs, predicate or check names, decisions, diagnostics, and receipt refs.

#### Scenario: Trace step binds state and receipt evidence
- GIVEN a state-machine proof trace step
- WHEN Molten validates the step contract
- THEN the step identifies the prior state ref, transition or command ref, resulting state ref, decision, diagnostics, and receipt ref
- AND the step remains bounded for deterministic replay.

### Requirement: State-machine proof traces replay validate
r[molten.testing.state_machine_proof.trace_validator] Molten MUST validate state-machine proof traces by checking each step's receipt bindings and by ensuring adjacent steps chain through matching state refs.

#### Scenario: Valid proof trace replays
- GIVEN a proof trace whose steps have valid receipt refs and matching adjacent state refs
- WHEN Molten replay-validates the trace
- THEN validation passes
- AND the summary binds the accepted step count and final state ref.

### Requirement: State-machine proof trace validation fails closed
r[molten.testing.state_machine_proof.trace_validator_negative] Molten MUST reject state-machine proof traces with missing receipts, tampered diagnostics, stale before-state refs, wrong after-state refs, or out-of-order steps.

#### Scenario: Tampered trace denies
- GIVEN a proof trace whose receipt diagnostics or adjacent state refs have been modified
- WHEN Molten replay-validates the trace
- THEN validation denies the trace
- AND diagnostics identify the first invalid proof binding.
