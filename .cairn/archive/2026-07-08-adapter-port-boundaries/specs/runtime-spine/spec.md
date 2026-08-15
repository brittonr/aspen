# Runtime Spine Delta: Adapter Port Boundaries

### Requirement: Effecting subsystems expose explicit ports or plans
r[molten.modularity.adapter_ports.explicit_ports] Runtime subsystems that need storage, transport, execution, policy, clock, seed, or effect-log interaction SHOULD express those interactions through explicit ports, deterministic plan records, or input/output structs rather than direct hidden calls from pure decision logic.

#### Scenario: Planner returns adapter plan
- GIVEN a runtime workflow that may write storage, publish transport data, execute code, or record effect evidence
- WHEN the pure planner evaluates valid admitted inputs
- THEN it returns a structured decision and planned adapter operations without performing the side effect itself

#### Scenario: Hidden side effect is rejected
- GIVEN a module identified as pure planning or admission logic
- WHEN reviewers inspect the implementation
- THEN direct filesystem mutation, network publication, executor invocation, clock reads, environment reads, or process execution are absent or moved behind the shell boundary

### Requirement: Admission precedes adapter effects
r[molten.modularity.adapter_ports.admission_before_effects] Adapter shells MUST execute mutation, transport, execution, or persistence effects only after the pure planner or admission gate returns a pass decision for the requested operation.

#### Scenario: Passing plan executes in shell
- GIVEN a planner returns a pass decision with planned adapter operations
- WHEN the shell executes the plan
- THEN effects are performed through the declared adapter boundary and canonical outcome evidence is recorded

#### Scenario: Denied plan does not mutate
- GIVEN missing authority, stale evidence, malformed input, resource denial, or unsupported adapter capability
- WHEN the planner evaluates the request
- THEN it returns a deny or unavailable decision with no mutation, transport publication, executor invocation, or destructive operation in the planned operations

### Requirement: Adapter outcomes are evidence-bearing
r[molten.modularity.adapter_ports.effect_receipts] Adapter shells SHOULD record canonical evidence for performed, denied, unavailable, replayed, or failed adapter outcomes when the outcome affects deterministic replay, admission review, or release evidence.

#### Scenario: Performed effect has receipt
- GIVEN an admitted adapter operation is performed
- WHEN the operation completes
- THEN the shell records evidence binding the plan, adapter profile, operation identity, relevant refs, and outcome

#### Scenario: Failed adapter does not grant trust
- GIVEN an adapter operation fails because a service, transport, store, executor, or environment is unavailable
- WHEN the shell records the outcome
- THEN the evidence marks the operation unavailable or failed and does not convert availability into authority, policy, provenance, resource, or replay trust

### Requirement: Adapter port changes carry positive and negative tests
r[molten.modularity.adapter_ports.tests] Adapter port refactors SHOULD include positive plan/execution tests and negative denial tests for the extracted boundary.

#### Scenario: Positive and negative port tests exist
- GIVEN a port boundary is introduced or changed
- WHEN reviewers inspect the test evidence
- THEN admitted inputs and denied inputs are both covered, including proof that denied inputs do not produce planned side effects
