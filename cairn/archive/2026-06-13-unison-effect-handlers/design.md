## Context

Molten's runtime spine admits all side effects through adapters. Wasmtime actors have narrow hostcalls, Steel scripts operate through public runtime APIs, Iroh and Redb are adapter-owned effects, and policy gates sit before side effects. This is already close to an effect-handler model, but the effect surface needs to be explicit, typed, searchable, and testable.

Unison abilities show a useful separation: business logic mentions an ability, and handlers decide how the ability runs. Molten should express that separation as artifact manifests, capability checks, adapter handler bindings, and receipt-backed traces.

## Goals

- Make required effects explicit in executable artifact metadata.
- Fail artifact installation or execution early when required effects cannot be handled with available capabilities.
- Support handler profiles for local tests, chaos simulation, profiling, and production deployment.
- Keep handlers behind envelope admission, capability checks, and receipt emission.
- Make effect use visible in canonical trace records.
- Allow docs/transcripts to pin the handler profile used to reproduce an example.

## Non-Goals

- Do not implement Unison abilities or Unison syntax.
- Do not require first-class continuations or generalized algebraic-effect handling in Rust.
- Do not let handlers bypass Molten envelope, dataspace, policy, storage, or evidence boundaries.
- Do not make ambient filesystem, network, time, random, process, or environment access available by default.
- Do not treat a declared effect as sufficient authority; capabilities and policies still admit each use.

## Effect manifest

An executable artifact should declare an effect manifest with entries such as:

- `DataspaceSend(subject_schema, payload_schema)`
- `DataspaceObserve(pattern_schema)`
- `BlobGet(content_ref_scope)`
- `BlobPut(size_limit, media_policy)`
- `RemoteInvoke(protocol_or_artifact_scope)`
- `StorageRead(schema_ref, namespace_scope)`
- `StorageWrite(schema_ref, namespace_scope)`
- `HttpClient(origin_policy)`
- `Clock(clock_kind)`
- `Random(random_kind)`
- `SpawnActor(artifact_scope)`
- `Trace(record_schema)`
- `PolicyAsk(contract_scope)`

Each entry should include schema refs, resource scopes, default deny behavior, and whether it is deterministic under a test handler.

## Handler binding

A handler binding maps one declared effect to an adapter implementation under a capability and policy context:

```text
artifact_id + effect_id + handler_profile + capability set
        -> admitted handler binding receipt
        -> executable adapter handle
```

Handler profiles may be:

- `production`: real Iroh, Redb, dataspace, Wasmtime hostcalls, and approved external services.
- `local`: in-process deterministic implementations for tests.
- `mock`: scripted responses declared as Preserves values.
- `chaos`: fault, delay, reorder, and partition injection within declared bounds.
- `profiling`: records resource use, network estimates, and trace timing without changing semantics where possible.
- `dry_run`: validates effect requests and produces planned receipts without performing external side effects.

Bindings are installed per actor/job/session, not globally ambient.

## Runtime effect request

Effect requests cross the envelope spine. A request should carry:

- artifact id and execution id,
- effect id from the manifest,
- handler profile id,
- canonical input Preserves value or content ref,
- presented capabilities,
- sequence/replay metadata,
- policy and prior evidence refs.

The handler validates the request against the manifest, capabilities, and policies, then either denies with a receipt or performs the adapter effect and emits a success receipt and trace record.

## Wasmtime and Steel

Wasmtime hostcalls should be generated or checked against the effect manifest. A Wasm component cannot call a host function that is not declared, bound, admitted, and represented by a capability.

Steel scripts and predicates may orchestrate effect requests only through public runtime APIs. A reviewed Steel predicate can be used as a dynamic contract backend, but the predicate itself must have an artifact id, declared effects, policy refs, and receipts.

## Distributed testing and profiling

Local and chaos handlers let distributed programs run without a live cluster. For example, `RemoteInvoke` can be interpreted by an in-process scheduler; `BlobPut` can store into a temporary content map; `Clock` can use logical time; and `IrohGossip` can be modeled as a deterministic queue with injected partitions. Trace records should make handler choices explicit so test transcripts are reproducible.

## Open Questions

- Should effect manifests be authored directly as Preserves/Nickel or derived from WIT/component metadata where possible?
- Which effects are small enough for the first milestone: dataspace send/observe, blob get/put, storage read/write, clock, random?
- How should handler profiles compose when one artifact calls another with a narrower effect set?
- What is the minimum evidence required to trust a mock or chaos handler result in a Cairn receipt?
