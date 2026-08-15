# Evidence Gates Delta: Retention Core and Store Boundary

### Requirement: Retention responsibilities are semantically separated
r[molten.retention.modularity.boundaries] Retention implementation SHOULD separate destructive admission, GC planning, plan application, audit, store persistence, bundle export, live remote-clearance transport, and receipt construction into reviewable boundaries.

#### Scenario: Retention module ownership is clear
- GIVEN retention code is reorganized
- WHEN reviewers inspect the module layout
- THEN each module has an identifiable responsibility such as admission, plan, apply, audit, store, bundle, live, or receipts

### Requirement: Destructive retention side effects require explicit admitted plans
r[molten.retention.modularity.destructive_plan] Retention shells MUST NOT delete, tombstone, redact, compact, unpin, or import remote-clearance evidence as authoritative unless a pure retention decision returns an admitted plan for the exact object, action, class, and evidence scope.

#### Scenario: Admitted destructive plan permits shell mutation
- GIVEN authority, policy, supporting evidence, reference-index, and remote-clearance inputs satisfy retention admission
- WHEN the pure retention planner evaluates the request
- THEN it returns an admitted plan that the shell may execute while recording canonical evidence

#### Scenario: Missing retention evidence denies mutation
- GIVEN authority, policy, supporting evidence, reference-index, remote-clearance, or class/action scope is missing or stale
- WHEN the pure retention planner evaluates the request
- THEN it returns a deny result with no destructive operation in the planned effects

### Requirement: Retention store IO stays in shell boundary
r[molten.retention.modularity.store_shell] Retention filesystem traversal, evidence-store writes, bundle directory writes, and live transport IO MUST be owned by shell or adapter modules rather than pure retention admission cores.

#### Scenario: Store shell loads evidence before core
- GIVEN retention evidence lives in a local store
- WHEN a retention command runs
- THEN the shell loads typed evidence summaries and passes them to the pure retention core

### Requirement: Retention modularity has positive and negative tests
r[molten.retention.modularity.tests] Retention boundary refactors SHOULD include positive tests for admitted plans and negative tests for missing authority, stale plan, plan drift, incomplete reference index, missing remote clearance, or overbroad evidence.

#### Scenario: Denied retention case has no planned side effect
- GIVEN a negative retention fixture
- WHEN the pure planner evaluates it
- THEN it denies before destructive side effects and returns an empty destructive-effect plan
