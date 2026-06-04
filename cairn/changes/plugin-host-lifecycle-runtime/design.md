## Context

Molten has reviewed Steel/Wasm execution, adapter preflight fixtures, artifact registry, effect handles, resources, and authority contexts. A plugin host composes those pieces into an operator-facing lifecycle: install, activate, call, health, upgrade, deactivate, remove.

## Goals

- Define `plugin-manifest-v1`, `plugin-install-receipt-v1`, `plugin-permission-receipt-v1`, `plugin-lifecycle-receipt-v1`, `plugin-hostcall-receipt-v1`, `plugin-health-receipt-v1`, and `plugin-upgrade-receipt-v1`.
- Require plugins to be artifact-backed with declared schemas, ABI version, lifecycle callbacks, effect manifest refs, hostcall refs, policy refs, resource refs, and supply-chain refs.
- Support first plugins as reviewed Wasm/Steel/native-adapter artifacts behind the existing executor boundary.
- Deny ambient filesystem, network, environment, clock, process, or node-control access unless explicitly declared and admitted.
- Integrate plugin lifecycle with service supervision and node adapter startup/shutdown.
- Support upgrade/remove with cleanup receipts and compatibility checks.

## Non-Goals

- No arbitrary dynamic library loading in the first slice.
- No host ABI compatibility with Aspen plugins.
- No plugin authority from install alone.
- No opaque plugin-specific logging as normative evidence.

## Records

```preserves
<plugin-manifest-v1 "molten.plugin.manifest.v1"
  <plugin-id "plugin:example">
  <artifact <artifact-ref>>
  <abi "molten.plugin.host-abi.v1">
  <lifecycle ["init" "start" "stop" "health"]>
  <effects [<effect-manifest-ref> ...]>
  <hostcalls [<hostcall-ref> ...]>
  <schemas [<schema-ref> ...]>
  <policy [<policy-ref> ...]>
  <resource [<resource-ref> ...]>
  <supply-chain [<provenance-ref> ...]>
  <checks [<check "artifact-backed" "pass"> ...]>>
```

```preserves
<plugin-lifecycle-receipt-v1 "molten.plugin.lifecycle-receipt.v1"
  <operation "install"|"init"|"start"|"health"|"stop"|"remove"|"upgrade">
  <decision "pass"|"deny">
  <plugin <plugin-ref>>
  <manifest <plugin-manifest-ref>>
  <executor <executor-receipt-ref>>
  <authority [<authority-receipt-ref> ...]>
  <resource [<resource-receipt-ref> ...]>
  <effects [<effect-receipt-ref> ...]>
  <diagnostics ["..." ...]>
  <checks [<check "no-ambient-authority" "pass"> ...]>>
```

## Lifecycle

Install verifies the artifact, schema, ABI, effect manifests, provenance refs, and policy. Activation binds handler profiles and hostcalls. Runtime hostcalls reuse executor hostcall/admission receipts. Stop/remove retract plugin-owned assertions, services, handles, and catalog entries. Upgrade creates a new manifest/artifact ref and emits compatibility/cleanup receipts.
