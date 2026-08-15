## Context

A general fabric cannot assume that every service uses one global cluster membership or one replica-placement policy. At the same time, each extension should not rebuild node inventory, locality, resource accounting, failure observation, assignment fencing, and drain mechanics outside Aspen's capability boundary.

## Decisions

### 1. Membership views are sourced snapshots, not ambient truth

**Choice:** A canonical membership view contains an ordered member set, node descriptor refs, view id or epoch, source profile, freshness, authority evidence, and non-claims. Providers may be static, policy-managed, consistency-backed, or simulation-owned.

**Rationale:** Different deployments have different membership authorities. A gossip observation must not silently become a committed cluster configuration.

### 2. Failure detection is separate from membership mutation

**Choice:** Failure detectors emit bounded observations such as reachable, suspected, unavailable, recovered, or unknown with detector profile, time basis, confidence, and evidence. They do not directly remove members or reassign roles.

**Rationale:** Partitions and delay make failure detection imperfect; policy or an authority service must decide how observations affect membership and placement.

### 3. Placement is a deterministic pure core

**Choice:** The placement core consumes an explicit membership view, role requirements, current assignments, resource inventory, locality and anti-affinity constraints, policy decisions, failure observations, and deterministic tie-break input. It returns a plan or structured unsatisfied constraints without performing I/O.

**Rationale:** Placement must be reproducible, explainable, simulation-friendly, and testable without a live cluster.

### 4. Roles and assignment lifecycle are extension-owned but fabric-hosted

**Choice:** Extensions define canonical role kinds and requirements. The fabric hosts propose, reserve, assign, acknowledge, activate, drain, replace, and release transitions under service-generation and assignment epochs.

**Rationale:** Replica, sequencer, scheduler-worker, shard, and compactor semantics differ, but recruitment mechanics are reusable.

### 5. Fencing strength is profile-scoped

**Choice:** Every assignment carries an epoch and fencing token issued by an admitted authority profile. Consumers validate tokens at effect boundaries. Profiles declare whether fencing is process-local, node-local durable, quorum-ordered, or externally enforced.

**Rationale:** A monotonically increasing number in one process is not distributed fencing.

### 6. Drain is preferred over abrupt reassignment

**Choice:** Planned removal enters drain, stops new placements or requests, transfers or checkpoints extension-owned state, waits within a bounded grace policy, and then releases. Failure replacement follows a separate path and records lost acknowledgements or uncertain ownership.

**Rationale:** Graceful maintenance and failure recovery have different safety and availability trade-offs.

## Functional core / imperative shell split

- Pure core: descriptor/view validation, failure-observation reduction, eligibility, placement planning, deterministic tie-breaks, assignment transitions, epoch and token validation, drain decisions, and evidence payloads.
- Shell: observe peers, fetch authority views, persist assignment state, invoke policy, query resources, start or stop extension roles, transfer state through admitted ports, and publish evidence.

## Risks / Trade-offs

- A generic placement DSL can become too complex. Start with typed hard constraints and explicit scored preferences; deny unknown policies.
- Failure suspicion can cause churn. Require freshness, stabilization policy, and authority decisions before assignment changes.
- Weak fencing profiles can be overclaimed. Surface their exact enforcement domain in every assignment and receipt.
