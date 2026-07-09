# Plugin Host Delta: Explicit Lifecycle FSM Core

## ADDED Requirements

### Requirement: Plugin lifecycle has a reviewed transition table
r[molten.plugin_lifecycle_state_proof.transition_table] Molten MUST model plugin lifecycle admission with a reviewed finite transition table over current lifecycle state, lifecycle event, active manifest identity, and explicit guard facts, and MUST deny lifecycle events that are not admitted by that table.

#### Scenario: Valid plugin progresses to active
- GIVEN a plugin manifest has passing install, permission, extension negotiation when required, authority, resource, effect, policy, and supply-chain evidence
- WHEN the activation event is evaluated against the reviewed transition table
- THEN Molten emits a passing lifecycle decision
- AND the next state is active for that manifest ref.

#### Scenario: Hostcall before permission denies
- GIVEN a plugin has a manifest and install receipt but no passing permission state
- WHEN a hostcall event is evaluated
- THEN the lifecycle decision is deny
- AND no callback, hostcall, or effect side effect is authorized.

### Requirement: Plugin lifecycle guards are explicit
r[molten.plugin_lifecycle_state_proof.guard_binding] Plugin lifecycle transitions MUST bind manifest, ABI, policy, resource, effect, supply-chain, extension negotiation, compatibility, health, cleanup, rollback, and recovery evidence as explicit guard facts instead of deriving lifecycle authority from receipt possession alone.

#### Scenario: Stale manifest guard denies
- GIVEN a lifecycle receipt was produced for an older manifest ref
- WHEN the active manifest state evaluates activation, hostcall, health, removal, or upgrade
- THEN the transition decision is deny
- AND diagnostics identify the manifest binding mismatch.

#### Scenario: Failed health blocks upgrade without recovery
- GIVEN the plugin lifecycle state includes failed or stale health evidence
- WHEN an upgrade event is evaluated without a later passing recovery fact
- THEN the upgrade transition denies
- AND the prior plugin state remains current.

### Requirement: Plugin lifecycle decisions bind states and authority closure
r[molten.plugin_lifecycle_state_proof.state_receipts] Plugin lifecycle decision evidence MUST bind the prior lifecycle state, requested event, target or next state, active manifest ref, selected guard refs, authority-closed flag, side-effect authorization class, decision, and diagnostics.

#### Scenario: Removal closes hostcall authority
- GIVEN plugin removal and cleanup pass for the active manifest
- WHEN a later hostcall event is evaluated
- THEN lifecycle evidence reports authority closed
- AND the hostcall event denies before invoking plugin code.

### Requirement: Plugin lifecycle traces cover positive and negative orders
r[molten.plugin_lifecycle_state_proof.transition_tests] Molten SHOULD test plugin lifecycle with positive ordered traces and negative traces for missing install, missing permission, activation without required negotiation, hostcall before permission, hostcall in stopped or removed state, stale manifest receipts, failed health without recovery, upgrade without rollback or cleanup evidence, and cleanup that leaves authority open.

#### Scenario: Generated lifecycle trace preserves table semantics
- GIVEN a generated plugin lifecycle trace includes both admitted and invalid lifecycle events
- WHEN the trace is evaluated
- THEN every pass corresponds to a reviewed table transition
- AND every invalid event emits deny evidence without authorizing side effects.