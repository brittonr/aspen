## ADDED Requirements

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
