## ADDED Requirements

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
