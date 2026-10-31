# World branch heads

Molten owns mutable names for immutable world commits. A branch head does not change either commit identity.

## Dependency cohort

Molten uses these reviewed sources:

- Choregraph `choregraph-history` from `rad://zL2ncTUeASVYwcoGkEXv9JKgGbAF` at `b3e08e19750f53bdbcae970cdf58a47a791ed20b`. Cargo uses its read-only HTTPS seed adapter.
- Artifact Auth from `ssh://git@github.com/OnixResearch/onix-artifact.git` at `5f3962d7471d1547e044f57daeda72f56621fd52`.

Choregraph supplies immutable event graphs and pure generation-fenced branch plans. It does not own Molten branch policy, storage, or authority.

Artifact Auth verifies exact signer statements under supplied observations. It does not authorize a branch mutation.

The independent witness provider is the canonical `OnixResearch/release-channel`
repository, selected by the user on 2026-10-29 as the dedicated governed owner of
the shared witness/high-water contract. Molten pins the published immutable
revision `fa389e19d5c9bd84cd75df914cc5418c70d92bfb` of `release-channel-index`
and `release-channel-witness` (MIT) in `Cargo.toml`, `Cargo.lock`, `flake.nix`
and `crate-hashes.json`. The compiled reviewed contract is
`world_witness::provider::contract` (`release-channel-index-v1`, `MIT`, that
revision); an operator pin naming any other revision is rejected. A later owner
revision supersedes this pin only through explicit integration review. Molten
does not consume `release-channel-transports`, so that crate's Bounded HTTP pin is
outside this consumer contract. Molten consumes normalized observations; it does
not own the provider wire format, operate its log, choose global trust roots, or
claim provider honesty, administrative or physical independence.

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

A stronger rollback claim requires the explicit `independent-witness-v1` profile.
The `local-generation-v1` profile remains scoped to intact local durable state and
cannot satisfy a policy requiring independent witnessing.

## Independent currentness and finalization

Durably stage the exact authenticated claim before contacting the provider. Check
the operation, provider contract, branch, claim identity, generation, signed
checkpoint, inclusion, consistency predecessor, quorum, and authenticated current
head observation against the consumer's retained highest state. Inclusion alone
is not an authoritative current-head read.

The provider reservation precedes the local transaction. The transaction rechecks
head, generation, policy, authority, staged claim, and retained witness state, then
publishes the local head and highest witness state together. A local commit while
the provider is reserved is recorded as `reserved`, not externally committed.
Only the confirmed external commit permits the final `admitted` currentness role.
An orphan witnessed claim or lost local compare-and-swap does not move the head.

The durable highest-state codec uses canonical packed Preserves and retains the
provider and checkpoint identities, checkpoint revision, signature observation,
observation time, branch, claim, generation, consistency predecessor, and sorted
quorum members. Parsing does not rerun signature verification or grant admission.
Missing or rolled-back retained state blocks strong admission; recovery must not
silently reset it. Provider forks halt admission for explicit operator handling;
provider unavailability may block strong-profile movement rather than downgrade it.

An uncertain append, external commit, or local commit remains uncertain.
Reconciliation reads the exact operation in both systems and classifies finalize,
already-complete, superseded, conflict, retryable, denied, or manual-review outcomes.

## Orphans and external ordinals

The external index ordinal is the provider namespace position, not the Molten head
generation. The head generation stays inside the canonical claim digest and still
advances by exactly one per accepted local mutation. `provider::ordinal::plan`
derives the external claim from a fresh nonce-bound signed query: it recovers an
exact pending reservation unchanged, otherwise reserves the retained external
high-water plus one with the retained committed claim as predecessor. A durable
reservation records the ordinal actually used, so lost-response recovery and later
current reads match the provider-signed claim rather than recomputing it.

An orphan keeps its reserved ordinal as the provider high-water. While it is
unresolved, every strong advance and every strong current read fails closed. Only
an explicit operator `resolve-orphan` lets the next successor proceed, and then
only above the abandoned high-water and from the exact committed predecessor; this
authenticated resolved-orphan high-water is the only admitted ordinal gap. The
abandoned claim digest can never be reserved again. A planner that ignored the gap
would ask for an ordinal the provider must reject; a planner that accepted another
gap would admit an unproven skip. Neither happens: forged gaps fail snapshot
validation and a stale (rolled-back) predecessor fails the committed-claim check.
Resolving an orphan grants no branch authority and does not repair forks.

## Detached witness receipts

Receipts separately name statement authentication, branch authorization, witness
currentness, local persistence, and reconciliation. They bind the actual claim,
authentication-decision, and authority-observation identities; those roles cannot
be populated with one synthetic shared reference. Receipt identity is BLAKE3 over
canonical packed Preserves, never serde serialization or debug output.

