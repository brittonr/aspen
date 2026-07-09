## Context

Molten uses Iroh for live node-control, framed envelopes, gossip, blob/doc exchange, diagnostics, and future protocol sessions. The accepted node-runtime spec already requires a runtime-managed Iroh protocol router with install, replacement, removal, shutdown, and unsupported-ALPN denial receipts. This change adds the missing registry discipline: a single reviewed source of truth for Molten-owned ALPN identifiers and their owner/admission metadata.

## Design

### Registry model

A registry entry should be a canonical record or generated table that can be rendered into Preserves evidence. Each entry records:

- protocol namespace and stable symbolic name;
- ALPN bytes or string, with a deterministic encoding rule;
- owning subsystem or adapter;
- handler profile id and supported schema/profile versions;
- operation class such as node-control, protocol-session, diagnostic-readback, blob/chunk, or peer-bootstrap;
- required authority, policy, resource, provenance, replay, or source-gate inputs for installing the handler;
- limit profile refs for frame size, stream count, session count, and retry behavior;
- lifecycle state: proposed, active, deprecated, migration-only, or removed;
- compatibility notes and receipt schema refs.

Human-authored policy/config for registry growth should prefer Nickel contracts. Runtime code may consume checked/generated artifacts or Rust constants, but router receipts should bind the canonical registry entry ref used for admission.

### Admission flow

```text
requested handler install/replace/remove
  -> parse ALPN and owner namespace
  -> check registry uniqueness and formatting
  -> check current generation and lifecycle state
  -> check handler profile compatibility
  -> check authority/policy/resource/evidence gates
  -> emit router receipt
  -> mutate live router map only on pass
```

Incoming connections for unsupported, removed, malformed, or stale registry entries deny before frame delivery. Replacement advances generation and records prior handler shutdown evidence.

### Ownership and compatibility

Each Molten-owned ALPN has exactly one owner namespace. App-specific protocols may live next to their adapter implementation, but they still need a registry record before runtime installation. Deprecated or migration-only entries remain explicit and must not silently become production defaults.

### Non-authority boundary

The registry routes connections. It does not authenticate operation authority. A peer that negotiates a valid ALPN still needs peer bootstrap, capability, authority, policy, resource, replay/idempotency, and subsystem-specific gates before side effects.

### Fixtures

Positive fixtures should cover unique valid entries, admitted install, admitted replacement, and removal. Negative fixtures should cover duplicate ALPN bytes, malformed encodings, wrong owner namespace, unsupported lifecycle state, stale generation, handler-profile mismatch, unsupported incoming ALPN, and attempts to treat ALPN negotiation as authority.

## Non-goals

- Do not introduce HTTP as an internal control plane.
- Do not claim ALPN registry entries are peer authority or operation grants.
- Do not require every external Iroh ecosystem ALPN to become Molten-owned.
- Do not mutate live router state from docs or rendered summaries without canonical registry and gate evidence.