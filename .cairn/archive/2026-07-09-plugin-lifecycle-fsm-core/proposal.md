## Why

Plugin lifecycle requirements already demand ordered receipts, health gates, cleanup authority closure, and active-manifest binding. The current model is still easy to read as a bundle of receipt-presence checks instead of a first-class lifecycle machine.

A plugin host should decide activation, hostcalls, health, upgrade, removal, and cleanup from an explicit state/event relation so impossible orders cannot pass by assembling enough otherwise-valid receipts.

## What Changes

- Define a plugin lifecycle FSM core with reviewed states, events, guards, and terminal/authority-closed behavior.
- Treat install, permission, activation, hostcall, health, upgrade, removal, cleanup, negotiation, compatibility, and recovery as events over current plugin state.
- Ensure manifest, ABI, policy, resource, effect, supply-chain, extension, and health evidence are guards on transitions rather than ambient facts.
- Add positive and negative trace fixtures for valid lifecycle progression, hostcall-before-permission, stale manifest receipts, failed-health upgrade, hostcall-after-removal, and incomplete cleanup.

## Impact

Plugin lifecycle review becomes table-driven and traceable. Hostcall and upgrade admission remain separate authority/effect decisions, but lifecycle state stops being inferred from receipt presence alone.