Receipt validation rejects witness-as-authority claims, local-profile independent
claims, mismatched claim/highest-state bindings, and committed local mutations
without independent authentication and authorization admission. A reserved provider
state remains distinguishable from confirmed external currentness even after local
persistence. Receipts carry explicit non-claims: no branch authority from witnessing,
world semantic truth, provider honesty, global consensus, release eligibility,
external effect completion, or future mutation authority.

## Authentication and authority

The shell verifies Ed25519 signatures through Artifact Auth. It then evaluates current Molten signer roles and mutation authority.

A valid signature does not grant branch authority. A valid claim remains inert when current policy or authority denies the mutation.

The standalone `world-head advance` command fails closed. It stays disabled until a current authority adapter is composed.

## Atomic local storage

The local adapter stores heads and transition receipts in one Redb transaction under the capability-rooted storage namespace.

The adapter reads the current head inside the transaction. It then reruns authentication and authority observations before insertion.

The transaction records the successor state and receipt together. A stale observation leaves the prior head unchanged.

An uncertain commit result enters reconciliation. It does not produce a confirmed success claim.

The operator trust root independently installs the canonical witness-policy
identity, including the exact provider key/source/license contract, required
profile, allowed quorum, quorum threshold, and observation age. Initial admission
and the final transaction re-read that installed identity. A mutation credential
or a provider response cannot install a weaker policy. A durable strongest-profile
marker fences the legacy local path even if its highest-state row disappears.

`world_witness::recovery::recover` reads the exact durable stage, actual
Transactional Reconciliation Core attempt/publication binding, local operation
receipt, and fresh authenticated provider state. `reconcile_and_resume` can finish
only that observed reservation; it never creates a new Reserve after a lost
response. Local-after/external-reserved remains retryable, not already complete.
Completed recovery uses the actual persisted authentication/authorization receipt
to recognize the old transaction, not a fabricated before-state or a new grant.
Forks, missing strongest state, and unresolved conflicting operations remain
denied or manual-review outcomes; there is no automatic reset or fork repair.

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

## Private same-host witness consumer pilot

`cargo run --bin world-witness-consumer-pilot -- run NEW_ROOT ACTUAL_PROVIDER_REV
[--provider-program PROVIDER]` provisions a private same-host fixture and runs each
consumer operation in a new process. Supply the actual reviewed full provider
source revision; the pilot does not invent a pin. `NEW_ROOT` must not exist. The
fixture retains its artifacts under that explicitly supplied root for inspection.

Provisioning creates actual Artifact Auth signatures, issuer-signed UCAN grants,
holder-signed claim invocations, and a current installed operator policy. Admission
uses the actual Basalt/UCAN adapter and separately durable replay and revocation
stores. With `--provider-program`, the pilot uses a separately built release-channel
`provider` executable (`provider init`, then `provider serve INDEX CONFIG` for every
request, including the admin-signed writer enrollment); the caller is responsible
for building it from the pinned revision. Without it, the pilot links the same
pinned `release-channel-index` library into separate `index-initialize` and
`index-serve` children. Either provider child opens the retained external index,
verifies the writer signature, and signs its exact reply. Fixed private fixture
keys are deliberately nonproduction; successful fixture execution never
establishes production trust or release eligibility.

The pilot exercises authenticated planning, durable staging, external reservation,
the fenced local transaction, external commit, and repeated independent current
reads. Each step must report its exact expected outcome; an exit status alone never
satisfies a step. Negative steps are part of the run: provisioning with another
real release-channel revision is rejected before any state is created; an abandoned
competing generation-three reservation that is orphaned blocks both the strong
advance and the strong current read; after explicit `resolve-orphan`, generation
three commits at external ordinal four, above the abandoned high-water; and with the
provider index unavailable the current read fails as `Unavailable` rather than
downgrading. Finally it restores the entire consumer root to its generation-one
snapshot, while the independent index and operator authority stores remain at
their newer state. A new consumer process must reject the old local head using the
real public currentness-read path and report a freshly authenticated, strictly
newer external claim. An arbitrary child error does not satisfy the rollback outcome.

The pilot head-policy identity is BLAKE3 over its exact canonical packed Preserves
policy projection, excluding only the self-reference. The independent operator
trust separately binds its installed Basalt policy and canonical witness policy.
Commit references identify materialized canonical opaque fixture payloads with
explicit predecessor links, not fabricated world-capture identities. The exact
policy, opaque commit, and crypto configuration payloads are retained under the
consumer root. These opaque payloads are not captured WorldCommit objects and do
not demonstrate world semantic validity, closure, restorability, or admission.

This is a same-host process-isolation fixture, not independent-host durability,
provider-honesty evidence, global consensus, or an executed result merely because
the source exists. Report the command's actual outcome separately.

## Non-claims

World-head evidence does not prove these properties:

- whole-store rollback detection from the local-generation profile alone
- distributed consensus or remote convergence
- remote publication or availability
- application merge correctness
- effect release
- world-commit semantic correctness
- release eligibility
