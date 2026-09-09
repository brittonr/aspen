# Fabric Simulation Delta

## ADDED Requirements

### Requirement: Differential conformance compares executed workloads
r[molten.fabric_simulation.live_differential] A live-simulation differential MUST execute one shared bounded workload against a simulated adapter and a live adapter through their declared port contracts. The comparison MUST evaluate permitted outcome classes and declared ordering properties and MUST NOT depend on physical timing or non-canonical physical bytes.

#### Scenario: Outcome classes agree
- GIVEN a shared workload executed against both adapters
- WHEN each observed outcome normalizes to its declared permitted class
- THEN the differential reports agreement with both recorded histories.

#### Scenario: Live adapter emits an unpermitted class
- GIVEN a live adapter returns an outcome class outside its declared permitted set
- WHEN the differential evaluates the histories
- THEN the differential reports failure at the recorded step with both observed classes.

### Requirement: Differential results distinguish divergence from incompleteness
r[molten.fabric_simulation.differential_results] A differential MUST record permitted divergence and incomplete executions as typed results distinct from failure. An unsupported behavior or an unavailable adapter MUST produce an incomplete result, and incomplete results MUST NOT count as agreement.

#### Scenario: Adapter cannot execute a step
- GIVEN a live adapter does not support one workload step
- WHEN the differential completes
- THEN the step is recorded as incomplete and the overall result is not agreement.

#### Scenario: Permitted divergence stays typed
- GIVEN two outcomes in different but declared-permitted classes for one step
- WHEN the differential completes
- THEN the step records a permitted divergence with the class pair and the result is not a failure.

### Requirement: Execution and structural differentials bind separately
r[molten.fabric_simulation.differential_evidence] The descriptor-level structural differential and the executed-workload differential MUST remain separately identified artifacts. Evidence MUST bind both sides' identities, the shared workload, permitted-outcome decisions, incomplete observations, and the applicable non-claims.

#### Scenario: Evidence identifies both artifacts
- GIVEN a completed execution differential and its structural companion
- WHEN the evidence bundle serializes
- THEN each artifact has its own identity and both reference the shared workload and the non-claims.
