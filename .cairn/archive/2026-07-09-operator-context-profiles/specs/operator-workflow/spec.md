## ADDED Requirements

### Requirement: Operator context profile artifact
r[molten.operator_workflow.context_profile.artifact] Molten SHOULD define a canonical operator context profile artifact that groups reviewed policy refs, capability refs, authority refs, resource refs, evidence refs, redaction refs, retention refs, allowed operation scopes, and caveats for repeatable CLI workflows.

#### Scenario: Context profile groups reusable refs
- GIVEN an operator has a reviewed context profile for a node-control workflow
- WHEN the profile artifact is inspected
- THEN it lists the reusable refs and allowed operation scopes as canonical fields
- AND the profile ref is derived from canonical bytes.

#### Scenario: Malformed context profile denies
- GIVEN a context profile with malformed refs, duplicate contradictory scopes, or missing required metadata
- WHEN context validation runs
- THEN validation denies before command inputs are expanded.

### Requirement: Context profiles expand into explicit command refs
r[molten.operator_workflow.context_profile.expansion] Commands that accept an operator context profile MUST expand it into the explicit refs required by the existing command core before admission. The expanded refs MUST be the values evaluated by downstream authority, policy, resource, provenance, retention, source-gate, and transport gates.

#### Scenario: Valid context expands for command core
- GIVEN a command requires policy, authority, resource, and evidence refs
- AND a supplied context profile contains compatible refs for that operation
- WHEN the CLI shell expands the profile
- THEN the command core receives explicit ref lists equivalent to passing the refs directly
- AND the command receipt records the context profile ref and expanded refs.

#### Scenario: Unsupported operation scope denies
- GIVEN a context profile is scoped to catalog readback
- WHEN an operator tries to use it for retention deletion or live mutation
- THEN expansion denies before the mutation command core runs
- AND diagnostics name the unsupported operation scope.

### Requirement: Context profile overrides are bounded
r[molten.operator_workflow.context_profile.overrides] Molten MUST define deterministic override rules for merging explicit CLI refs with context-profile refs, and MUST deny overrides that remove required refs, exceed profile scope, or create ambiguous duplicate authority or policy context.

#### Scenario: Additive evidence override is recorded
- GIVEN a context profile supplies required policy and authority refs
- WHEN an operator adds supporting evidence refs explicitly on the CLI
- THEN expansion records the additive override and passes the combined explicit evidence refs to the command core.

#### Scenario: Conflicting authority override denies
- GIVEN a context profile binds one authority scope for a command
- WHEN an operator supplies a conflicting authority ref or broader scope override
- THEN expansion denies before side effects
- AND diagnostics identify the conflicting authority context.

### Requirement: Context profiles are convenience evidence only
r[molten.operator_workflow.context_profile.evidence_only] Operator context profile presence MUST NOT by itself grant authority, satisfy policy admission, prove resource sufficiency, prove source-gate or provenance trust, satisfy retention clearance, or authorize mutation.

#### Scenario: Profile-only authority attempt denies
- GIVEN a command receives a context profile artifact but profile expansion does not produce a valid authority ref for the requested operation
- WHEN the downstream gate evaluates admission
- THEN admission denies
- AND diagnostics state that context profile presence is not authority.
