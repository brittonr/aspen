## ADDED Requirements

### Requirement: Bounded retained client input

r[molten.chaoscontrol_client.bounded_input]

The client MUST validate a versioned finite topology, command vocabulary, request identities, payload limit and recovery budget before effects. It MUST retain canonical input and reduce only prerequisite-valid prefixes bound to the original input.

#### Scenario: Invalid prefix

- GIVEN a reopen without a preceding crash
- WHEN input is admitted
- THEN admission MUST reject before effects

### Requirement: Real native execution

r[molten.chaoscontrol_client.real_execution]

WorkloadExecutor MUST call admitted production operations through the existing live-process harness and MUST record command execution separately from semantic observations. Expected arrays or a successful no-op MUST NOT qualify.

#### Scenario: Native baseline

- GIVEN a retained supported scenario
- WHEN the native driver executes
- THEN independent participant readbacks MUST determine the result

### Requirement: Independent recovery observations

r[molten.chaoscontrol_client.observation]

The checker MUST retain fault intent/application, storage application/sync/unknown/quarantine, local/peer/absent recovery and every participant fault-off horizon separately. Missing observations MUST remain incomplete or not evaluated.

#### Scenario: Short recovery

- GIVEN one eligible participant has a short observation horizon
- WHEN aggregate progress otherwise succeeds
- THEN the client MUST NOT qualify the cohort

### Requirement: Attributable semantic controls

r[molten.chaoscontrol_client.controls]

A semantic control MUST separately bind deployment, reached divergence and intended property rejection in one history. Suppression MUST remove detection or invalidate qualification. Healthy, not reached, survived, unsupported and incomplete outcomes MUST remain distinct.

#### Scenario: Duplicate application

- GIVEN an admitted request is deliberately applied twice through the actual handler
- WHEN apply-once observes duplicate request identities
- THEN only that property MUST count detection

### Requirement: Guest dependency boundary

r[molten.chaoscontrol_client.guest_boundary]

The client MUST use the existing ChaosControl guest intake, require a single-vCPU supported profile and bind exact source, binary/image, kernel, adapter, oracle, schema and dependency closure identities. Native execution MUST NOT satisfy missing guest evidence.

#### Scenario: Missing guest closure

- GIVEN guest binary or dependency identity is unavailable
- WHEN guest qualification is requested
- THEN it MUST remain explicitly blocked

### Requirement: Executed verification only

r[molten.chaoscontrol_client.verification]

Reports MUST identify actual commands, retained artifacts, bounds and non-claims; an independent repeat MUST bind the original immutable binary and property identity. Changed candidates MUST use new build bindings.

#### Scenario: Unexecuted campaign

- GIVEN only source fixtures exist
- WHEN client status is reported
- THEN no guest campaign or replay verdict MUST be claimed
