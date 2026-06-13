## Context

Molten's runtime spine routes canonical envelopes through local dataspace adapters, remote Iroh bridges, execution adapters, policy gates, and receipt stores. That gives a message substrate, but not a global description of which multi-party conversations are legal. A choreography layer supplies that missing protocol contract.

Trellis provides the right foundation for the first version:

- `choreography_global::GlobalChoreo` and `GlobalBranch` for finite global protocols.
- `choreography_local::LocalChoreo` and `LocalBranch` for projected endpoint protocols.
- `choreography_projection::projectable` and `project_endpoint` for admission and endpoint generation.
- `choreography_step` helpers for one-step global and local semantics.

Molten should not duplicate those semantics. Molten should add names, schemas, envelope integration, dataspace execution, and policy/evidence boundaries around them.

## Goals

- Represent Molten protocols as finite Trellis-backed global choreographies.
- Preserve a clear boundary between pure choreography validation/projection and effectful runtime interpretation.
- Map human-readable protocol manifests to role ids, label ids, and payload tags deterministically.
- Project per-role endpoint state before execution and persist enough metadata to inspect what each actor is allowed to do next.
- Execute projected local endpoints over the dataspace without bypassing envelope admission, capability checks, or receipt emission.
- Make branch selection explicit, auditable, and visible to non-decider roles through the projected local protocol.

## Non-Goals

- Do not implement recursive, unbounded, or higher-order choreographies in the first version.
- Do not prove application-level payload semantics from choreography alone.
- Do not replace the dataspace, Iroh, Basalt, Cairn, Nickel, Steel, Wasmtime, or Redb layers.
- Do not claim ChoRus compatibility or depend on `chorus_lib` for protocol execution.
- Do not allow protocol interpreters to publish dataspace assertions directly without policy admission.

## Architecture

```text
Protocol manifest / DSL
  named roles, labels, payload schemas, metadata
        |
        v
Molten choreography compiler
  deterministic role/label/payload registries
  lowers to Trellis GlobalChoreo
        |
        v
Trellis admission
  well_formed_global + projectable
  project_endpoint(role, global)
        |
        v
Molten endpoint runtime
  LocalChoreo state machine per role/session
  dataspace-backed send/receive/choice/offer interpreter
        |
        v
Runtime envelope spine
  Preserves payloads, blob refs, capabilities, evidence refs
  Basalt/Nickel/Steel/Trellis/Cairn policy and receipts
        |
        v
Local dataspace, Iroh bridge, Wasmtime/Steel/native actors, Redb store
```

## Protocol Manifests

A manifest should contain:

- `protocol_id`: stable id or content hash for the protocol definition.
- `roles`: unique human-readable names mapped to Trellis `RoleId` values.
- `labels`: unique protocol-local labels mapped to Trellis `LabelId` values.
- `payloads`: payload names, schema ids, canonical encoding, and Trellis `PayloadTag` values.
- `global`: finite global choreography expressed in manifest syntax.
- `policy`: references to Nickel contracts, Basalt contract ids, required capabilities, and receipt rules.

The compiler lowers the manifest to a Trellis `GlobalChoreo`. The lowered result, manifest hash, role map, label map, payload registry, and policy references become the protocol installation artifact.

## Endpoint Runtime

For each active protocol session, each participant stores:

- protocol id and session id,
- local role id,
- projected `LocalChoreo`,
- current local step state,
- next expected send/receive/choice/offer shape,
- monotonic operation or sequence counter,
- evidence references for installation and admission decisions.

The interpreter handles Trellis local nodes as follows:

- `Send`: validate capability and policy, construct a protocol-message envelope, publish it through the dataspace, record receipt, and advance with `local_send_step`.
- `Recv`: subscribe/wait for a matching protocol-message envelope from the expected peer with the expected label and payload tag, validate payload and evidence, then advance with `local_recv_step`.
- `InternalChoice`: require an admitted local decision, publish the selected label as the next protocol message or choice evidence, record receipt, and advance with `local_internal_choice_step`.
- `Offer`: wait for the decider's selected label/evidence, validate it against the available branches, then advance with `local_offer_step`.
- `End`: no further protocol messages are admitted for the session except inspection or cleanup receipts.

## Dataspace Message Shape

The runtime envelope body should contain or reference a protocol message with at least:

- `protocol_id`
- `session_id`
- `from_role`
- `to_role`
- `label`
- `payload_tag`
- `op_index` or effect id
- `body` as canonical Preserves data or `content_ref` for large values
- `projection_hash` or protocol artifact hash
- policy and receipt evidence references

The dataspace adapter routes by protocol id, session id, target role, label, and payload tag. Large bodies use content references and Iroh blobs; the protocol message carries the reference and content-integrity evidence.

## Policy and Evidence

Protocol installation is a trust-boundary action. It must require:

- Trellis projectability result for the lowered global choreography.
- Nickel contract admission for static manifest, schema, role, payload, and adapter policy.
- Basalt enforcement of required capabilities for installation and later sends/effects.
- Cairn validation of installation receipts and per-operation receipts.
- Trellis bounded predicates for replay/sequence, routing limits, role membership, and content integrity where applicable.

Runtime send, receive, branch choice, and external effects must produce evidence that identifies the protocol id, session id, local role, local step, selected label if any, policy decision, and receipt reference.

## Open Questions

- Should role/label ids be assigned by sorted manifest order, explicit manifest values, or content-hash-derived ids?
- Should a branch decision be encoded as a zero-payload protocol message or as a separate choice receipt referenced by subsequent sends?
- Should protocol artifacts be stored as Preserves values, JSON projections, or both?
- How much endpoint state should be durable before the Redb adapter lands?
