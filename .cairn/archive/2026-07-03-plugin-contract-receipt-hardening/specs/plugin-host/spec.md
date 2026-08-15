## ADDED Requirements

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
