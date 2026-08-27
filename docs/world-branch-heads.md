# World branch heads

Molten owns mutable names for immutable world commits. A branch head does not change either commit identity.

## Dependency cohort

Molten uses these reviewed sources:

- Choregraph `choregraph-history` from `rad://zL2ncTUeASVYwcoGkEXv9JKgGbAF` at `b3e08e19750f53bdbcae970cdf58a47a791ed20b`. Cargo uses its read-only HTTPS seed adapter.
- Artifact Auth from `ssh://git@github.com/OnixResearch/onix-artifact.git` at `c932138d880ddf4c2967f4c024b489b5c0022bf1`.

Choregraph supplies immutable event graphs and pure generation-fenced branch plans. It does not own Molten branch policy, storage, or authority.

Artifact Auth verifies exact signer statements under supplied observations. It does not authorize a branch mutation.

## Claim structure

A `molten.world-head-claim.v1` claim binds these values:

- branch identity and branch class
- expected and successor world commits
- expected and successor generations
- create, advance, merge, or recovery purpose
- exact policy identity
- explicit merge source heads

The claim uses canonical packed Preserves. Its BLAKE3 identity is the Artifact Auth statement subject.

Signatures and authentication decisions remain detached. They do not enter either world-commit identity.

## Planning

The pure core validates the claim, bounds, policy, signer observations, authority observation, and currentness facts.

Molten projects each world commit into one Choregraph history event. The world identity remains the event payload identity.

A normal advance requires the expected head as an immediate successor parent. A merge requires every declared source as an immediate parent.

An explicit creation can establish an absent branch. Recovery requires policy admission and an independent currentness observation.

## Generation fencing

Each accepted mutation advances the durable generation by one. Old, repeated, skipped, or contradictory generations fail closed.

This check protects against stale claims while the observed durable store remains intact. It does not detect rollback of both head and generation state.

A stronger rollback claim needs an independent witness. The local protocol does not supply that witness.

## Authentication and authority

The shell verifies Ed25519 signatures through Artifact Auth. It then evaluates current Molten signer roles and mutation authority.

A valid signature does not grant branch authority. A valid claim remains inert when current policy or authority denies the mutation.

The standalone `world-head advance` command fails closed. It stays disabled until a current authority adapter is composed.

## Atomic local storage

The local adapter stores heads and transition receipts in one Redb transaction under the capability-rooted storage namespace.

The adapter reads the current head inside the transaction. It then reruns authentication and authority observations before insertion.

The transaction records the successor state and receipt together. A stale observation leaves the prior head unchanged.

An uncertain commit result enters reconciliation. It does not produce a confirmed success claim.

## Conflicts

Competing valid claims for one expected head and generation form a bounded conflict set.

Molten sorts conflict members for stable identity. It does not select by time, arrival order, lexical order, or last writer.

An operator must select a policy action or create an explicit merge. Conflict storage does not grant that authority.

## Operator commands

`molten world-head` provides these commands:

- `plan` writes one canonical detached claim.
- `sign` uses the capability-rooted authority key adapter.
- `inspect` reads one local branch state.
- `advance` validates inputs and fails closed without a current authority adapter.
- `conflicts` lists bounded stored conflict records.
- `reconcile` reports manual review without automatic head selection.

Planning and signing do not mutate a branch. Signing does not authorize the signed transition.

## Non-claims

World-head evidence does not prove these properties:

- whole-store rollback detection
- distributed consensus or remote convergence
- remote publication or availability
- application merge correctness
- effect release
- world-commit semantic correctness
- release eligibility
