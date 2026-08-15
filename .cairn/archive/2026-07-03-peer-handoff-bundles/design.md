# Design: peer handoff bundles

## Scope

This change generalizes node-control live workflow bundles into a reusable peer handoff artifact. It improves operator workflows and subsystem reuse while preserving the fail-closed evidence model.

## Bundle model

`peer-handoff-bundle-v1` contains:

- bundle schema and profile identity,
- issuer/receiver node ids and peer ids,
- expected endpoint/topic/docs/job/sync scope,
- ticket refs or embedded ticket values,
- peer admission/session/agreement refs,
- accepted capability refs,
- policy and resource refs,
- optional authority grant refs for named operations,
- freshness/epoch/revocation metadata,
- supporting receipt refs and embedded review artifacts,
- checks proving member refs and expected bindings match.

The bundle ref is over canonical Preserves bytes. Embedded members are imported only after verify/gate/apply checks pass.

## Verify, gate, import, apply

`verify` is offline and checks structure, member refs, duplicate/malformed members, scope binding, freshness, and expected peer/node/topic values. `gate` repeats verification and may require a current matching verify receipt. `import` stores allowed members in the sender state root while marking import receipts non-authority. `apply` runs a subsystem preflight; live send or remote work occurs only when the subsystem command explicitly requests it.

## Consumers

Node-control keeps its existing live workflow commands but emits/accepts the generic bundle as the canonical reusable form. Remote dataspace, job worker, retention clearance, and remote artifact sync consumers may accept peer handoff refs only when the bundle scope matches their operation and their normal gates still pass.

## Functional core

The core parses and validates bundles, computes binding diagnostics, extracts importable members, and returns deterministic decisions. Shells own file IO, state-root mutation, live transport calls, and operator rendering.

## Non-goals

- No bundle-level authority grant without matching authority grant artifacts.
- No automatic live send during verify, gate, or default apply dry-run.
- No bypass of subsystem provenance, source-gate, retention, execution, policy, resource, or replay requirements.
- No claim that peer handoff bundles create global peer discovery or global consistency.
