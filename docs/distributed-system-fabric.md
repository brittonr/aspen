# Distributed-system fabric boundary

r[impl molten.fabric_boundary.fabric_identity] r[impl molten.fabric_boundary.mechanism_semantics_separation] r[impl molten.fabric_boundary.extension_tiers] r[impl molten.fabric_boundary.port_registry] r[impl molten.fabric_boundary.evidence_granularity] r[impl molten.fabric_boundary.reference_system_exit_criteria] r[impl molten.fabric_boundary.non_claims]

Molten is a workload-neutral distributed-systems fabric. It supplies canonical communication, lifecycle, authority, resources, execution, durability, transport, scheduling, supervision, policy, simulation, and evidence mechanisms. A database, replicated log, queue, scheduler, object store, federation service, or workflow engine remains a system extension or workload; none defines global Molten semantics.

## Ownership law

| Layer | Owns | Must not own |
| --- | --- | --- |
| Primitive | Pure deterministic validation, transition laws, canonical identity inputs, bounds, plans, and non-claim decisions | Files, sockets, clocks, entropy, processes, ambient mutable state, backend handles, or workload-specific side effects |
| Adapter | Capability-rooted external effects and translation between a typed port and one substrate | Authority grants, canonical semantic identity, consistency policy, retry policy, placement policy, or workload semantics |
| System extension | Optional long-lived service protocol, durable state machine, membership/placement policy, delivery policy, replication policy, and service receipts | Ambient authority, silent port fallback, bypasses around admission, or promotion of its semantics into node core |
| Application/workload | User-facing behavior composed from admitted services | Direct system-extension authority or adapter access inferred from artifact possession |

A useful test for a primitive is: **can its decisions be tested from in-memory facts without standing up the world?** If not, the external observation or effect belongs behind a port. A useful test for an adapter is: **would replacing the substrate leave the semantic transition law unchanged?** If not, policy has leaked into the adapter.

## Extension tiers

Molten distinguishes three tiers:

1. A **sandboxed plugin** receives only declared plugin hostcalls. An operation string that looks like storage, networking, timers, membership, placement, or consistency does not grant those powers.
2. A **system extension** is an optional supervised service. Activation requires a canonical system-extension manifest, passing policy and provenance evidence, exact fabric-port bindings, resource grants, and lifecycle admission.
3. An **application/workload** consumes admitted service APIs. It does not receive protocol ownership or direct durable-state, timer, membership, placement, consistency, or adapter authority.

The pure tier validator in `crates/molten-core/src/fabric/tier.rs` rejects system authority outside the system-extension tier and rejects incomplete system-extension evidence. The Preserves projection in `src/fabric/mod.rs` emits canonical tier-admission evidence; the Rust structure itself is not canonical authority.

## Fabric-port registry

A fabric port is a mechanism contract, not a backend object. Its canonical `fabric-port-descriptor-v1` value binds:

- stable port id and exact version;
- mechanism class and operation classes;
- canonical input and output schema ids;
- required authority and resource classes;
- determinism and replay classes;
- one selected implementation profile;
- BLAKE3 conformance refs;
- explicit non-claims and enabled state.

Activation follows a fail-closed sequence:

```text
validate descriptors
  -> reject malformed or duplicate (port id, version) keys
  -> resolve the exact requested id and version
  -> require exact class, schemas, determinism, replay, and profile
  -> require every descriptor authority and resource to be admitted
  -> reject disabled, unknown, incompatible, over-authorizing, or fallback candidates
  -> emit canonical descriptor, registry, and binding refs
```

Different versions or profiles are diagnostics, not fallback candidates. A binding proves only that a reviewed mechanism contract matched the request. It does not prove delivery, durability, consensus, correctness, compatibility, or readiness. Adapter runtime objects such as Iroh endpoints, Redb transactions, filesystem paths, and simulator handles never enter the canonical descriptor.

## Evidence granularity

Trust, lifecycle, semantic commit, checkpoint, failure, and operator-observation boundaries emit canonical evidence. Internal page reads, packets, scheduler polls, and cache lookups use a reviewed bounded aggregate or are omitted. A diagnostic profile may select per-operation evidence, but that profile remains diagnostic and cannot become a production default through fallback.

The pure profile validator requires:

- all semantic boundaries to use canonical boundary receipts;
- all evidence boundaries to be declared exactly once;
- production profiles not to require canonical receipts for every internal operation;
- every bounded aggregate to name a canonical BLAKE3 limit-profile ref;
- all fabric non-claims to remain explicit.

## Reference-system exit criteria

The reference systems are conformance witnesses, not Molten product identities.

| Reference system | Common fabric mechanisms | Extension-owned semantics |
| --- | --- | --- |
| Transactional key-value service | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Transaction isolation and conflict resolution |
| Replicated log | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Consumer offsets and log retention |
| Distributed scheduler | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Scheduling policy and task ownership |

A matrix fails if a common capability is missing, a workload semantic is assigned to core, or implementation uses ambient filesystem, network, clock, process, or adapter access. A failure identifies a missing general primitive; it does not justify an extension-specific shortcut in core.

## Consensus boundary

Molten does not require one global coordination mechanism. Normal actor traffic, blob transfer, DAG exchange, federation, and unrelated service state do not pass through a mandatory consensus engine. A consistency service is selected through explicit ports and remains an optional system extension.

OpenRaft is not selected, adapted, or used. Native deterministic transition laws and any Trellis-backed proof or conformance support remain independent of transport, storage, time, entropy, leadership, and implementation-specific DTOs. A future consensus extension must pass its own reviewed Cairn change and cannot transfer local proof claims to whole-system correctness.

## Canonical identity and adapters

Canonical identity is the BLAKE3 hash of canonical Preserves bytes. Rust structs are in-memory validation inputs only; field layout, debug text, enum discriminants, backend ids, tickets, paths, endpoint ids, and transport frames are not canonical authority.

Adapters may implement ports with Iroh, Redb, capability-rooted filesystems, logical-time sources, cryptographic entropy, tracing exporters, deterministic simulators, or later reviewed substrates. An adapter reports effects and failures but does not decide who is authorized or what a workload operation means.

## Project licensing boundary

Repository-owned Molten source is licensed under `AGPL-3.0-or-later`; see the root `LICENSE`. Third-party dependencies, vendored manifest snapshots, generated artifacts containing upstream material, and external references retain their original terms and notices; see `THIRD_PARTY_LICENSES.md`.

Earlier permissive grants remain valid. The current declaration applies prospectively to project-owned contributions and does not revoke rights already received. Compatibility remains driven by requirements, protocols, independently authored fixtures, and conformance tests. Reading an external implementation does not transfer its correctness, security, authority, or proof claims.

## Non-claims

Fabric descriptors, bindings, simulations, receipts, and reference matrices do not by themselves prove:

- database or extension semantic correctness;
- global ordering or global consensus;
- transport delivery or durable persistence;
- Byzantine tolerance;
- protocol or API compatibility;
- production or release readiness.

Those claims require separately scoped implementation, conformance, simulation, operational, and release evidence. Lower-scope evidence must not silently satisfy a stronger claim.
