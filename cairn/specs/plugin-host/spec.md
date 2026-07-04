# Plugin Host Specification

## Purpose

Defines the `plugin-host` capability and the receipt-backed Molten plugin host ABI boundary for artifact-backed plugins, lifecycle callbacks, admitted hostcalls, health/cleanup, and compatibility-gated upgrades.

## Requirements

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

### Requirement: Plugin lifecycle transitions are receipt ordered
r[molten.plugin_lifecycle_state_proof.ordered_lifecycle] Molten MUST prove that plugin install, permission, activation, hostcall, health, upgrade, removal, and cleanup receipts are accepted only in an order that preserves the plugin manifest, ABI, policy, resource, effect, and supply-chain bindings.

#### Scenario: Hostcall before permission denies
- GIVEN a plugin manifest and hostcall request with no passing permission receipt
- WHEN the hostcall is evaluated
- THEN the hostcall receipt decision is `deny`
- AND the hostcall side effect is not executed.

### Requirement: Plugin health gates further lifecycle use
r[molten.plugin_lifecycle_state_proof.health_gate] Molten MUST prove that failed or stale health evidence blocks plugin upgrade, hostcall execution, and continued activation unless a later passing health or recovery receipt is bound.

#### Scenario: Failed health blocks upgrade
- GIVEN a plugin with a failed health receipt
- WHEN an upgrade receipt is requested without recovery evidence
- THEN upgrade decision is `deny`
- AND diagnostics identify failed health evidence.

### Requirement: Plugin removal and cleanup close authority
r[molten.plugin_lifecycle_state_proof.cleanup_closes_authority] Molten MUST prove that plugin removal and cleanup retract or invalidate plugin hostcall authority, owned resources, and lifecycle callbacks before subsequent hostcall attempts can pass.

#### Scenario: Hostcall after removal denies
- GIVEN a passing plugin removal receipt
- WHEN a hostcall is requested for the removed plugin
- THEN hostcall decision is `deny`
- AND no plugin callback is invoked.

### Requirement: Hostcall operation refs are canonical
r[molten.plugin_contract_hardening.hostcall_ref_binding] Plugin hostcall receipts MUST bind the requested operation to the canonical hostcall ref or declared hostcall descriptor ref for that operation before the receipt can pass.

#### Scenario: Mismatched hostcall ref denies
- GIVEN a plugin manifest that declares the `storage.read` hostcall ref
- WHEN a hostcall request carries operation `network.open` with the `storage.read` hostcall ref
- THEN Molten emits a plugin hostcall receipt with decision `deny`
- AND no executor, effect, or network side effect is treated as admitted.

### Requirement: Plugin receipts bind active manifest identity
r[molten.plugin_contract_hardening.manifest_receipt_binding] Plugin install, permission, lifecycle, hostcall, health, removal, upgrade, negotiation, and compatibility receipts MUST parse and expose the manifest refs they bind, and lifecycle evaluation MUST reject receipts whose manifest refs do not match the active manifest state for the evaluated operation.

#### Scenario: Stale manifest receipt denies
- GIVEN an active plugin manifest and a same-plugin receipt produced for an older manifest ref
- WHEN lifecycle evaluation considers the stale receipt for activation, hostcall, health, removal, or upgrade
- THEN the lifecycle decision is `deny`
- AND diagnostics identify the manifest binding mismatch.

### Requirement: Receipt decisions and checks are coherent
r[molten.plugin_contract_hardening.receipt_check_coherence] Plugin receipt parsers MUST verify required check names, required check statuses, and decision/check consistency. A passing receipt MUST NOT contain failed required gates, and a denied receipt MUST identify at least one failed required gate or diagnostic before it can be accepted as denial evidence.

#### Scenario: Forged pass receipt is rejected
- GIVEN a plugin hostcall receipt with decision `pass` and a required `declared-hostcall` check marked `fail`
- WHEN Molten parses the receipt
- THEN parsing fails as invalid harness evidence
- AND lifecycle evaluation cannot treat the receipt as hostcall authority.

