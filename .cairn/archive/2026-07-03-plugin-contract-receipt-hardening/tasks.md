# Tasks: plugin-contract-receipt-hardening

## Phase 1: Pure receipt validation

- [x] [serial] r[molten.plugin_contract_hardening.hostcall_ref_binding] Add pure hostcall operation/ref binding checks for primitive hostcall descriptors.
- [x] [serial] r[molten.plugin_contract_hardening.manifest_receipt_binding] Parse and expose manifest refs on plugin hostcall, health, removal, and lifecycle receipt structs where missing.
- [x] [parallel] r[molten.plugin_contract_hardening.receipt_check_coherence] Replace check-name-only parser requirements with required-status and decision/check coherence validation.

## Phase 2: Lifecycle integration

- [x] [serial] r[molten.plugin_contract_hardening.manifest_receipt_binding] Update plugin lifecycle state evaluation to reject stale or mismatched manifest refs for activation, hostcall, health, removal, and upgrade requests.
- [x] [parallel] r[molten.plugin_contract_hardening.hostcall_ref_binding] Ensure hostcall receipts deny before side effects when operation and ref disagree.

## Phase 3: Positive and negative tests

- [x] [parallel] r[molten.plugin_contract_hardening.hostcall_ref_binding] Add a positive declared-hostcall test where operation and hostcall ref match.
- [x] [parallel] r[molten.plugin_contract_hardening.hostcall_ref_binding] Add a negative operation/ref mismatch test using `network.open` with the `storage.read` ref.
- [x] [parallel] r[molten.plugin_contract_hardening.manifest_receipt_binding] Add stale same-plugin manifest receipt denial tests.
- [x] [parallel] r[molten.plugin_contract_hardening.receipt_check_coherence] Add forged pass/failed-check and deny-without-diagnostics parser rejection tests.

## Phase 4: Evidence and validation

- [x] [serial] r[molten.plugin_contract_hardening.hostcall_ref_binding] r[molten.plugin_contract_hardening.manifest_receipt_binding] r[molten.plugin_contract_hardening.receipt_check_coherence] Run focused plugin tests and Cairn validation.
