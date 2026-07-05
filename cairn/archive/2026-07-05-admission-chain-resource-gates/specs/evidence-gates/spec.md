# Evidence Gates Delta: admission chain resource gates

### Requirement: Resource admission emits ordered chain receipts
r[molten.resource_admission.ordered_chain_receipts] Molten MUST gate resource create, update, status, delete, and reconcile-apply intents through an ordered admission chain that binds envelope decode, schema validation, authority preflight, defaulting, reviewed mutation, final validation, policy/evidence gates, and commit-plan receipt refs. A later phase MUST NOT claim success when an earlier phase denied.

#### Scenario: Admitted resource change records every phase
- GIVEN a valid resource update with schema refs, authority evidence, reviewed defaulting refs, policy refs, and final validation evidence
- WHEN resource admission evaluates the update
- THEN Molten emits an admission pass receipt binding each phase result in order
- AND the shell may persist only the candidate bound by the receipt.

#### Scenario: Missing phase evidence denies commit
- GIVEN a resource update that has final validation evidence but no authority preflight evidence
- WHEN resource admission evaluates the update
- THEN Molten denies the commit plan before persistence
- AND diagnostics identify the missing phase evidence.

### Requirement: Mutation requires reviewed rule evidence
r[molten.resource_admission.mutation_requires_reviewed_rule] Molten MUST allow admission defaulting or mutation only when deterministic reviewed rule refs explain the transformation from pre-mutation candidate ref to post-mutation candidate ref. Molten MUST deny mutation claims that depend on clocks, ambient state, unreviewed code, or missing rule evidence.

#### Scenario: Reviewed mutation is admitted
- GIVEN a candidate resource and a reviewed mutation rule that deterministically adds an allowed default field
- WHEN admission evaluates the mutation phase
- THEN the pass receipt binds the rule ref, pre-mutation ref, and post-mutation ref.

#### Scenario: Unreviewed mutation denies
- GIVEN a candidate resource whose post-mutation ref differs from its pre-mutation ref without reviewed rule evidence
- WHEN admission evaluates the mutation phase
- THEN Molten denies before final validation or persistence.

### Requirement: Status operations cannot mutate desired state
r[molten.resource_admission.status_subresource_isolated] Molten MUST isolate status operations so they can update observed-state refs and status conditions for an observed generation, but MUST NOT advance desired generation, change desired-state refs, alter finalizers, or alter authority-bearing metadata.

#### Scenario: Status update records observation
- GIVEN a controller observation for the current generation with condition evidence and an observed-state ref
- WHEN the controller submits a status operation
- THEN admission accepts only the status condition and observed-state changes.

#### Scenario: Status update attempts desired mutation
- GIVEN a status operation that also changes desired-state ref, desired generation, finalizers, or authority-bearing metadata
- WHEN admission evaluates the operation
- THEN Molten denies the operation
- AND the desired resource record remains unchanged.
