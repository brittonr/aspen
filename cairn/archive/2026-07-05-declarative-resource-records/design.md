# Design: declarative resource records

## Context

This change adapts Kubernetes' resource-model discipline, not its API surface. Molten resources remain canonical Preserves values admitted through Nickel, Basalt/UCAN, Steel/Trellis predicates, and evidence receipts.

## Resource shape

A resource record should bind:

- `resource_type`: reviewed domain/type/version identifier.
- `resource_ref`: BLAKE3 ref over canonical resource identity bytes.
- `scope_ref`: tenant, node, workflow, authority, or local scope ref.
- `name`: deterministic human-facing name within the scope.
- `generation`: desired-state generation advanced by admitted desired-state changes.
- `desired_ref`: canonical desired-state body ref.
- `observed_ref`: optional observed-state body ref.
- `metadata`: labels, annotations, owner refs, finalizers, and evidence refs.
- `status`: observed generation and condition records.

The record is an envelope payload pattern. It does not imply Kubernetes object schemas, JSON merge patches, YAML admission, REST paths, etcd storage, or CRD compatibility.

## Functional core

Pure core functions should validate resource identity, metadata bounds, generation transitions, condition transitions, owner/finalizer invariants, and deletion eligibility over in-memory Preserves-derived DTOs. The core returns either a normalized candidate record or denial diagnostics.

## Imperative shell

The shell reads and writes stores, computes canonical refs, asks policy/authority gates, publishes dataspace assertions, and emits receipts. It does not hide status or deletion decisions in logs.

## Lifecycle notes

Status updates must carry the generation they observed. Finalizers block deletion until matching cleanup receipts are present. Owner refs permit GC only when authority, finalizer, retention, and pin evidence all agree.
