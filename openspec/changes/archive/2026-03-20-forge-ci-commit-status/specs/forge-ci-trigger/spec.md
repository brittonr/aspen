## MODIFIED Requirements

### Requirement: Node startup wires Forge-CI integration

When both `forge` and `ci` features are enabled, the node startup sequence SHALL create `ForgeConfigFetcher`, `OrchestratorPipelineStarter`, `TriggerService`, and register `CiTriggerHandler` with the Forge gossip service. It SHALL also create a `ForgeStatusReporter` and pass it to the `PipelineOrchestrator`.

#### Scenario: Both features enabled

- **WHEN** a node starts with both `forge` and `ci` features enabled
- **THEN** the `TriggerService` SHALL be created and registered with Forge gossip
- **AND** a `ForgeStatusReporter` SHALL be created with access to the KV store and gossip service
- **AND** the `PipelineOrchestrator` SHALL be constructed with the `ForgeStatusReporter`
- **AND** the trigger service SHALL be ready to process push events

#### Scenario: Only one feature enabled

- **WHEN** a node starts with only `ci` (no `forge`) or only `forge` (no `ci`)
- **THEN** no Forge-CI integration SHALL be wired
- **AND** the `PipelineOrchestrator` SHALL be constructed without a `StatusReporter`
- **AND** no errors SHALL occur during startup
