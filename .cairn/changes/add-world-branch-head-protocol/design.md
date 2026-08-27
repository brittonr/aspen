## Context

World commits are immutable. Operators and runtimes need stable branch names that can move between commit identities.

Choregraph already defines immutable event DAGs, generation-fenced `BranchRef` values, and pure compare-and-swap plans. It deliberately leaves persistence, authentication, and product merge meaning to the host.

Artifact Auth authenticates exact canonical statements under supplied trust and currentness observations. It does not authorize a Molten branch change.

## Decisions

### Decision: Keep head claims detached from world commits

**Choice:** Define `world-head-claim-v1` as a signed statement over branch identity, expected head, successor head, generation, purpose, policy identity, and statement currentness inputs.

The claim is not part of either commit hash.

**Rationale:** Branch heads are mutable. Detached claims permit new signatures, thresholds, and evidence without changing immutable world identities.

### Decision: Reuse Choregraph branch mechanics

**Choice:** Map Molten world commit identities into Choregraph history IDs and use its pure branch compare-and-swap planning.

Molten validates world-specific ancestry, branch classes, policy, authority, persistence, and receipts.

**Rationale:** Reimplementing generation fences would duplicate an existing checked component. Choregraph must not become the Molten head store or authority owner.

### Decision: Require explicit ancestry

**Choice:** A normal advance requires the expected current head among the successor parents. A merge requires all declared source heads among the merge parents.

An import can establish a root only through a separate explicit branch-creation action.

**Rationale:** A valid digest or signature must not permit unrelated history replacement.

### Decision: Treat authentication and authorization separately

**Choice:** Artifact Auth verifies exact statement bytes and supplied signer observations. Molten then checks current branch policy, Basalt and UCAN facts, signer roles, threshold rules, and Durable Authority State observations.

The shell owns private-key access and signing.

**Rationale:** Authentication proves who signed supplied bytes. It does not prove permission to move a branch now.

### Decision: Reject stale claims relative to intact durable state

**Choice:** Each branch has a monotonically advancing generation in the durable head store. An accepted claim must name the exact expected generation and current head.

The shell rejects older, repeated, skipped, or conflicting generations unless an explicit recovery policy admits a fenced repair. Receipts classify this protection as relative to the observed durable store.

The local protocol does not claim detection when an attacker rolls back both the head and its generation state. A later change must require an independent currentness or witness observation for that stronger claim.

**Rationale:** Signatures alone do not prevent replay of an older valid claim. Local generations reject stale claims only while their durable currentness state remains intact.

### Decision: Preserve conflicts instead of selecting by time

**Choice:** When multiple valid successor claims target the same expected generation, retain a bounded conflict set. Block automatic head movement until policy selects or merges them.

Do not use wall-clock order, arrival order, lexical identity, or last-writer-wins behavior as semantic resolution.

**Rationale:** Those orders do not prove application intent or authority.

### Decision: Publish through one local transaction

**Choice:** The pure core returns a planned head transition. The shell rechecks current head, generation, authority, policy, and authenticated statement facts inside the mutation boundary.

The store atomically replaces the head and records the transition operation. Uncertain commit outcomes use explicit reconciliation.

**Rationale:** Planning cannot prove persistence. Rechecks prevent time-of-check drift.

## Rollout

1. Add pure claim and branch validation without mutable storage.
2. Add artifact-auth compatibility fixtures and detached statement codecs.
3. Add a local single-writer head store and transition receipts.
4. Add competing-claim inspection and manual policy selection.
5. Enable remote claim exchange only through the later distribution change.

## Risks / Trade-offs

- Durable generations require recovery policy after storage loss. Recovery must not silently reset currentness state or claim whole-store rollback protection.
- Multi-writer conflicts can block progress. This is safer than implicit semantic selection.
- A passing signature can mislead operators. Every receipt must state authentication and authorization non-claims.
- Local atomicity does not prove remote publication or convergence.
