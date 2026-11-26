# Aspen distributed-system fabric boundary

<a id="distributed-system-fabric-boundary"></a>

r[impl molten.fabric_boundary.fabric_identity] r[impl molten.fabric_boundary.mechanism_semantics_separation] r[impl molten.fabric_boundary.extension_tiers] r[impl molten.fabric_boundary.port_registry] r[impl molten.fabric_boundary.evidence_granularity] r[impl molten.fabric_boundary.reference_system_exit_criteria] r[impl molten.fabric_boundary.non_claims]

Aspen is a workload-neutral distributed-systems fabric. It supplies canonical communication, lifecycle, authority, resources, execution, durability, transport, scheduling, supervision, policy, simulation, and evidence mechanisms. A database, replicated log, queue, scheduler, object store, federation service, or workflow engine remains a system extension or workload; none defines global Aspen semantics.

## Ownership law

| Layer | Owns | Must not own |
| --- | --- | --- |
| Primitive | Pure deterministic validation, transition laws, canonical identity inputs, bounds, plans, and non-claim decisions | Files, sockets, clocks, entropy, processes, ambient mutable state, backend handles, or workload-specific side effects |
| Adapter | Capability-rooted external effects and translation between a typed port and one substrate | Authority grants, canonical semantic identity, consistency policy, retry policy, placement policy, or workload semantics |
| System extension | Optional long-lived service protocol, durable state machine, membership/placement policy, delivery policy, replication policy, and service receipts | Ambient authority, silent port fallback, bypasses around admission, or promotion of its semantics into node core |
| Application/workload | User-facing behavior composed from admitted services | Direct system-extension authority or adapter access inferred from artifact possession |

A useful test for a primitive is: **can its decisions be tested from in-memory facts without standing up the world?** If not, the external observation or effect belongs behind a port. A useful test for an adapter is: **would replacing the substrate leave the semantic transition law unchanged?** If not, policy has leaked into the adapter.

## Extension tiers

Aspen distinguishes three tiers:

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

The reference systems are conformance witnesses, not Aspen product identities.

| Reference system | Common fabric mechanisms | Extension-owned semantics |
| --- | --- | --- |
| Transactional key-value service | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Transaction isolation and conflict resolution |
| Replicated log | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Consumer offsets and log retention |
| Distributed scheduler | Authority, resources, durable state, transport, scheduling, simulation, membership, placement, consistency, time, supervision, policy, evidence | Scheduling policy and task ownership |

A matrix fails if a common capability is missing, a workload semantic is assigned to core, or implementation uses ambient filesystem, network, clock, process, or adapter access. A failure identifies a missing general primitive; it does not justify an extension-specific shortcut in core.

## Consensus boundary

Aspen does not require one global coordination mechanism. Normal actor traffic, blob transfer, DAG exchange, federation, and unrelated service state do not pass through a mandatory consensus engine. A consistency service is selected through explicit ports and remains an optional system extension.

OpenRaft is not selected, adapted, or used. Native deterministic transition laws and any Trellis-backed proof or conformance support remain independent of transport, storage, time, entropy, leadership, and implementation-specific DTOs. A future consensus extension must pass its own reviewed Cairn change and cannot transfer local proof claims to whole-system correctness.

### Long-running Raft service bounds

The live reliability registry (`live_reliability::service`) and the collaboration-scope native runtime are long-running Raft services, not bounded qualification runs. Their limits are rates, concurrency limits, size thresholds, or retention windows; none is a lifetime total. A service stops only on an operator stop or teardown request, cancellation, a failed replica effect (fail-stop), or a transport or listener failure that ends Raft ingress.

