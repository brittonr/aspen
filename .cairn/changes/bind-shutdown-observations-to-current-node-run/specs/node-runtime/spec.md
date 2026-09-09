## ADDED Requirements

### Requirement: Duplicate results remain historical
r[molten.audit_f14.duplicate_scope] Shutdown duplicate suppression MUST remain separate from current stopped classification and MUST NOT automatically reexecute a historical request.

#### Scenario: Prior-run duplicate stays suppressed
- GIVEN shutdown request R completed in run A and run B is active
- WHEN run B receives exact request R
- THEN dispatch suppresses repeated shutdown effects and reports the historical result separately

#### Scenario: Historical success cannot stop the loop
- GIVEN run B retains its active lock after duplicate suppression
- WHEN the control loop reads the passing receipt from run A
- THEN that receipt alone does not set `has_stopped=true` or terminate the loop as stopped

### Requirement: Stopped observations bind the current run
r[molten.audit_f14.current_binding] Current stopped classification MUST require matching current-run shutdown observations and MUST distinguish separate runs with identical startup content.

#### Scenario: Current shutdown establishes stopped state
- GIVEN a newly admitted request completes shutdown for the current run
- WHEN the shell supplies matching lifecycle observations
- THEN the core can classify the current run as stopped

#### Scenario: Equal startup content does not merge runs
- GIVEN separate runs with identical canonical startup inputs
- WHEN a shutdown observation belongs only to the earlier run
- THEN the later run cannot use that observation as its own stopped evidence

### Requirement: Incomplete observations preserve state
r[molten.audit_f14.preserve_state] Missing, conflicting, or unreadable current-run observations MUST produce an unresolved or rejected classification without shutdown effects or active-lock mutation.

#### Scenario: Consistent active observations stay active
- GIVEN a current active lock and no current shutdown observation
- WHEN the core classifies a historical duplicate
- THEN current lifecycle status remains active

#### Scenario: Observation error cannot prove shutdown
- GIVEN an unreadable shutdown artifact or conflicting run binding
- WHEN the shell requests lifecycle classification
- THEN the result does not claim stopped state and preserves current lifecycle state

### Requirement: Compatibility and validation retain evidence scope
r[molten.audit_f14.validation] F14 validation MUST cover restart duplicates, current shutdown, legacy receipt interpretation, and adapter rejection while preserving executed-versus-static claim boundaries.

#### Scenario: Repository regression records bounded behavior
- GIVEN a normal repository test for shutdown R, restart, and duplicate R
- WHEN the test executes
- THEN it records duplicate suppression, current active state, and no false stopped observation

#### Scenario: Legacy receipt cannot gain a run binding
- GIVEN a historical receipt without sufficient current-run evidence
- WHEN compatibility or replay code reads it
- THEN it remains historical evidence and does not gain current shutdown or execution claims
