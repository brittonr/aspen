## Context

Molten has several layers that already deny ambient authority:

- capability contexts and authority contexts decide whether a principal may act,
- policy gates decide whether a request is admitted,
- resource grants decide how much work may be consumed,
- executor preflights decide which hostcalls an executor may request,
- replay/evidence validation recomputes requests and decisions from canonical Preserves values.

What is still missing is a first-class identity for the concrete effect surface being used. An actor may legitimately hold two storage handles, two blob stores, two dataspaces, two clocks, two remote-sync peers, or two replay profiles. Encoding only `effect-kind = storage` or `operation = read` forces validators to infer which surface was meant from surrounding context.

Bluefin is useful prior art because it treats effects as value-level handles introduced by handlers. Molten should adapt the value-level handle discipline while keeping Molten's own Preserves schemas, authority model, resource governance, chain evidence, and replay law as normative.

## Goals

- Make concrete effect/adapter surfaces explicit at runtime and in evidence.
- Allow multiple same-kind effects in one actor/session/turn without ambiguity.
- Prove handle introduction-before-use, scope, expiry, and revocation during replay validation.
- Keep handles separate from authority: a handle identifies an effect surface, but authority still comes from capability/authority/policy/resource evidence.
- Bind handles into hostcall/effect request envelopes and gate receipts.
- Support compound handler profiles with multiple handles and dynamic operation sets.
- Keep the pure runtime core free of handle plumbing unless it crosses an effect/trust boundary.

## Non-Goals

- Do not implement or expose the Bluefin API, Haskell effect rows, `Eff`, `State`, `Exception`, or Bluefin's implementation strategy.
- Do not use handles as ambient authority or bearer tokens. Possessing a handle ref is not enough to act.
- Do not require ordinary pure helper functions or internal transition logic to pass handles around.
- Do not make ordinary actor messages depend on global handles or global effect ordering.
- Do not make remote handles transferable by default.
- Do not replace capability contexts, authority contexts, resource grants, policy gates, or executor preflights.

## Evidence model

Introduce canonical Preserves schemas for handler and handle evidence. The implemented binding schema is intentionally acyclic: a handler binding does not list child handle refs, because each handle ref is derived from a value that already contains the handler-binding ref. Compound handlers can add a separate aggregate/index artifact later without making the base binding hash circular.

Implemented handler binding shape:

```preserves
<handler-binding-v1 "molten.effects.handler-binding.v1"
  <profile "local" | "record" | "replay" | "chaos" | "profiling" | "production">
  <scope <run-ref> <session-ref> <actor-ref-or-none> <turn-ref-or-none>>
  <implementation <adapter-kind> <adapter-ref> <executor-preflight-ref-or-none>>
  <policy <policy-ref> <capability-context-ref> <authority-context-ref-or-none>>
  <resources [<resource-grant-ref> ...]>
  <operations ["read" "write" "send" ...]>
  <evidence [<receipt-ref> ...]>
  <checks [<check "deny-ambient-effects" "pass"> ...]>>
```

Implemented effect handle shape:

```preserves
<effect-handle-v1 "molten.effects.handle.v1"
  <kind "storage" | "blob" | "dataspace" | "clock" | "random" | "network" | "remote-sync" | "hostcall" | ...>
  <scope <run-ref> <session-ref> <actor-ref-or-none> <turn-ref-or-none>>
  <handler <handler-binding-ref>>
  <operations ["read" "write" ...]>
  <authority <capability-context-ref> <authority-context-ref-or-none>>
  <resources [<resource-grant-ref> ...]>
  <validity <not-before-or-none> <expires-at-or-none> [<revocation-ref> ...]>
  <transfer "local-only" | "attenuated-delegation" | "remote-proxy">
  <parent <parent-handle-ref-or-none>>
  <evidence [<receipt-ref> ...]>
  <checks [<check "handle-not-authority" "pass"> ...]>>
```

Handle refs are canonical hashes of `effect-handle-v1` values. Handler-binding refs are canonical hashes of `handler-binding-v1` values. If an implementation needs ephemeral in-memory objects, those objects are private cache entries keyed by canonical handle refs; they are not the source of identity.

## Request binding

The first implementation milestone binds executor hostcall requests; generic runtime effect-request and adapter-specific request records will follow the same pattern.

