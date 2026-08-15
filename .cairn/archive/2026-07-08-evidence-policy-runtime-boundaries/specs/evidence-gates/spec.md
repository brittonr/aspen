# Evidence Gates Delta: Evidence, Policy, Runtime Boundaries

### Requirement: Evidence, policy, runtime, and adapters have explicit ownership
r[molten.modularity.layer_boundaries.ownership] Evidence, policy, runtime, and adapter modules SHOULD have documented ownership boundaries before code is extracted across crate boundaries.

#### Scenario: Workflow ownership is reviewable
- GIVEN a workflow selected for modularity cleanup
- WHEN reviewers inspect its source organization or design notes
- THEN evidence construction, policy admission, runtime planning, and adapter effects are each assigned to an owning layer

#### Scenario: Ambiguous ownership blocks extraction
- GIVEN a workflow mixes evidence construction, policy decisions, runtime state transitions, and adapter IO in one implementation surface
- WHEN crate extraction is proposed
- THEN the extraction is blocked or staged until ownership is clarified

### Requirement: Evidence does not grant policy authority
r[molten.modularity.layer_boundaries.evidence_policy_split] Evidence construction and verification MUST remain separate from policy authority decisions; evidence-only receipts MUST NOT by themselves grant authority, resource rights, provenance trust, transport trust, retention authority, or execution permission.

#### Scenario: Policy consumes evidence summary
- GIVEN verified evidence inputs and explicit policy, authority, resource, or provenance refs
- WHEN policy admission evaluates a request
- THEN it may use evidence summaries as inputs while producing its own pass or deny decision

#### Scenario: Evidence-only input is denied as authority
- GIVEN a request presents only an evidence receipt without the required authority, policy, resource, provenance, retention, or execution admission input
- WHEN policy admission evaluates the request
- THEN the request is denied before side effects occur

### Requirement: Runtime planning stays separate from adapter effects
r[molten.modularity.layer_boundaries.runtime_adapter_split] Runtime modules SHOULD consume admitted policy/evidence results and return deterministic state transitions or planned effects; adapter modules perform IO only after the runtime plan is admitted.

#### Scenario: Runtime returns planned effect
- GIVEN admitted policy and evidence inputs for a runtime operation
- WHEN the runtime core evaluates the operation
- THEN it returns deterministic state transitions, traces, receipts, or planned effects without directly performing adapter IO

#### Scenario: Adapter availability is not trust
- GIVEN an adapter is reachable, a transport identity is observed, or a store contains an artifact
- WHEN runtime or policy admission evaluates a trust-boundary action
- THEN availability or presence alone does not grant authority, policy, resource, provenance, retention, execution, or replay trust

### Requirement: Layer boundary changes include denial tests
r[molten.modularity.layer_boundaries.tests] Layer-boundary refactors SHOULD include positive tests for admitted flow and negative tests for evidence-only authority, stale policy inputs, adapter availability-as-trust, and denied side effects.

#### Scenario: Boundary test matrix is reviewable
- GIVEN a workflow boundary is refactored
- WHEN reviewers inspect test evidence
- THEN the tests cover at least one admitted flow and at least one denial where evidence, policy, runtime, or adapter responsibilities could otherwise be confused