### Requirement: Extension contracts are canonical artifacts
r[molten.plugin_extension_contracts.contract_artifact] Molten MUST represent plugin extension surfaces with canonical `plugin-extension-contract-v1` artifacts that bind extension id, version, compatible host ABI, lifecycle callback changes, hostcall descriptors, input/output schema refs, authority requirements, resource requirements, effect manifest refs, replay/idempotency class, error class refs, conformance refs, policy refs, and supply-chain evidence refs.

#### Scenario: Manifest binds extension contract
- GIVEN a plugin manifest that uses an extension-provided hostcall
- WHEN Molten parses the manifest
- THEN the manifest binds the extension contract ref that declares the hostcall descriptor
- AND activation still requires separate authority, resource, effect, policy, and supply-chain evidence.

### Requirement: Hostcall admission uses per-hostcall contract requirements
r[molten.plugin_extension_contracts.per_hostcall_requirements] Plugin hostcall admission MUST evaluate the specific hostcall descriptor from the bound extension contract and MUST require matching input schema refs, output schema refs when applicable, authority refs, resource refs, effect manifest or effect receipt refs, and executor evidence before the receipt can pass.

#### Scenario: Generic authority is insufficient
- GIVEN a plugin extension contract whose `storage.read` descriptor requires a storage-read authority scope and storage resource ref
- WHEN a hostcall request supplies an unrelated authority ref and unrelated resource ref
- THEN Molten emits a plugin hostcall receipt with decision `deny`
- AND diagnostics identify the missing descriptor-specific requirements.

### Requirement: Extension contracts are Nickel-authored and canonically exported
r[molten.plugin_extension_contracts.nickel_authoring] Human-authored plugin extension contracts MUST use typed Nickel contracts by default and MUST export checked-in canonical evidence refs consumed by Rust validation. Runtime plugin admission MUST NOT execute Nickel or treat Nickel source presence as authority.

#### Scenario: Invalid authored contract fails before runtime admission
- GIVEN a Nickel-authored extension contract missing a required hostcall schema ref
- WHEN contract export or validation runs
- THEN the contract fails validation before a plugin manifest can bind it
- AND no runtime admission path treats the invalid Nickel source as trusted authority.

### Requirement: Extension negotiation is explicit and fail-closed
r[molten.plugin_extension_contracts.negotiation] Plugin activation MUST compare required and optional plugin extension contract refs against a host-supported extension or feature snapshot, emit a canonical negotiation receipt, and deny activation when any required extension is missing, incompatible, downgraded unsafely, or admitted only by implicit fallback.

#### Scenario: Missing required extension denies activation
- GIVEN a plugin manifest requiring a storage extension contract version that the host does not support
- WHEN plugin activation negotiation runs
- THEN Molten emits a negotiation receipt with decision `deny`
- AND no lifecycle callback or hostcall from that extension is admitted.

### Requirement: Extension compatibility gates upgrades
r[molten.plugin_extension_contracts.compatibility_receipts] Plugin upgrades MUST emit an extension compatibility receipt that compares old and new extension contract refs, host ABI compatibility, retained required extension ids, compatible versions, retained or migrated hostcall descriptors, schema compatibility, authority/resource/effect requirement compatibility, migration refs, rollback refs, cleanup refs, and conformance refs before replacing an active manifest.

#### Scenario: Removed required hostcall denies upgrade
- GIVEN an active plugin manifest whose extension contract requires a declared hostcall descriptor
- WHEN a replacement manifest drops that descriptor without a compatible migration and cleanup receipt
- THEN Molten emits an extension compatibility receipt with decision `deny`
- AND the active manifest remains unchanged.

### Requirement: Extension conformance evidence is bound
r[molten.plugin_extension_contracts.conformance_evidence] Production-admitted plugin extension contracts MUST bind positive, negative, and bounded property conformance suite refs, and plugin activation or upgrade MUST deny when required conformance evidence is missing, stale, or failed for the selected contract refs.