Evidence-bearing effect requests and executor hostcalls should include:

- `handle_ref`,
- effect kind and operation,
- actor/session/run/turn refs,
- sequence/replay metadata,
- canonical input ref,
- capability context / authority context refs,
- policy decision refs,
- resource grant/consumption refs,
- handler-binding ref,
- denial or response refs.

The request is invalid if the named handle does not permit the operation, does not match the request context, or lacks matching admission/resource evidence.

## Scope and lifetime rules

Validators must fail closed if:

1. A request uses a handle whose `effect-handle-v1` artifact is missing or malformed.
2. The handle's handler binding is missing, malformed, or not admitted.
3. The handle is used before the binding that introduced it.
4. The run/session/actor/turn scope does not match the request.
5. The handle is expired, revoked, or outside `not-before` bounds.
6. The operation is not in the handle's allowed operation set.
7. The request's capability, authority, policy, or resource refs do not match the handle and handler binding.
8. A local-only handle appears in remote, transferred, replay-imported, or peer-proxy context without explicit attenuation or remote-proxy evidence.
9. Two same-kind effects are present and the request relies on effect kind alone instead of a handle ref.

Denials must emit canonical receipts before side effects.

## Multiple same-kind effects

The primary practical win is unambiguous same-kind effects. Examples:

- two storage roots in one job,
- one public blob store and one confidential blob store,
- deterministic fixture clock plus production wall-clock adapter in a profiling run,
- two remote peers offering the same sync protocol,
- one replay effect log and one record effect log in a migration suite.

The effect kind describes the interface class; the handle ref names the concrete admitted surface.

## Compound and dynamic handlers

A compound handler profile may expose related handles, such as:

- storage + blob + trace,
- peer sync + gossip + docs,
- executor hostcalls + fuel/resource accounting,
- replay input + record output.

Compound evidence should list child handle refs and shared policy/capability/resource evidence. A compound handler may leave some operations dynamic, but each dynamic operation must still be represented by canonical operation names, reviewed callable or adapter refs, and deterministic request/response evidence.

## Remote handles

Remote handles are dangerous because they can blur identity, authority, transport, and lifetime. Default behavior is local-only:

- A local handle ref is not transferable by merely sending the ref to a peer.
- Remote use requires authority context, peer bootstrap agreement, resource limits, revocation policy, and remote-proxy handler evidence.
- Remote-proxy handles should be attenuated: narrower operation set, narrower scope, shorter expiry, and explicit peer/node identity binding.
- Imported replay bundles may reference historical handle refs but cannot use them for new side effects.

## Integration points

- `executor-hostcall-boundary`: hostcall requests bind handle refs before executor output can affect state.
- `capability-context-admission` and `authority-identity-revocation`: handles point at authority evidence but do not replace it.
- `resource-governance-backpressure`: handles bind allowed resource grants and consumption receipts.
- `peer-bootstrap-negotiation` and `federated-pull-sync`: remote handles require negotiated features and receiver-driven policy.
- `chain-hashed-evidence-ledger`: handler and handle lifecycle receipts can be linked into scoped evidence chains.
- `first-class-testing-harness`: harness fixtures can define multiple handles of the same kind and replay their exact use.

## Costs and mitigations

- Evidence size grows. Mitigate by storing full handle/binding artifacts once and referencing hashes in repeated requests.
- Validator complexity grows. Mitigate by centralizing handle validation and requiring small, explicit schemas.
- Actor ergonomics may suffer. Mitigate by applying handles only at trust/effect boundaries, not throughout pure code.
- Handle refs can be mistaken for authority. Mitigate with schema checks, documentation, and denial tests proving handle-only evidence is insufficient.
- Migration may require compatibility parsers. Mitigate by first adding optional handle refs to new surfaces, then making them mandatory for evidence-bearing adapters.

## Open Questions

- Which existing hostcall/effect envelopes should gain mandatory handle refs first: harness hostcalls, storage/blob adapters, or remote-sync handlers?
- Should turn-scoped handles be common, or should most handles be session/run-scoped with per-turn resource consumptions?
- How should GC treat expired handle artifacts that remain necessary for historical replay?
- Should handle refs be included directly in chain-link payload context for turn journals and adapter lineage?
