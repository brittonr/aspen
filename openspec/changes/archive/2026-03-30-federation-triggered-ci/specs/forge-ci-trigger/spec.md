## MODIFIED Requirements

### Requirement: Node startup wires Forge-CI integration

Startup SHALL also scan for federation mirrors and auto-watch them when federation_ci_enabled is true.

#### Scenario: Both features enabled with federation CI

- **WHEN** node starts with forge+ci and federation_ci_enabled=true
- **THEN** TriggerService SHALL scan and watch federation mirrors
