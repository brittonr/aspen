## Context

ChoRus packages choreographic programming as a Rust library rather than a standalone language. The most relevant ideas are ergonomic, not semantic:

- marker types for locations and location sets;
- located values that can be unwrapped only at the owning location;
- a `ChoreoOp`-style operator interface injected into choreography definitions;
- direct runner semantics for tests and endpoint projection for distributed execution;
- an approachable API for send, broadcast, multicast, choice/conclave, fan-in, and fan-out patterns.

Molten already has an accepted choreography model: manifest registries compile to Trellis `GlobalChoreo`; Trellis projectability gates installation; `LocalChoreo` endpoints become canonical protocol endpoint/session records; runtime transitions publish or consume canonical protocol-message records through dataspace and policy/evidence gates. That path remains authoritative.

## Design

### Adapted surface

The adapted surface is a typed facade over Molten protocol manifests after admission:

```text
Nickel/Preserves protocol manifest
  -> deterministic role/label/payload registries
  -> Trellis GlobalChoreo
  -> Trellis projectability gate
  -> Trellis LocalChoreo per role
  -> protocol install receipt
  -> generated typed Rust facade
  -> Sans-IO transition core
  -> dataspace-backed shell after gates
```

The facade may expose role marker types, action enums, located payload wrappers, and operator traits for authoring and testing. Those types are generated from or checked against the admitted manifest and projection evidence. They are not the source of protocol truth.

### EPP-as-DI in Molten terms

ChoRus's EPP-as-DI pattern maps to a Molten operator trait that receives explicit state and facts and returns transition outputs:

- local computation intent;
- protocol-message descriptor;
- branch selection descriptor;
- receive/offer expectation;
- diagnostics;
- receipt input facts;
- next endpoint state or denial.

The trait implementation used in tests can run entirely in memory. The production shell translates admitted transition outputs into dataspace assertions/messages, receipt writes, and transport carrier effects only after authority, policy, resource, replay, and evidence gates pass.

### Role-scoped values

Located payload wrappers should make wrong-role payload access unrepresentable where Rust types can express it. Dynamic boundaries still require runtime denial because manifests, remote messages, and receipt bundles are external data. Wrong role, wrong peer, wrong label, wrong payload tag, stale sequence, and missing evidence all deny in the pure transition core before any shell side effect.

### Runner/projection parity

A facade runner is useful only as a deterministic test oracle. It should execute against the same projected endpoint state and canonical transition refs that the production interpreter uses. Runner output must compare canonical protocol message refs, before/after endpoint state refs, branch evidence refs, and gate receipt refs rather than logs or debug strings.

### Not adopted

Molten does not adopt ChoRus as a crate dependency or compatibility target. In particular, Molten does not adopt:

- ChoRus local blocking-queue transport;
- ChoRus HTTP transport or retry policy;
- serde_json payload identity;
- ChoRus runtime projection as evidence;
- ChoRus derive macros as source of protocol truth;
- ChoRus API or wire compatibility claims.

ChoRus remains a reference codebase under its MIT license. Any copied code would require explicit license preservation and review, but this change intends adaptation by design rather than source import.

### Evidence and non-claims

Generated facade artifacts should bind:

- protocol manifest ref;
- install receipt ref;
- role/label/payload registry refs;
- projected endpoint refs;
- generated source or artifact ref;
- generator version/ref;
- explicit non-claims: no transport trust, no authority grant, no provenance approval, no policy/resource admission, no ChoRus compatibility.

## Alternatives

- **Adopt `chorus_lib` directly**: rejected because it would bypass Molten's canonical Preserves, Trellis, policy, replay, and evidence contracts.
- **Keep only manifest APIs**: safe but less ergonomic for Rust protocol authors and less useful for typed fixture generation.
- **Generate code before projectability**: rejected because it can create APIs for protocols Molten will not admit.
