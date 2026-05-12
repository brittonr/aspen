## ADDED Requirements

### Requirement: VM plugin fixtures track host ABI [r[plugins.vm-fixtures-track-host-abi]]

Plugin fixtures used by VM acceptance checks MUST remain compatible with Aspen's current host ABI or be isolated from unrelated cluster-formation assertions.

#### Scenario: Forge plugin fixture compiles against current metadata APIs [r[plugins.vm-fixtures-track-host-abi.forge-metadata]]

- GIVEN a VM check includes a WASM forge plugin fixture
- WHEN the fixture is built for the check
- THEN its use of forge metadata types such as `ForgeRepoInfo` and plugin metadata types such as `PluginInfo` SHALL match the current host API
- AND API drift SHALL be fixed at the fixture boundary rather than hidden by disabling the whole cluster check

#### Scenario: Optional plugin fixture drift does not block core cluster proof [r[plugins.vm-fixtures-track-host-abi.optional-isolation]]

- GIVEN a VM check includes optional plugin or forge subtests after core cluster formation
- WHEN the optional fixture cannot build or load because of plugin ABI drift
- THEN the check or its reporting SHALL preserve the core cluster proof boundary separately
- AND the optional fixture failure SHALL be diagnosed as plugin/fixture drift, not as Raft/Iroh clustering failure
