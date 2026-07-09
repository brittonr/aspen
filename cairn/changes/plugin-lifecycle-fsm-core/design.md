## Context

The plugin host already parses manifest-bound receipts and has a lifecycle-state evaluator. The improvement is to make that evaluator an explicit finite state machine with typed events and transition outputs, following the same style as the shared lifecycle relation.

## Design

### State and event model

Model plugin lifecycle with closed states such as manifest-declared, installed, permitted, active, healthy, degraded, stopped, removed, upgrading, upgraded, and cleanup-required where the final names are chosen in code and receipts. Events include install, permission-review, activate, hostcall, health-pass, health-fail, stop, remove, cleanup, negotiate-extension, compatibility-check, upgrade, rollback, and recover.

The pure core takes:

- current plugin lifecycle state or replayed receipt-derived state;
- event request;
- active manifest identity;
- parsed receipts and guard facts for ABI, policy, resource, effect, supply-chain, extension negotiation, compatibility, health, cleanup, and recovery;
- deterministic evaluation turn or freshness facts where needed.

The core returns decision, diagnostics, authorized side-effect class, next state, authority-closed flag, and receipt input facts. It does not invoke callbacks, execute hostcalls, persist receipts, inspect filesystem state, or call adapter APIs.

### Guards and authority boundaries

Hostcall and upgrade transitions require the plugin to be installed, permitted, active, manifest-current, and not authority-closed. Hostcall requires declared hostcall/effect/capability guard evidence. Upgrade requires compatible ABI/schema/extension and rollback/cleanup evidence. Failed health blocks activation, hostcall, and upgrade unless an explicit recovery event is bound.

Removal and cleanup close plugin-owned hostcall authority before later hostcall events can pass.

### Receipts

Lifecycle decision receipts should expose the evaluated prior state, event, next state or preserved state, active manifest ref, selected guard receipt refs, decision, diagnostics, and side-effect authorization class. Existing receipt records can remain; the FSM receipt or decision value should make the order proof easy to replay.

### Tests

Add table-driven relation tests and generated traces. Negative cases should include missing install, missing permission, activation without negotiation when extensions are required, hostcall before permission, hostcall in stopped or removed state, failed health without recovery, upgrade with stale manifest, upgrade without rollback/cleanup, and cleanup that leaves authority open.