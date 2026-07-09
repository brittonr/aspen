# Effect manifest handler profiles

Molten adapts Unison's effect visibility principle without adopting Unison syntax, typechecking, runtime semantics, or compatibility. Executable artifacts declare possible effects in canonical `effect-manifest-v1` records; concrete handler profiles require admission before any side-effecting shell can execute a request.

## Boundary

The pure effects core now records and validates:

- declared effect id and operation;
- input and output schema refs;
- resource class;
- required capability refs;
- manifest policy and evidence refs;
- handler profile policy, capability context, resource refs, determinism class, replay class, and evidence refs.

`admit_handler_profile_for_manifest` emits `handler-profile-admission-receipt-v1` evidence that a profile supports every declared effect with matching schemas, resource class, and capabilities under current policy/capability context. Stale policy refs, stale or revoked capability context, missing handler bindings, missing resource bounds, missing evidence, and schema/resource/capability mismatches deny.

`admit_effect_request` still performs request-level admission and now also denies requests missing capabilities declared by the effect manifest. Denials happen before handler invocation.

`bind_effect_profile_replay_evidence` emits exact effect-manifest, handler-profile, and profile-admission refs for replay, transcript, evaluation-cache, job DAG, and remote-execution evidence. Profile or manifest drift denies unless explicit compatibility evidence is bound.

## Fixtures

Focused effects tests cover:

- declared manifests with resource classes and capability needs;
- passing handler profile admission;
- undeclared and missing-capability request denial;
- stale context and schema/resource/capability mismatch denial;
- replay/cache profile drift denial;
- Unison runtime compatibility claim denial.

Run focused evidence with:

```sh
nix develop -c cargo test actions::tests --lib
```
