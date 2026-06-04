## Why

Aspen's plugin lessons are central to Molten's extensibility goal, but Molten currently has executor fixtures and artifact records rather than an operational plugin host. A plugin runtime must make install, permission review, lifecycle callbacks, hostcall mapping, resource budgets, upgrade, and removal receipt-backed and deny-by-default.

## What Changes

- Add canonical plugin manifest, install, permission, lifecycle, hostcall, health, upgrade, and removal receipts.
- Treat plugins as artifact-backed executors/adapters with explicit effect manifests and handler bindings.
- Gate plugin activation through policy, authority, resource, schema compatibility, and supply-chain evidence.
- Route plugin hostcalls through the existing executor/effect-handle boundary.
- Add tests for ambient authority denial, lifecycle cleanup, upgrade compatibility, and failure isolation.

## Impact

This provides the extension point needed for production adapters and applications without compromising Molten's Preserves/evidence spine.
