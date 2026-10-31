## Context

A signed head claim can remain valid after it becomes stale. Local durable generations reject that claim only when the observer retains newer local state.

External witnessing gives another currentness observation. It does not grant branch authority, prove world correctness, or make a local and external mutation atomic.

## Decisions

### Decision: Offer explicit local and witnessed assurance profiles

**Choice:** Keep a `local-generation-v1` profile for stale-claim rejection under intact local state. Add `independent-witness-v1` for stronger rollback resistance.

Every receipt names its selected profile. The local profile cannot satisfy a policy that requires independent witnessing.

**Rationale:** The system must not silently promote local currentness into whole-store rollback protection.

### Decision: Consume the shared witness contract

**Choice:** Molten adapts the provider selected by the governed release-channel work. The adapter normalizes append, inclusion, consistency, checkpoint, quorum, unavailable, and fork observations into product-owned DTOs.

Molten does not define provider wire formats, operate a log, or select global trust roots.

**Rationale:** Release metadata and world heads need the same bounded witness mechanics. A Molten-only protocol would duplicate ownership.

### Decision: Witness before final head publication

**Choice:** The shell uses this staged flow:

1. Validate and durably stage the signed head claim and expected local state.
2. Request witness append for the exact claim identity.
3. Validate inclusion, consistency, checkpoint, provider, and quorum observations.
4. Recheck the local head, generation, policy, authority, and staged claim.
5. Atomically publish the new head, highest admitted witness state, and local transition record.

A witnessed claim whose final local transaction loses compare-and-swap remains an orphan observation. It does not move the branch.

**Rationale:** External append cannot join the local transaction. Witness-first finalization prevents an unwitnessed head from satisfying the strong profile.

### Decision: Preserve explicit uncertainty and reconciliation

**Choice:** Timeouts or disconnects after an append request produce uncertain witness state. Failures after local transaction submission produce uncertain local state.

Reconciliation reads both systems, validates exact operation identities, and returns finalize, already-complete, superseded, conflict, retryable, denied, or manual-review decisions.

**Rationale:** Converting uncertainty to failure can duplicate append or head mutation. Converting it to success can publish an unwitnessed head.

### Decision: Persist highest admitted currentness

**Choice:** The local head transaction stores the provider identity, signed checkpoint identity, consistency predecessor, branch generation, and quorum set admitted for that transition.

Missing or rolled-back witness state blocks the strong profile. Recovery cannot silently reset the highest admitted checkpoint.

**Rationale:** Witness proofs have no anti-rollback value if the consumer forgets the latest admitted state.

### Decision: Keep witness, authentication, and authorization separate

**Choice:** Artifact Auth authenticates the head statement. Basalt, UCAN, and durable authority observations authorize branch mutation. Witness checks establish bounded external currentness only.

**Rationale:** Inclusion in a log does not authorize a signer or prove the world commit is valid.

## Rollout

1. Wait for the workspace ownership decision and pin the reviewed provider contract.
2. Add pure normalized observation and profile validation.
3. Add staging and observation-only append planning.
4. Add final local publication and uncertain reconciliation.
5. Run one provider pilot and one deterministic fake-provider conformance suite.
6. Require the strong profile only after rollback, fork, and unavailable-provider tests pass.

## Risks / Trade-offs

- Witness unavailability can block strong-profile head movement. Policy must make this visible.
- Provider forks require operator action and may halt a branch.
- Witness-first flow can leave harmless orphan claims after local contention.
- One provider can remain a common failure domain. Quorum policy must name independence assumptions.
- A passing witness observation does not prove external storage durability or global consensus.
