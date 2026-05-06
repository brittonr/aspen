## ADDED Requirements

### Requirement: Hyperlight Runtime Runner [r[runtime-host-loading.hyperlight-runner]]
Aspen MUST provide a Hyperlight runner contract for isolated runtime units that are compatible with the Hyperlight host boundary.

#### Scenario: Hyperlight runner capability is advertised [r[runtime-host-loading.hyperlight-runner.capability]]
- GIVEN an Aspen node includes a compatible Hyperlight runtime
- WHEN the node reports runtime runner capabilities
- THEN it SHALL advertise Hyperlight support, runner version, supported ABI profiles, resource limits, and artifact profiles

#### Scenario: Hyperlight artifact is verified before start [r[runtime-host-loading.hyperlight-runner.artifact-verification]]
- GIVEN a runtime unit declares a Hyperlight image or program artifact
- WHEN the runner prepares to start the unit
- THEN it SHALL verify content identity, ABI compatibility, and resource policy before exposing host calls or capability handles

#### Scenario: Host ABI exposes only declared capabilities [r[runtime-host-loading.hyperlight-runner.capability-binding]]
- GIVEN a Hyperlight unit requests Aspen substrate access
- WHEN the runner constructs the host ABI
- THEN it SHALL expose only declared capability-scoped handles for KV, blob, logging, metrics, timers, routes, or outputs

#### Scenario: Hyperlight admission fails closed [r[runtime-host-loading.hyperlight-runner.fail-closed]]
- GIVEN a Hyperlight unit has an invalid artifact, unsupported ABI, missing runner capability, or denied capability binding
- WHEN admission evaluates the unit
- THEN Aspen SHALL reject the assignment before start and SHALL emit a redacted rejection receipt

#### Scenario: Hyperlight output is receipt-backed [r[runtime-host-loading.hyperlight-runner.outputs]]
- GIVEN a Hyperlight unit exits or emits declared outputs
- WHEN the runner finalizes the attempt
- THEN outputs SHALL be stored as Aspen-approved artifacts or receipt fields and SHALL include the verified input artifact identity and runner identity
