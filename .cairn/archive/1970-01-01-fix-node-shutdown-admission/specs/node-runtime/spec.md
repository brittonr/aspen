## ADDED Requirements

### Requirement: Shutdown admission precedes effects
r[molten.audit_f01.admission] Node shutdown MUST obtain a passing admission decision before adapter shutdown, successful shutdown receipt publication, or active-lock removal.

#### Scenario: Admitted shutdown executes
- GIVEN a current active node and valid shutdown evidence
- WHEN shutdown admission passes
- THEN the shell executes the admitted shutdown plan

#### Scenario: Missing authority denies
- GIVEN an active node and a shutdown request with `authority_refs=[]`
- WHEN dispatch evaluates admission
- THEN dispatch denies before any shutdown effect

### Requirement: Rejection preserves lifecycle state
r[molten.audit_f01.preserve_state] A denied shutdown MUST preserve the active lock, startup evidence, adapter state, and existing successful shutdown evidence.

#### Scenario: Separate denial evidence remains diagnostic
- GIVEN a denied shutdown request
- WHEN the shell records denial evidence
- THEN the evidence describes rejection without changing active lifecycle state

#### Scenario: Missing policy cannot stop the node
- GIVEN an active node and a shutdown request with empty policy or resource references
- WHEN admission denies
- THEN the active lock and successful lifecycle artifacts remain unchanged

### Requirement: Receipts distinguish plans from observations
r[molten.audit_f01.observed_effects] Shutdown receipts MUST distinguish admission, observed effect completion, and failed or uncertain effects without treating a plan as completed shutdown.

#### Scenario: Completed shutdown has observed evidence
- GIVEN an admitted shutdown whose required effects complete
- WHEN the shell publishes success
- THEN the receipt binds the request and observed shutdown effects

#### Scenario: Adapter error cannot become success
- GIVEN an admitted shutdown whose adapter reports an error
- WHEN the shell records the outcome
- THEN the outcome does not claim complete shutdown from admission alone

### Requirement: Validation preserves evidence limits
r[molten.audit_f01.validation] F01 validation MUST include normal repository regressions, rejection state comparisons, adapter ordering tests, and explicit executed-versus-static evidence labels.

#### Scenario: Executed regression supplies scoped evidence
- GIVEN accepted and rejected shutdown tests in the normal repository suite
- WHEN the implementation checks execute
- THEN their results identify the source revision and tested boundaries

#### Scenario: Static review is not execution evidence
- GIVEN only the original static F01 analysis
- WHEN a completion receipt describes validation
- THEN it does not claim an executed shutdown reproduction or live-node correctness
