# Tasks: plugin-extension-negotiation-conformance

## Phase 1: Negotiation receipts

- [x] [serial] r[molten.plugin_extension_contracts.negotiation] Define `plugin-extension-negotiation-receipt-v1` constructors and parsers binding manifest ref, required contract refs, optional contract refs, host-supported feature snapshot ref, selected refs, decision, diagnostics, and checks.
- [x] [serial] r[molten.plugin_extension_contracts.negotiation] Implement pure fail-closed negotiation over required and optional extension contract refs.
- [x] [parallel] r[molten.plugin_extension_contracts.negotiation] Add activation gate integration so missing required extensions deny before lifecycle callbacks or hostcalls.

## Phase 2: Compatibility receipts

- [x] [serial] r[molten.plugin_extension_contracts.compatibility_receipts] Define `plugin-extension-compatibility-receipt-v1` for old/new contract refs, host ABI compatibility, version compatibility, hostcall descriptor retention/migration, schemas, authority/resource/effect requirements, migration refs, rollback refs, cleanup refs, and conformance refs.
- [x] [serial] r[molten.plugin_extension_contracts.compatibility_receipts] Integrate extension compatibility receipts into plugin upgrade validation before active manifest replacement.
- [x] [parallel] r[molten.plugin_extension_contracts.compatibility_receipts] Preserve existing plugin id, ABI, schema, rollback, and cleanup gates while adding extension-specific diagnostics.

## Phase 3: Conformance evidence

- [x] [serial] r[molten.plugin_extension_contracts.conformance_evidence] Add conformance refs to extension contract validation for positive, negative, and bounded property suites.
- [x] [serial] r[molten.plugin_extension_contracts.conformance_evidence] Require passing conformance evidence for production activation and upgrade profiles.
- [x] [parallel] r[molten.plugin_extension_contracts.conformance_evidence] Allow diagnostic-only development contracts only when receipts mark them non-production and non-authority.

## Phase 4: Positive and negative tests

- [x] [parallel] r[molten.plugin_extension_contracts.negotiation] Add missing required extension and unsafe downgrade denial tests.
- [x] [parallel] r[molten.plugin_extension_contracts.negotiation] Add optional extension omission tests that preserve fail-closed authority boundaries.
- [x] [parallel] r[molten.plugin_extension_contracts.compatibility_receipts] Add compatible upgrade pass and removed required hostcall denial tests.
- [x] [parallel] r[molten.plugin_extension_contracts.conformance_evidence] Add missing, stale, and failed conformance evidence denial tests.

## Phase 5: Evidence and validation

- [x] [serial] r[molten.plugin_extension_contracts.negotiation] r[molten.plugin_extension_contracts.compatibility_receipts] r[molten.plugin_extension_contracts.conformance_evidence] Run focused plugin tests, conformance fixtures, and Cairn validation.
