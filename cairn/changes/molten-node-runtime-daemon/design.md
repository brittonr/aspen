## Context

The repository currently has strong local primitives: canonical Preserves values, ledgers, artifact registries, chunk stores, typed storage, job DAGs, remote dataspace envelopes, identity/bootstrap, authority, resources, and receipts. Most are callable as one-shot CLI commands. Aspen 2.0 requires a coherent node process with explicit lifecycle and durable state roots.

## Goals

- Introduce `molten node init`, `molten node run`, `molten node status`, and `molten node stop` commands.
- Define canonical `node-config-v1`, `node-startup-receipt-v1`, `node-adapter-receipt-v1`, `node-control-request-v1`, `node-control-receipt-v1`, and `node-shutdown-receipt-v1` records.
- Bind node identity, config ref, state root profile refs, policy refs, capability refs, resource refs, effect-handle refs, and binary/version refs into startup evidence.
- Start adapters in deterministic dependency order: ledger, registry, chunk store, typed storage, cache, remote dataspace, job runtime, control surface.
- Provide a local Preserves control socket/file/stdio profile whose text output is only a rendered view over canonical receipts.
- Make shutdown graceful and receipt-backed: stop control intake, drain admitted turns/jobs, persist indexes, close adapters, emit final health/shutdown evidence.

## Non-Goals

- No global cluster manager.
- No Raft requirement for single-node local mode.
- No implicit authority from local filesystem ownership.
- No production network listener without explicit config and admission.
- No opaque JSON-only control API; Preserves remains normative.

## Records

```preserves
<node-config-v1 "molten.node.config.v1"
  <node-id <node-identity-ref>>
  <state-root <state-root-profile-ref>>
  <adapters [<adapter "ledger" <profile-ref>> ...]>
  <policy [<policy-ref> ...]>
  <capability [<authority-context-ref> ...]>
  <resource [<resource-ref> ...]>
  <effects [<handler-profile-ref> ...]>
  <checks [<check "explicit-state-root" "pass"> ...]>>
```

```preserves
<node-startup-receipt-v1 "molten.node.startup-receipt.v1"
  <decision "pass"|"deny">
  <node-config <node-config-ref>>
  <identity <node-identity-receipt-ref>>
  <adapters [<adapter "ledger" <adapter-receipt-ref>> ...]>
  <policy [<policy-ref> ...]>
  <capability [<authority-receipt-ref> ...]>
  <resource [<resource-receipt-ref> ...]>
  <version [<artifact-or-binary-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "no-ambient-authority" "pass"> ...]>>
```

## Control Surface

The first control profile is local-only and accepts canonical Preserves requests:

- `<node-status-request-v1 ...>`
- `<artifact-install-request-v1 ...>`
- `<job-run-request-v1 ...>`
- `<remote-dataspace-request-v1 ...>`
- `<gate-check-request-v1 ...>`
- `<node-shutdown-request-v1 ...>`

Every request returns a `node-control-receipt-v1` that binds request ref, caller authority ref, resource decision, sub-receipt refs, and the final decision.

## Denial Cases

Startup or control requests deny when config refs are stale, state roots are implicit, authority/resource/effect refs are missing, adapter indexes fail verification, control requests use unknown schemas, production network adapters are enabled without policy, or adapter startup order would introduce an invisible side effect.
