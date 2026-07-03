## ADDED Requirements

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
