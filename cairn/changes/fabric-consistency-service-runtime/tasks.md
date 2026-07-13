## Phase 1: Correct profile scope and admission

- [x] [serial] Broaden explicit consistency-group scope to admitted system-extension state while preserving opt-in use and ordinary-message bypass. r[molten.consensus.scope]
- [x] [serial] Reclassify `in-process-raft-control-registry-v1` as model or simulation only and deny it for production runtime selection. r[molten.consensus.algorithm_profile_manifest]
- [x] [parallel] Add positive model-profile fixtures and negative production-selection, silent-fallback, missing-live-evidence, and claim-overreach fixtures. r[molten.consensus.algorithm_profile_manifest] r[molten.fabric_consistency.production_admission]

## Phase 2: Extension-facing consistency port

- [ ] [serial] Add canonical group create or attach, propose, read, snapshot, recover, configuration, health, drain, and status commands and outcomes for system extensions. r[molten.fabric_consistency.extension_port]
- [ ] [parallel] Bind every group and operation to extension/service generation, application state-machine manifest, engine profile, membership/config epoch, placement, fencing, resources, policy, and non-claims. r[molten.fabric_consistency.group_isolation]
- [ ] [parallel] Add positive group fixtures and negative cross-extension, stale-generation, stale-epoch, unsupported-read, unsupported-config, and over-resource fixtures. r[molten.fabric_consistency.extension_port] r[molten.fabric_consistency.group_isolation]

## Phase 3: Live engine service shell

- [ ] [serial] Connect consensus engine instances to admitted transport, durable-log, snapshot, time, entropy, membership, placement, fencing, supervision, and resource ports. r[molten.fabric_consistency.live_service_ports]
- [ ] [serial] Implement the first live Raft service profile with protocol registration, elections, replication, commit, reads, snapshots, recovery, and bounded static membership before admitting wider transitions. r[molten.fabric_consistency.live_raft]
- [ ] [parallel] Preserve pure application state machines and prevent engine internals from entering extension state identity or semantics. r[molten.fabric_consistency.group_isolation]

## Phase 4: Distributed evidence and operations

- [ ] [serial] Add multi-process fixtures for quorum formation, commit, quorum-backed read, partition, quorum loss, crash/restart, snapshot catch-up, and stale-leader fencing using distinct endpoints and durable namespaces. r[molten.fabric_consistency.production_admission]
- [ ] [parallel] Add bounded group admission, configuration, selected commit, read-currentness, snapshot, recovery, failure, and aggregate health evidence without per-heartbeat receipts. r[molten.fabric_consistency.evidence_granularity]
- [ ] [parallel] Add operator readback and bounded create, inspect, drain, snapshot, recover, and remove workflows with dry-run preflights. r[molten.fabric_consistency.operator_readback]

## Phase 5: Validation

- [ ] [serial] Run shared engine conformance, deterministic simulation, multi-process live, failure, recovery, fencing, extension-isolation, policy, resource, non-claim, and model-profile denial tests. r[molten.fabric_consistency.final_validation]
- [ ] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_consistency.final_validation]

## Blocker

Phases 2–5 remain blocked on a reusable admitted cross-process transport shell. The
current `IrohTransportAdapter` owns only `live_loopback_frame`, which creates both
endpoints inside one process and returns after one echo. It cannot register a
long-lived listener, publish or consume an endpoint address, connect distinct
node processes, or route consensus protocol frames between supervised replicas.
Using it for a multi-replica fixture would therefore be same-process simulation,
which this change explicitly forbids as production evidence. Resume only after
the transport port has a bounded cross-process listener/session implementation
and distinct-process conformance evidence; do not substitute ambient sockets or
fabricated quorum receipts.
