## Context

Molten can restore logical tasks from canonical runtime state. ChaosControl can capture complete CPU, memory, and device state under an exact KVM profile. VM Cohort plans bounded copy-on-write workers from retained initialized checkpoints.

These mechanisms are complementary. They are not interchangeable representations of one semantic heap.

## Decisions

### Decision: Use closed logical and opaque profiles

**Choice:** A logical profile references Molten-owned durable state, tasks, history, scheduler, virtual time, entropy commitments, effect state, runtime profile, and policy.

An opaque profile references one exact machine snapshot descriptor and its cohort-bound closure. Unknown profile classes deny.

**Rationale:** Profile type determines completeness, restore, replay, retention, and merge rules.

### Decision: Bind complete compatibility cohorts

**Choice:** Every profile binds exact behavior-relevant cohort identities. Opaque cohorts include architecture, KVM state profile, CPU feature inventory, vCPU topology, device identities, memory and disk format, runtime build, and backend state.

Logical cohorts include runtime ABI, schema set, handler set, task model, scheduler, time, entropy, and effect-log profiles.

**Rationale:** Matching bytes without matching execution semantics cannot prove restorable state.

### Decision: Require explicit completeness inventories

**Choice:** Each snapshot descriptor carries a closed required-component inventory and observed component identities. Missing, duplicate, unsupported, stale, or incompatible components block complete status.

No decoder fills missing state with defaults.

**Rationale:** Silent defaults can turn an incomplete snapshot into a false replay claim.

### Decision: Preserve logical and opaque restore boundaries

**Choice:** Logical restore uses pure ordering over Molten roots and application-owned adapters. Opaque restore delegates snapshot validation and machine reconstruction to the admitted ChaosControl profile.

A mixed world may reference both only with explicit ownership and synchronization facts. It cannot claim one representation validates the other.

**Rationale:** Each runtime owns its own state meaning and recovery mechanics.

### Decision: Recreate host handles and current authority

**Choice:** File descriptors, timers, sockets, credentials, keys, transport sessions, and other host handles stay outside snapshot bytes.

Restore obtains new handles through admitted ports and rechecks current policy, capability, revocation, resource, and adapter facts.

**Rationale:** Host handles are ambient capabilities, not portable deterministic state.

### Decision: Use VM Cohort only for copy-on-write clone mechanics

**Choice:** After VM Cohort implementation and the ChaosControl pilot pass, Molten can request bounded clone plans from one exact retained checkpoint.

Each child binds parent identity and isolated memory, device, disk, and endpoint overlays. Molten retains workload, authority, scheduling, retention, and release decisions.

**Rationale:** Clone mechanics are reusable. World and workload meaning remain product-owned.

### Decision: Opaque snapshots never enter semantic merge

**Choice:** World diff can report exact identity and cohort differences for opaque snapshots. World merge rejects divergent opaque roots.

Operators can select one branch, rerun from an ancestor, or use application-level reconciliation after restore.

**Rationale:** Byte or page merge cannot preserve arbitrary CPU, device, task, or effect semantics.

### Decision: Snapshot receipts remain bounded evidence

**Choice:** Receipts report supplied profile, completeness, compatibility, capture, clone, restore, and replay observations.

They do not prove guest correctness, sandboxing, current authority, cross-host portability, future replay, or release eligibility.

**Rationale:** Snapshot fidelity is narrower than whole-system correctness.

## Rollout

1. Define logical profile validation and restore planning.
2. Add an observation-only ChaosControl descriptor adapter.
3. Restore one exact local opaque fixture under the same cohort.
4. Add VM Cohort copy-on-write cloning after its repository gate and consumer pilot pass.
5. Add mixed-profile support only after explicit synchronization fixtures exist.

## Risks / Trade-offs

- Exact opaque compatibility limits portability. Explicit denial is safer than silent degraded restore.
- Opaque snapshots can be large. Content-addressed overlays and retention policy control storage growth.
- Mixed profiles can double-count state. Ownership and synchronization facts must be explicit.
- VM Cohort remains an optional mechanism. Clone realization requires its exact published revision, complete observations, certain cleanup, and Molten-owned admission.
