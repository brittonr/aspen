## Context

Dogfood and production readiness workflows exercise many subsystems together. That is useful for release review but risky as an implementation dependency: integration harnesses should depend on stable runtime APIs, not the other way around.

## Design

### Integration ownership

- `operator`: runbook and release workflow orchestration.
- `dogfood`: local node workflow bundles and evidence aggregation.
- `prod`: production-readiness and soak evidence summaries.
- `nixos_vm`: VM topology and platform integration shell.
- `evidence`: canonical operator receipt constructors and parsers.

### Direction of dependency

Runtime and node cores must not import dogfood, prod-soak, or NixOS VM modules. Operator workflows may call stable runtime/node APIs and adapters, then package results as review evidence.

### Evidence-only boundary

Dogfood, soak, and VM receipts are release-review evidence. They do not grant authority, policy, resource, provenance, retention, execution, or transport trust unless separate admission evidence says so.

### Test strategy

Positive tests cover valid evidence aggregation. Negative tests cover missing child evidence, stale refs, VM unavailable, diagnostic logs without canonical receipts, and overbroad pilot or production claims.

## Non-goals

- Do not remove dogfood or soak workflows.
- Do not claim VM or soak evidence proves broad production readiness by itself.
- Do not make Nix or VM execution a runtime dependency.
