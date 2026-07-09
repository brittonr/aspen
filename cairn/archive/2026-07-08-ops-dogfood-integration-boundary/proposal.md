## Why

Operator dogfood, production soak, and NixOS VM evidence are integration and release-readiness surfaces. They should not be entangled with runtime cores or adapter semantics, because their job is to observe and package evidence, not to grant runtime authority.

## What Changes

- Separate operator/dogfood orchestration from runtime, node, storage, and transport cores.
- Treat NixOS VM and production soak code as integration shells that consume stable runtime APIs and emit evidence.
- Move reusable release-readiness summaries behind typed evidence inputs.
- Add positive and negative tests for evidence-only operator workflows.

## Impact

Operational evidence remains strong, but core runtime crates stay smaller and less coupled to dogfood, Nix, and VM-specific environments.