- **Snapshots.** `ReplicaProfile.snapshot_threshold_entries` must be positive and below `max_log_entries`. After each applied event the host checks `is_snapshot_due` and, once a running voter retains that many applied entries above its snapshot, issues `CreateSnapshot` with its application state ref. The reliability registry uses 2,048 of 4,096 entries. A denied or failed snapshot ends the reliability host and marks the collaboration runtime uncertain; it is never ignored.
- **Follower log bound.** A follower appends from an `AppendEntries` batch only the entries that fit `max_log_entries` after truncating any conflicting suffix, and acknowledges exactly that prefix. Commit advancement through it lets the host's snapshot check compact the log; the leader resends the remainder from the acknowledged index. A full follower therefore never rejects or fails on valid replication.
- **Effects per step.** Leader commit advancement commits at most as many entries as `max_effects_per_step` leaves room for beside the commit record, the application batch and the executor's two durable intent fences (reliability: 128 effects, so at most 124 entries per step). Each later successful append response, such as a heartbeat acknowledgement, continues from the new commit index.
- **Timers.** The live time port cancels the election timer when it arms heartbeats and the heartbeat timer when it arms an election timer. An already-queued election timeout whose token was superseded, an election timeout reaching a leader, or a heartbeat tick reaching a non-leader is a benign no-op (`stale_timer_event`): the transition keeps the state and plans no effects, and the reliability host skips it without failing stop.
- **Protocol liveness.** A follower acknowledges a current-term `ReadProbe` whether or not its log has reached the probe's read index, adopting a higher term first. `AppendEntries` checks duplicate request refs only against local entries the merge retains, so a healed former leader truncates an uncommitted suffix that holds a retried request and accepts the new leader's committed copy. Recovery snapshot installation first checks committed evidence against the snapshot: peer-reported committed copies and locally committed entries must match the boundary term and, within the duplicate window, the request the snapshot records at each index. Same index and term with another request among committed evidence is divergence: recovery fails closed with a `malformed-observation` failure and neither rewrites the log nor installs the snapshot. Uncommitted local entries are suspect, not evidence. The local suffix is kept when the local boundary entry matches the snapshot's term and recorded request. Otherwise the log is repaired from peer reports before the snapshot is written: the faulty boundary and earlier entries take the committed copy the reporting peers agree on, suffix entries with a differing committed peer copy are replaced by that copy, peer-confirmed entries are never dropped, and only entries peers establish as uncommitted are truncated. The repair is one durable log mutation that also rewrites the confirmed tail, so a crash before the snapshot replays either the original or the repaired, term-ordered log. While any suffix entry is unresolved, a needed committed copy has not been reported, or the repaired log would still break term order, installation waits and the local evidence is kept.
- **Duplicate requests.** `request_retention_entries` is the window W (reliability: 256). After applying index i a replica retains exactly the request refs committed in (i − W, i], so replicas at one applied index make identical duplicate decisions whatever their snapshot timing. A retry is recognized as a duplicate, returning its original committed index without a new append, only while that commit is among the last W applied entries; an older retry is a new request and is applied again. Snapshots carry the window at their boundary, installation restores it, and a snapshot holding a ref outside (boundary − W, boundary] is rejected. Service admission encodes a complete maximum-width window and rejects configurations exceeding the selected protocol payload frame before storage/network effects; the selected frame remains 65,536 bytes, with its eight-byte prefix accounted separately.
- **Durable records.** When the durable namespace declares `retention_authority_ref` and the Raft durability port is given that authority (the reliability mechanism derives it from the service, namespace and signed registration authority), each synchronized snapshot is followed by a bounded compaction header, one existing log-mutation record per retained entry, and a matching completion footer. The header carries the hard state and durable commit boundary; no record grows with the suffix. Only a complete base permits snapshot retirement and truncation of covered records. An interrupted untruncated base retains the original complete suffix and recovery uncertainty; a truncated incomplete base is rejected. Storage holds the newest snapshot, its complete base and subsequent records within the namespace limits. Metadata retirement covers every snapshot ref, while each unreferenced shared payload is deleted once. Election churn without commits is not compacted. Without retention authority every record/snapshot remains, so namespace limits bound the service's lifetime.
- **Ingress.** `delivery_limit` deliveries are admitted per `delivery_window` (reliability: 4,096 per second; the live-process harness: 1,024 per second). A spent budget pauses polling until the window ends. A frame whose payload fails to decode is dropped, counted in `ReplicaIngressPump::dropped_frames` and logged under the `molten_fabric_consistency` tracing target; any other listener error ends ingress with that error.
- **Public connections.** The reliability host's `max_connections` (reliability: 16, admitted 1–256) bounds connections served at once. At the bound the host stops accepting and further connections wait in the listen backlog.

## Canonical identity and adapters

Canonical identity is the BLAKE3 hash of canonical Preserves bytes. Rust structs are in-memory validation inputs only; field layout, debug text, enum discriminants, backend ids, tickets, paths, endpoint ids, and transport frames are not canonical authority.

Adapters may implement ports with Iroh, Redb, capability-rooted filesystems, logical-time sources, cryptographic entropy, tracing exporters, deterministic simulators, or later reviewed substrates. An adapter reports effects and failures but does not decide who is authorized or what a workload operation means.

## Project licensing boundary

Repository-owned Aspen source is licensed under `AGPL-3.0-or-later`; see the root `LICENSE`. Third-party dependencies, vendored manifest snapshots, generated artifacts containing upstream material, and external references retain their original terms and notices; see `THIRD_PARTY_LICENSES.md`.

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
