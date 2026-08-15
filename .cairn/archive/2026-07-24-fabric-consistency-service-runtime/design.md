## Context

The accepted consensus spec already separates algorithms, registers engine profiles, normalizes receipts, and keeps application state portable. The remaining architectural gap is execution: the default descriptor currently names an in-process control registry, and tests cannot establish that separate node processes exchange protocol messages, lose quorum, recover durable logs, fence old leaders, or serve quorum-backed reads.

The fabric changes provide the ports needed to close that gap. This change uses them without making consensus universal.

## Decisions

### 1. Consistency is an optional extension-facing port

**Choice:** A system extension may create or attach to an admitted consistency group and submit canonical commands, declared reads, snapshot requests, recovery operations, and supported configuration transitions. Each group names an application state-machine manifest owned by the extension.

**Rationale:** Databases and schedulers often need several narrowly scoped agreement groups, while logs or actors may use different mechanisms. One global consensus path would be both a bottleneck and a semantic overclaim.

### 2. A live engine is a supervised service over fabric ports

**Choice:** Engine instances use canonical transport sessions for peer protocol traffic, durable log and snapshot ports for persistence, time and entropy ports for elections and deadlines, membership and placement refs for replicas, fencing for active epochs, and resource/supervision ports for lifecycle.

**Rationale:** This makes dependencies explicit, capability-scoped, and substitutable in deterministic simulation.

### 3. Application semantics remain outside the engine

**Choice:** The engine orders and commits canonical command envelopes. A pure extension-owned state machine applies committed commands. Transaction isolation, log offsets, scheduler policy, and other domain semantics do not enter Raft or the consistency port.

**Rationale:** Consensus ordering is a mechanism, not a database or service implementation.

### 4. Production admission requires observed distributed behavior

**Choice:** A production profile must demonstrate distinct processes, distinct durable namespaces, admitted live transport, quorum formation and loss, election or leadership behavior, committed writes, quorum-backed reads, crash recovery, snapshot catch-up, stale-epoch fencing, and bounded operator workflows under its declared failure model.

**Rationale:** In-process transition correctness and fabricated receipts cannot substantiate a live distributed-service claim.

### 5. The current in-process engine becomes model-only

**Choice:** Reclassify `in-process-raft-control-registry-v1` as deterministic model or simulation evidence. It remains useful for pure state-machine, receipt, and negative-policy tests but cannot satisfy production engine selection.

**Rationale:** Relabeling preserves useful fixtures while correcting the claim boundary immediately.

### 6. Evidence follows protocol boundaries

**Choice:** Emit canonical evidence for group admission, configuration epochs, committed application boundaries selected by policy, quorum-backed reads, snapshots, recovery, material failures, and aggregate health. Raft heartbeats, every append message, and every local log read do not require standalone heavyweight receipts.

**Rationale:** Consensus traffic is latency-sensitive and high-volume; evidence must remain bounded.

## Functional core / imperative shell split

- Pure core: engine descriptors, group admission, Raft transitions, deterministic application state machines, quorum and read-currentness validation, configuration transitions, fencing, recovery planning, receipt payloads, and profile admission decisions.
- Shell: open durable ports, bind protocols, send messages, arm timers, execute effects, supervise replicas, persist snapshots/evidence, and expose operator actions.

## Dependencies

- Distributed-system fabric boundary and system-extension runtime.
- Fabric transport, durable-state, time/scheduling, and membership/placement ports.
- Receipt-first cluster harness for multi-process run-directory and offline evidence conventions.

## Risks / Trade-offs

- A first live Raft service is substantial. Keep the initial group API narrow and deny unsupported membership or read modes.
- Evidence checks can accidentally become part of every heartbeat. Pre-admit sessions/resources and aggregate protocol telemetry.
- A passing small-cluster fixture can be overread as broad production readiness. Bind production admission to exact environment, failure model, scale envelope, and evidence refs.

## Implementation status and dependency resolution

The unsafe pre-existing claim has been corrected: `in-process-raft-control-registry-v1`
is model-only, production construction denies it, and model construction is explicit.
Unknown profiles still deny without fallback, and existing non-claim fixtures remain
in force.

`fabric-cross-process-transport-shell` is archived with a reusable capability-scoped
Iroh listener/client shell, parent-observed distinct-process evidence, bounded reads,
cancellation, cleanup, and no ambient socket fallback. The extension-facing pure port
now binds every operation to the exact group, owner, service generation, application
manifest, engine profiles, membership/configuration, placement, fencing, resources,
policy, and authority inputs. It admits only declared read/configuration modes,
normalizes opaque outcomes, and applies lifecycle/configuration changes only after a
matching successful outcome.

This resolves the transport dependency and unblocks the live-service phases. It does
not itself provide a production consensus engine: live Raft still requires admitted
durable log/snapshot effects, timers, membership and placement, distinct replica
processes, quorum behavior, crash recovery, and the declared distributed evidence
matrix.
