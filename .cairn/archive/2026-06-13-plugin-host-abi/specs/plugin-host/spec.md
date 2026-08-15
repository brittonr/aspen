# Plugin Host Delta: Host ABI Contract

### Requirement: Host ABI artifacts are canonical
r[molten.host_abi.artifact_model] Molten MUST represent plugin host ABI boundaries with canonical plugin manifests that bind plugin id, artifact ref, ABI id/version, lifecycle callbacks, effect manifest refs, hostcall refs, schema refs, policy refs, resource refs, and supply-chain evidence refs. The manifest MUST remain an install/activation input and MUST NOT grant authority by itself.

#### Scenario: Manifest binds ABI and effects
- GIVEN a plugin manifest for an artifact-backed plugin
- WHEN Molten parses the manifest
- THEN the canonical manifest ref binds the ABI version, lifecycle callbacks, effect manifest refs, hostcall refs, schemas, policy refs, resource refs, and supply-chain refs
- AND activation still requires separate permission and executor/effect receipts.

### Requirement: Host ABI results are canonical Preserves values
r[molten.host_abi.preserves_results] Molten MUST encode plugin host ABI results as canonical Preserves `plugin-host-abi-result-v1` values with the supported ABI schema, status, optional payload ref, and explicit error text. Richer redaction metadata, stable error class catalogs, or retry/idempotency guidance MUST be carried by referenced receipts or future admitted extensions and MUST NOT be inferred from ad hoc strings.

#### Scenario: Error result is explicit
- GIVEN a plugin ABI result with status `error`
- WHEN Molten renders the result
- THEN the result contains explicit error data
- AND no success payload is treated as authority or policy evidence.

### Requirement: Aspen RPC shape is non-normative
r[molten.host_abi.no_aspen_rpc_shape] Molten MUST document Aspen's plugin ABI discipline as prior art only and MUST NOT adopt Aspen JSON/postcard RPC enum compatibility as a Molten host ABI promise.

#### Scenario: ABI records use Preserves
- GIVEN a Molten plugin host ABI record
- WHEN an operator inspects it
- THEN it is a canonical Preserves record
- AND it does not claim Aspen RPC compatibility.

### Requirement: ABI version is receipt-bound
r[molten.host_abi.version_receipts] Plugin install, permission, lifecycle, hostcall, health, removal, and upgrade receipts MUST bind plugin and manifest refs whose manifest carries the ABI id/version, and host ABI result values MUST name the supported ABI schema.

#### Scenario: Hostcall receipt binds manifest version
- GIVEN a plugin hostcall receipt
- WHEN the receipt is inspected with the bound manifest
- THEN the ABI version used for the call is recoverable from the manifest ref
- AND unsupported ABI versions are rejected when the manifest is parsed.

### Requirement: Lifecycle callbacks are declared
r[molten.host_abi.lifecycle_exports] Molten MUST require plugin manifests to declare supported lifecycle callbacks before they are invoked. The completed initial callback set is `init`, `start`, `health`, `stop`, `remove`, and `upgrade`; richer `artifact_info`, `handle_turn`, `handle_request`, timer, or event callbacks remain future extensions unless admitted in a later ABI version.

#### Scenario: Undeclared callback is denied
- GIVEN a plugin manifest that does not declare a lifecycle operation
- WHEN Molten attempts to emit a lifecycle receipt for that operation
- THEN the receipt decision is denied before the callback can grant authority or perform side effects.

### Requirement: Hostcalls use admitted effect boundaries
r[molten.host_abi.effect_hostcalls] Plugin hostcalls MUST be represented by declared hostcall refs and MUST be admitted through executor and effect-handle receipt refs before side effects. The current fixture covers declared `storage.read` and ambient `network.open` denial; broader send, assert, retract, observe, blob, storage, trace, clock, and random wrappers require explicit declared hostcall/effect manifests and do not become ambient runtime APIs.

#### Scenario: Ambient hostcall is denied
- GIVEN a plugin manifest that declares only a storage read hostcall
- WHEN the plugin attempts an undeclared network hostcall
- THEN Molten emits a deny hostcall receipt
- AND no network side effect is admitted.

### Requirement: Namespace and resource isolation are checked per call
r[molten.host_abi.namespace_isolation] Plugin activation, lifecycle, and hostcall receipts MUST require explicit authority, policy, resource, supply-chain, and effect-boundary refs as applicable, and missing refs MUST deny before side effects.

#### Scenario: Missing authority denies activation
- GIVEN a plugin manifest with policy, resource, and supply-chain refs
- WHEN activation lacks authority or effect-boundary evidence
- THEN Molten emits a deny permission receipt
- AND install evidence is not treated as runtime authority.

### Requirement: Supervision receives callback failures
r[molten.host_abi.supervision_integration] Plugin health, lifecycle failure, stop, removal, and cleanup receipts MUST isolate plugin failures and bind service-supervision or cleanup evidence rather than corrupting node state.

#### Scenario: Failed health is isolated
- GIVEN a plugin health check reports failure
- WHEN Molten emits the health receipt
- THEN the decision is denied with diagnostics
- AND cleanup/supervision requirements are explicit evidence.

### Requirement: ABI compatibility gates upgrades
r[molten.host_abi.compatibility] Plugin upgrades MUST compare plugin id, ABI version, retained schema refs, rollback refs, and cleanup refs before replacing an active manifest.

#### Scenario: ABI mismatch denies upgrade
- GIVEN an installed plugin manifest and a replacement manifest with a different ABI
- WHEN upgrade validation runs
- THEN Molten emits a deny upgrade receipt
- AND the active manifest remains unchanged.

### Requirement: Initial Wasm binding is Preserves-first
r[molten.host_abi.wasm_binding_plan] The first Molten host ABI binding MUST be the primitive canonical Preserves/receipt interface used by reviewed Wasm, Steel, and native-adapter executor boundaries. WIT/component bindings MAY be added later as adapters around the same Preserves/effect admission contract.

#### Scenario: Component binding is not required for ABI evidence
- GIVEN a plugin lifecycle or hostcall receipt
- WHEN Molten validates the receipt
- THEN the evidence is canonical Preserves and executor/effect receipt refs
- AND it does not require a WIT/component transport shape.

### Requirement: Hostcall denial is tested
r[molten.host_abi.hostcall_tests] Molten SHOULD test that undeclared or unauthorized plugin hostcalls, raw host paths, missing artifact refs, stale supply-chain evidence, failed health, and incomplete cleanup deny before side effects.

#### Scenario: Unauthorized hostcall test denies
- GIVEN the plugin hostcall test invokes an undeclared ambient hostcall
- WHEN the receipt is parsed
- THEN the decision is `deny`
- AND diagnostics identify the ambient hostcall boundary.

### Requirement: Host ABI properties are deterministic
r[molten.host_abi.property_tests] Molten SHOULD include bounded property tests for manifest/ref determinism, lifecycle receipt determinism, authority gating, result encoding stability, and no-ambient-access invariants.

#### Scenario: Manifest ref is deterministic
- GIVEN generated bounded lifecycle callback sets
- WHEN Molten renders and reparses the plugin manifest
- THEN the manifest ref is stable
- AND missing authority remains denied.

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