#### Scenario: Missing conformance evidence denies production admission
- GIVEN a plugin extension contract selected for a production activation profile
- WHEN the contract lacks required negative or property conformance evidence refs
- THEN Molten emits an activation or negotiation denial receipt
- AND diagnostics identify the missing conformance evidence.

### Requirement: Plugin capability grants are canonical artifacts
r[molten.plugin_capability_grants.grant_artifact] Molten MUST represent plugin host authority with canonical `plugin-capability-grant-v1` artifacts that bind the subject plugin ref, plugin id, active manifest ref, optional extension contract ref, hostcall descriptor ref, operation, input/output schema refs, resource refs and scope, effect manifest refs, policy refs, issuer/proof refs, attenuation metadata, revocation evidence refs, and replay/idempotency class. The BLAKE3 grant ref MUST identify the exact canonical grant value and MUST NOT be treated as authority unless the grant body parses and validates for the requested operation.

#### Scenario: Grant ref binds exact hostcall authority
- GIVEN a plugin manifest that declares `storage.read`
- AND a canonical capability grant bound to that manifest, operation, descriptor, schemas, resource, policy, effect, issuer, and proof evidence
- WHEN plugin hostcall admission evaluates the request
- THEN the hostcall may pass only by binding the matching capability grant ref
- AND a different BLAKE3 artifact ref is not accepted as authority.

### Requirement: Hostcall admission requires typed capability grants
r[molten.plugin_capability_grants.hostcall_admission] Plugin hostcall admission MUST require supplied capability grant refs to resolve to `plugin-capability-grant-v1` artifacts whose subject, manifest, extension contract, descriptor, operation, schemas, resources, effects, policies, and proofs match the selected hostcall descriptor before any Steel, Wasm, native-adapter, or remote-proxy host side effect can occur. Generic authority refs MAY be retained as compatibility or proof evidence, but they MUST NOT satisfy a descriptor that requires typed capability grants by themselves.

#### Scenario: Generic authority ref is insufficient
- GIVEN a plugin extension contract whose `storage.read` descriptor requires a typed capability grant
- WHEN a hostcall request supplies only a non-empty generic authority ref and no matching capability grant artifact
- THEN Molten emits a plugin hostcall receipt with decision `deny`
- AND diagnostics identify the missing typed capability grant.

### Requirement: Capability attenuation and revocation are deterministic
r[molten.plugin_capability_grants.revocation_attenuation] Plugin capability grant validation MUST enforce attenuation and revocation from explicitly supplied canonical evidence, including narrowed operations, resource sub-scopes, schema/profile constraints, delegation depth, budget refs, turn/tick validity evidence, and revocation receipt refs. The pure admission core MUST NOT read clocks, files, networks, or mutable revocation registries while deciding whether a grant is valid.

#### Scenario: Revoked grant denies hostcall
- GIVEN a plugin hostcall request with a capability grant whose operation and resource match the descriptor
- AND canonical revocation evidence invalidates that grant for the evaluated turn
- WHEN hostcall admission runs
- THEN the hostcall receipt decision is `deny`
- AND no host side effect is admitted.

### Requirement: Capability grants are Nickel-authored and canonically exported
r[molten.plugin_capability_grants.nickel_authoring] Human-authored plugin capability grant fixtures and grant templates SHOULD use typed Nickel contracts by default and MUST export checked-in canonical Preserves evidence before Rust validation consumes them. Runtime admission MUST NOT execute Nickel or treat Nickel source presence as authority.

#### Scenario: Invalid grant fixture fails before admission
- GIVEN a Nickel-authored plugin capability grant fixture whose resource ref does not match the declared hostcall descriptor
- WHEN export or validation checks run
- THEN the fixture fails before runtime admission can bind it
- AND the invalid Nickel source is not treated as trusted authority.
