## Phase 1: Correct profile scope and admission

- [x] [serial] Broaden explicit consistency-group scope to admitted system-extension state while preserving opt-in use and ordinary-message bypass. r[molten.consensus.scope]
- [x] [serial] Reclassify `in-process-raft-control-registry-v1` as model or simulation only and deny it for production runtime selection. r[molten.consensus.algorithm_profile_manifest]
- [x] [parallel] Add positive model-profile fixtures and negative production-selection, silent-fallback, missing-live-evidence, and claim-overreach fixtures. r[molten.consensus.algorithm_profile_manifest] r[molten.fabric_consistency.production_admission]

## Phase 2: Extension-facing consistency port

- [x] [serial] Add canonical group create or attach, propose, read, snapshot, recover, configuration, health, drain, and status commands and outcomes for system extensions. r[molten.fabric_consistency.extension_port]
- [x] [parallel] Bind every group and operation to extension/service generation, application state-machine manifest, engine profile, membership/config epoch, placement, fencing, resources, policy, and non-claims. r[molten.fabric_consistency.group_isolation]
- [x] [parallel] Add positive group fixtures and negative cross-extension, stale-generation, stale-epoch, unsupported-read, unsupported-config, and over-resource fixtures. r[molten.fabric_consistency.extension_port] r[molten.fabric_consistency.group_isolation]

## Phase 3: Live engine service shell

- [x] [serial] Connect consensus engine instances to admitted transport, durable-log, snapshot, time, entropy, membership, placement, fencing, supervision, and resource ports. r[molten.fabric_consistency.live_service_ports]
  - Evidence: the pure startup plan and thin host projection require the exact active group, canonical group integrity, running service generation, admitted policy cohort, static membership, timer/resource bounds, and canonical transport/durable-log/snapshot/time/entropy/membership/placement bindings. The effect shell executes durability before transport in declared order, publishes state only after all effects pass, and retains partial evidence on failure. The concrete cohort constructor rejects adapter substitution across Redb durability/snapshots, Iroh protocol transport, Tokio time/production entropy, application, and supervision, then executes bound startup through the scoped service. Static membership, placement, fencing, and resource refs remain exact runtime-cohort identities rather than dynamic-transition claims. Deterministic core-owned timer tokens deny stale queued timeouts before durability or network effects. Production admission remains false; this completion establishes port wiring, not quorum, recovery, or release readiness.
- [x] [serial] Implement the first live Raft service profile with protocol registration, elections, replication, commit, reads, snapshots, recovery, and bounded static membership before admitting wider transitions. r[molten.fabric_consistency.live_raft]
  - Evidence: a pure static three-voter transition core binds every protocol envelope to group, generation, configuration, fencing, and member identity; persists hard state and flushed log mutations before dependent sends; elects and commits by current-term majority; suppresses duplicate requests; and ignores stale responses. Linearizable reads remain pending until a unique current-term read context reaches a majority. Canonical snapshots compact only committed state, retain idempotency, install durably before application restore and acknowledgement, and recover from Redb with a checked committed suffix; gaps, stale epochs, invalid boundaries, and tampered identities deny before effects.
  - Live evidence: concrete Iroh, capability-rooted Redb, admitted Tokio time, production entropy, application, and supervision ports execute election, replication, commit, read-index, snapshot catch-up, and cleanup. A bounded ingress pump decouples frame acceptance from protocol execution so response sends cannot block the next admitted frame; cancellation returns the listener for explicit drain. The distinct-process fixture runs three separate OS process IDs, endpoint identities, and durable roots, deliberately leaves the third voter behind, then proves majority commit/read and durable snapshot catch-up with clean child exits. Protocol traffic has no socket fallback, durability has no ambient filesystem fallback, and protocol timing/randomness remain on admitted ports. This completes the first profile but does not satisfy Phase 4 partition, quorum-loss, crash/restart, stale-leader, aggregate-evidence, or production-admission requirements.
- [x] [parallel] Preserve pure application state machines and prevent engine internals from entering extension state identity or semantics. r[molten.fabric_consistency.group_isolation]
  - Evidence: application handlers receive only extension-neutral request, command, schema, snapshot, and application-state refs. Raft terms, indexes, roles, logs, timers, transports, and durable handles remain inside the engine shell; adapter receipts bind the exact group and application manifest without promoting engine internals into extension state identity.

## Phase 4: Distributed evidence and operations

- [ ] [serial] Add multi-process fixtures for quorum formation, commit, quorum-backed read, partition, quorum loss, crash/restart, snapshot catch-up, and stale-leader fencing using distinct endpoints and durable namespaces. r[molten.fabric_consistency.production_admission]
  - Progress: the three-process fixture proves distinct process/endpoint/root quorum formation, commit, majority read-index, deliberate follower lag, durable snapshot catch-up, and bounded ingress cancellation. It then partitions both followers at the protocol-delivery boundary and verifies that a flushed second proposal remains uncommitted and a second linearizable read remains pending without majority evidence. After canonical active-state checkpoints, the parent kills all three processes without service cleanup and starts three new process IDs over the same explicit durable roots. Recovery restores committed application state, clears transient reads, retains the uncommitted request as uncommitted, and exits cleanly. An installed machine-loss snapshot itself establishes its atomic recovery commit boundary, avoiding a separate-marker crash window. Stale-leader fencing remains open.
- [ ] [parallel] Add bounded group admission, configuration, selected commit, read-currentness, snapshot, recovery, failure, and aggregate health evidence without per-heartbeat receipts. r[molten.fabric_consistency.evidence_granularity]
- [ ] [parallel] Add operator readback and bounded create, inspect, drain, snapshot, recover, and remove workflows with dry-run preflights. r[molten.fabric_consistency.operator_readback]

## Phase 5: Validation

- [ ] [serial] Run shared engine conformance, deterministic simulation, multi-process live, failure, recovery, fencing, extension-isolation, policy, resource, non-claim, and model-profile denial tests. r[molten.fabric_consistency.final_validation]
- [ ] [serial] Run Cairn validation and proposal, design, and tasks gates before sync and archive. r[molten.fabric_consistency.final_validation]

## Dependency status

`fabric-cross-process-transport-shell` is archived and the extension-facing pure
consistency port is complete. Phases 3–5 are now unblocked. Production admission
remains denied until the live service, distinct-replica quorum/recovery evidence,
and operator workflows pass; the archived transport evidence proves connectivity
and bounded cleanup only, not consensus correctness or durability.
