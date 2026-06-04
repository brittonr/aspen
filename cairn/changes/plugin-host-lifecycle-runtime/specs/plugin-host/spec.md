# Plugin Host Delta: Lifecycle Runtime

### Requirement: Plugins are artifact-backed and permission-gated
r[molten.plugin_host_lifecycle.spec.install_gate] A plugin MUST install only from an admitted artifact-backed manifest with explicit ABI, schemas, effect manifests, hostcalls, policy, resource, and supply-chain evidence.

#### Scenario: Valid plugin installs
- GIVEN a plugin manifest referencing an admitted artifact, ABI, effect manifest, and policy refs
- WHEN install validation runs
- THEN Molten emits a plugin install receipt with decision `pass`
- AND activation remains separate from install authority

#### Scenario: Raw host path denied
- GIVEN a plugin manifest that identifies executable code by host path or process command
- WHEN install validation runs
- THEN Molten emits a denial receipt
- AND no plugin code is loaded

### Requirement: Runtime hostcalls use the effect boundary
r[molten.plugin_host_lifecycle.spec.hostcalls] Plugin hostcalls MUST be declared in the plugin manifest and admitted through the executor/effect-handle boundary before side effects.

#### Scenario: Declared hostcall passes
- GIVEN an active plugin with a declared storage read hostcall and matching authority/resource refs
- WHEN the plugin invokes the hostcall
- THEN Molten emits a plugin hostcall receipt binding the executor and effect receipts

#### Scenario: Ambient network hostcall denied
- GIVEN a plugin without a network hostcall declaration
- WHEN the plugin attempts network access
- THEN the hostcall is denied before any network side effect

### Requirement: Lifecycle cleanup is complete
r[molten.plugin_host_lifecycle.spec.cleanup] Plugin stop, removal, failed health, or authority revocation MUST clean up plugin-owned services, assertions, handles, and catalog entries with receipts.

#### Scenario: Remove retracts owned refs
- GIVEN an active plugin with owned service assertions and effect handles
- WHEN the plugin is removed
- THEN cleanup receipts bind all retractions and handle revocations

### Requirement: Plugin upgrades are compatibility-gated
r[molten.plugin_host_lifecycle.spec.upgrade] Plugin upgrades MUST emit compatibility, rollback, and cleanup evidence before replacing an active manifest.

#### Scenario: Compatible upgrade passes
- GIVEN an installed plugin and a replacement manifest with the same plugin id, compatible ABI, and retained schema refs
- WHEN upgrade validation runs
- THEN Molten emits a plugin upgrade receipt with decision `pass`
- AND the receipt binds rollback and cleanup evidence

#### Scenario: ABI mismatch denies upgrade
- GIVEN an installed plugin and a replacement manifest with an incompatible ABI
- WHEN upgrade validation runs
- THEN Molten emits a plugin upgrade receipt with decision `deny`
- AND the active plugin manifest remains unchanged
