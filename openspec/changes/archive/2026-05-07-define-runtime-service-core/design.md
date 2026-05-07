## Context

Aspen already has Raft/KV, Iroh transport, auth/capabilities, blobs/snix artifacts, jobs/CI, Forge, plugins, deploy records, and dogfood receipts. The host-loading OpenSpec now defines how runtime artifacts and host boundaries should be classified. That is necessary but not sufficient: Aspen still needs the control-plane object model and lifecycle contract for services and applications.

Current Forge wiring is direct in-process startup plus handler registration. It becomes distributed by using Aspen primitives internally, not because it is reconciled as a runtime service. The first runtime-service slice should wrap that existing behavior rather than rewrite Forge.

## Goals

- Define a durable service model above Raft/KV and below Forge/Executioner/user apps.
- Define only minimal application identity/ownership references needed by service specs; full app install/upgrade remains out of scope.
- Keep pure model types portable and data-only.
- Make native built-ins first-class runtime services.
- Use Forge as the first built-in service target because it already has routes, durable state, startup wiring, and receipts-worthy lifecycle events.
- Record redacted receipts for lifecycle and route transitions.

## Non-Goals

- Do not move all first-party services into WASM.
- Do not implement full distributed scheduling or workflow replay in this change.
- Do not claim runtime completeness after only model/Forge-wrapper work.
- Do not add plain production container execution.

## Decisions

### 1. Runtime service core is above substrate and below applications

**Choice:** `RuntimeServiceSpec`, `RuntimeServiceInstance`, lifecycle state, health, routes, capabilities, placement, resources, restart policy, and receipts form the first durable runtime service core.

**Rationale:** These are the common contracts every post-KV durable layer repeats today: startup, route ownership, health, capability handles, shutdown, and operator evidence.

### 2. Pure model first, runtime effects later

**Choice:** The first portable model types stay data-only and avoid process/network/filesystem/crypto side effects. Node-local effects in this change are limited to trait boundaries and Forge wrapper surfaces; a distributed reconciler/scheduler remains a later slice.

**Rationale:** Portable model tests can land early without destabilizing the node runtime and will give Forge/Executioner migrations a stable target.

### 3. Built-in native services are the first host path

**Choice:** First-party services such as Forge use linked `NativeBuiltIn` factories, service manifests, and runtime route declarations.

**Rationale:** This matches Aspen's current architecture and avoids unsafe dynamic native plugin loading while preserving a path to WASM/Hyperlight/microVM services later.

### 4. Forge is wrapped, not rewritten

**Choice:** The first Forge slice adds a runtime service manifest/lifecycle/route/health/receipt wrapper around existing Forge startup and handler paths.

**Rationale:** It proves the service contract against real code without coupling the first change to a full Forge internals rewrite.

### 5. Receipts must remain secret-safe

**Choice:** Runtime service receipts may include service names, generations, route IDs, artifact identities, node IDs, health states, and redacted capability summaries, but MUST NOT include raw tokens, private keys, cluster cookies, tickets, connection strings, or kernel/env secrets.

**Rationale:** Runtime receipts are operator-facing and may become long-lived evidence artifacts.

## Risks / Trade-offs

- **Overclaiming runtime completeness**: Mitigate by keeping this change scoped to service core and Forge built-in wrapper only.
- **Trait boundary too effectful**: Mitigate by separating pure model types from node-local factory/reconciler traits.
- **Forge wrapper drift**: Mitigate with source-anchor tests around current Forge startup and handler registration.
- **Receipt leakage**: Mitigate with tests that assert raw secret field names/values do not appear in runtime service receipts.

## Validation Plan

1. Validate OpenSpec strict parser shape.
2. Add pure unit tests for service model invariants.
3. Add source-anchor/doc tests that host loading is not the whole runtime.
4. Add Forge wrapper tests for manifest, route declarations, health, and redacted receipt shape.
5. Run focused cargo tests and whitespace checks.
