## Context

The admitted Raft service, ports, ingress pump, application port, durability
adapter, and control/status evidence already exist. The `live_cluster` fixture
assembles them in-process, and `live_process` runs three test children through a
shared signal directory. Both are test-only and use test reference generators.

The conformance adapter now projects real committed applies through the
producer wire contract. ChaosControl needs a production-shaped replica entry
point that can be launched from an exact configuration inside a guest.

## Decisions

### Decision 1: One explicit canonical configuration

**Choice:** `molten cluster replica` reads one bounded canonical configuration
containing the group binding, node id, membership, peer endpoints, durability
root, application manifest/handler, listener key source, address, protocol /
timer / entropy / supervision references, and finite bounds. Every reference
must be a canonical content ref; every path must be absolute or
capability-rooted.

**Rationale:** A guest cohort cannot depend on workspace-relative paths, test
reference generators, or implicit defaults. One reviewed input makes the
cohort handoff exact and diffable.

### Decision 2: Reuse admitted service and port composition

**Choice:** The shell constructs `ScopedLiveReplica` through the existing
admitted port bundle and start plan. It does not add a second consensus engine,
transport, durability store, or application runtime.

**Rationale:** The service is already exercised by the native and live tests.
The shell must own lifecycle and I/O, not protocol or application semantics.

### Decision 3: Explicit listener and key source

**Choice:** The listener is created from an explicitly supplied key file and
bind address. There is no ambient key discovery, inherited secret, wildcard
public bind, or fallback transport. Peer endpoints are exact immutable
descriptors.

**Rationale:** A network identity is capability, not configuration convenience.
Guest packaging needs one exact identity per replica.

### Decision 4: Fresh and recover modes are distinct

**Choice:** `fresh` initializes a new durability root; `recover` reopens an
existing root and runs the existing recovery/reconciliation path. Existing or
missing roots outside the requested mode fail closed. A recovered replica
publishes its recovery ref and terminal receipt.

**Rationale:** Process restart, snapshot catch-up, and stale-generation fencing
must remain observable and distinct from first boot.

### Decision 5: Bounded ingress and event loop

**Choice:** The shell uses the existing `ReplicaIngressPump` with explicit
event capacity, delivery rate/window, accept timeout, and poll interval. The
event loop has a finite iteration/stop contract. No unbounded queue, unbounded
read, or implicit retry is introduced.

**Rationale:** Guest execution must remain bounded under faults and slow peers.

### Decision 6: Readiness and terminal evidence are explicit

**Choice:** Readiness is published only after listener bind, service start, and
application admission. The terminal receipt binds group, node, source/build/
closure refs, endpoints, durability root, recovery ref, clean shutdown, and
observed terminal state. Status readback is a projection, not authority.

**Rationale:** A launched process is not a ready replica, and a clean process
exit is not durable or machine-loss evidence.

### Decision 7: Application binding is closed

**Choice:** The configuration names one supported admitted application handler
and manifest. A collaboration-scope handler is the first supported production
application. Unknown, test-only, or mismatched handlers fail closed.

**Rationale:** The shell must not become a generic test harness or a place to
invent application semantics.

### Decision 8: Guest handoff is a separate claim

**Choice:** The change produces a Nix-buildable shell and exact runtime inputs.
ChaosControl owns guest image assembly, vCPU topology, device mapping, fault
scheduling, and campaign evidence. A successful native run is not guest or KVM
evidence.

**Rationale:** Native process execution cannot inherit guest determinism or
device semantics.

## Risks / Trade-offs

- A production shell expands the attack surface for network identity and
  durability roots. Explicit key files and canonical configs bound it.
- Reusing the existing test service assembly may retain hidden test assumptions.
  The implementation must replace every test-ref/path dependency, and negative
  config tests must prove that.
- Multi-process native tests are slower and host-dependent. Bound them and keep
  guest execution in ChaosControl.
- The shell does not add authentication, policy, or tenancy. Those remain with
  the existing ports and the eventual deployment boundary.
