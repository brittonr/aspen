# Testing harness deltas

## ADDED Requirements

### Requirement: Missing descriptor bridge is classified as unsupported
r[molten.testing.descriptor_bridge_unsupported] When the test-workspace descriptor bridge (`/proc/self/fd`) is missing or unusable, the workspace helper MUST return the `Unsupported` error class on every host, not the raw filesystem error.

#### Scenario: Unix host without a descriptor bridge
- GIVEN a Unix host where `/proc/self/fd/<descriptor>` cannot be resolved
- WHEN the workspace helper resolves a descriptor path
- THEN it MUST return an `Unsupported` error.

#### Scenario: Host with a descriptor bridge
- GIVEN a host where `/proc/self/fd/<descriptor>` resolves
- WHEN the workspace helper resolves a descriptor path
- THEN it MUST return the resolved path.
