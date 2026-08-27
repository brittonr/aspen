## Context

Molten already has strong identities for individual runtime concerns. It also has deterministic replay, virtual time, explicit entropy, effect logs, task state, artifact binding, and runtime root inventories.

These mechanisms do not share one causal snapshot identity. Capturing each subsystem independently can produce a set of individually valid roots that never formed one coherent runtime cut.

A world commit must remain a thin composition protocol. It must not absorb storage, authority, execution, evidence, or application merge meaning from existing owners.

## Decisions

### Decision: Molten owns the first world-commit domain

**Choice:** Define `molten-world-commit-v1` inside Molten. Do not create a stack-global crate or generic `RealmCommit` protocol during the first pilot.

**Rationale:** Molten owns runtime composition and can prove a real consumer contract. Choregraph owns history mechanics, while other repositories retain their separate authority and evidence boundaries.

### Decision: Use typed roots instead of one generic digest

**Choice:** The core carries distinct references for artifact, schema, durable state, tasks, history, effect state, scheduler state, time, entropy, runtime profile, and policy.

An authority-observation reference may record prior facts. It never grants current authority. An opaque machine-snapshot reference is optional and has a distinct type.

**Rationale:** A generic digest would permit domain substitution. It would also hide the different restore, replay, retention, and merge rules for each root.

### Decision: Hash only the immutable commit core

**Choice:** Compute the commit identity from a versioned domain tag and canonical Preserves bytes. Parent order and root types are part of the identity.

Keep signatures, attestations, mutable head claims, currentness observations, and operator annotations in detached envelopes over the commit identity.

**Rationale:** Detached envelopes avoid recursive hashing. They also preserve Valence evidence roles and prevent new evidence from changing world identity.

### Decision: Publish a coherent cut through fences and immutable roots

**Choice:** The pure core plans a capture from supplied root observations and revision fences. The shell performs these steps:

1. Observe required roots and their mutable revisions.
2. Persist every missing immutable root object.
3. Recheck every mutable revision and required completeness fact.
4. Reject the capture when any observation drifted or became incomplete.
5. Publish the canonical world-commit object as the final local mutation.

The receipt records the observed revisions, recheck results, root closure, publication outcome, and non-claims.

**Rationale:** Independent stores cannot provide one universal transaction. Immutable roots plus final fenced publication provide a coherent causal cut without claiming cross-service atomicity.

### Decision: Separate validation, restoration, and execution

**Choice:** Closure validation proves only that required typed objects are present and match their declared identities. A pure restore planner orders required adapter actions.

The shell performs materialization and runtime admission. Normal authority, policy, artifact, schema, resource, and effect gates still apply.

**Rationale:** Commit integrity does not prove restorability, compatibility, authorization, or successful execution.

### Decision: Classify snapshot profiles explicitly

**Choice:** A logical profile uses Molten state, tasks, scheduler, time, entropy, and effect roots. An opaque profile may reference a cohort-bound machine snapshot.

A mixed profile must state which subsystem owns each root. It cannot claim semantic merge between logical and opaque state.

**Rationale:** Molten and ChaosControl preserve different state classes. One reference type must not erase that distinction.

### Decision: Keep the core functional and bounded

**Choice:** Canonicalization, validation, capture planning, closure comparison, and restore ordering remain pure. All I/O enters through application-owned ports.

Every collection has a caller-supplied bound. Duplicate parents, roots, or domain-conflicting references fail before shell effects.

**Rationale:** This preserves deterministic behavior, high assertion density, and direct positive and negative testing.

## Rollout

1. Add pure DTOs, canonical encoding, identity, and validation.
2. Add an observation-only capture command that does not publish commits.
3. Add local durable publication with revision rechecks.
4. Restore one logical Molten fixture from a complete commit.
5. Admit an opaque snapshot reference only after cohort and closure checks exist.
6. Enable dependent branch, merge, promotion, and replication changes after the core gates pass.

## Risks / Trade-offs

- A large root set adds bookkeeping. Typed adapters and bounded inventories keep ownership visible.
- Uninstrumented native state can make a capture incomplete. The system must deny a complete-world claim in that case.
- Root objects can become unavailable after capture. Retention and replication remain separate dependent changes.
- A coherent local cut does not prove distributed consensus or external effect atomicity.
- The first schema can expose missing root classes. Version the schema instead of reinterpreting old bytes.
