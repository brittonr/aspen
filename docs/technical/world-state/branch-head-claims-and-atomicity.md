# Branch-Head Claims and Atomicity

A world commit is immutable; a branch is a mutable name selecting such a commit. This distinction lets Molten record history without pretending that content identity authorizes mutation. This article examines the claim, admission, and local transaction boundaries. It assumes the [branch-head contract](../../world-branch-heads.md) and [world-commit model](../../world-commit.md); see also the [Technical companion](../README.md).

## The transition, not merely its destination, is signed

A `molten.world-head-claim.v1` claim binds branch identity and class, expected and successor commits, expected and successor generations, purpose, policy identity, and explicit merge sources. Canonical packed Preserves determines the claim identity used as the Artifact Auth statement subject. Detached signatures do not alter either world commit.

This binding prevents a reviewer from interpreting a signature over one destination as blanket permission to reach it from any branch or generation. Purpose matters too: creation establishes absence, ordinary advancement follows the expected head, merge preserves declared source ancestry, and recovery is separately admitted. Choregraph supplies structural history and generation-fenced plans, not Molten's signer roles or mutation authority ([governing ownership boundary](../../world-branch-heads.md#dependency-cohort)).

The [admission implementation](../../../crates/molten-core/src/world_head/admission.rs) checks policy, generation, authentication, authority/currentness, ancestry, and bounds as distinct stages. Authentication observations contain more than a cryptographic pass bit: duplicate signer identities, unrecognized roles, stale or revoked signers, and insufficient admitted signer counts are rejected. A threshold of valid signatures is therefore not a substitute for current role and authority checks.

## Generations describe a durable transition sequence

For an admitted claim, the successor generation equals the expected generation plus one, with overflow checked explicitly. Creation uses expected generation zero and no expected head. Non-creation compares both expected head and expected generation against the supplied current state.

Generation and content identity solve different problems. A commit may legitimately recur in history while its branch generation increases. Conversely, a new commit identity does not establish that an expected predecessor is still current. Comparing both fields prevents a stale proposal from becoming fresh merely because the destination object exists.

Recovery deserves a narrow reading. The governing document says recovery requires independent currentness. The inspected core's `validate_authority_currentness` checks for an independent reference when the policy's `require_independent_recovery_currentness` flag is set. This article does not resolve that difference by claiming the core unconditionally enforces the stronger wording. Review the admitted policy and composition before relying on that property.

## Where local atomicity begins and ends

[LocalWorldHeadStore](../../../src/world_head/store.rs) places heads and transition receipts in the same Redb write transaction. It reads the current head inside that transaction and compares the complete observed state with the plan's predecessor. Only then does it call the fresh-admission closure.

Fresh admission checks authentication, authority, exact policy identity, and observed generation. The adapter inserts the successor state and canonical transition receipt before committing. Thus the local atomic unit is the head-plus-receipt transition, not the earlier claim construction, signature collection, object publication, or external authority service.

The implementation distinguishes `AlreadyApplied`, `Stale`, `Applied`, and `Uncertain`. In particular, observing the exact successor returns `AlreadyApplied` without performing another insertion. This is recognition of durable state, not a fresh grant to mutate. A commit error yields `Uncertain`; it is not evidence that the transaction certainly failed, and it is not confirmed success either.

## Worked race: two admissible proposals

Suppose, illustratively, branch `analysis` currently selects commit A at generation 7. Two independently valid claims propose A-to-B and A-to-C, each targeting generation 8. Both successor commits have A as an immediate parent, and both claims pass the initial policy review.

If the B transition commits first, the C transaction subsequently reads B at generation 8. Its predecessor comparison fails, so it returns stale without overwriting B. Signature validity has not changed; currentness has. The local transaction prevents both proposals from being accepted against the same unchanged predecessor state.

This should not be described as a semantic winner-selection algorithm. The [conflict contract](../../world-branch-heads.md#conflicts) retains bounded competing claims with stable conflict identity and does not choose by timestamp, arrival order, lexical order, or last writer. Resolving their meaning requires an explicit policy action or merge. Likewise, an uncertain B commit needs reconciliation rather than immediately trying C as a convenient substitute.

## Review and verification guidance

A useful review follows one claim through canonicalization, detached authentication, pure admission, transactional reread, fresh admission, and receipt insertion. Check the negative boundaries independently: stale predecessor, wrong policy, changed generation, revoked signer, and uncertain transaction outcome.

Suggested integration exercises are two proposals sharing one predecessor and a lost acknowledgement after commit. Inspect both durable state and the transition receipt; a diagnostic “planned” or “signed” line establishes neither. These exercises are proposed, not executed here. The standalone `world-head advance` command remains fail-closed without a composed current-authority adapter, so successful planning is not a mutation smoke test.

## Limits and non-claims

Local atomicity does not imply remote convergence, distributed consensus, or remote publication. Restoring an old database containing both head and generation can evade this local fence; independent rollback evidence is outside the protocol. Branch movement also proves neither application merge correctness nor effect release. Receipts record transitions and observations, not future authority or production readiness.

## Sources

- [World branch heads](../../world-branch-heads.md)
- [World commits](../../world-commit.md)
- [Pure transition admission](../../../crates/molten-core/src/world_head/admission.rs)
- [Transactional local head store](../../../src/world_head/store.rs)
- [Technical companion](../README.md)
