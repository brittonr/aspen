# Revocation and Stale Authority

Current authority depends on more than whether a reference still parses or a historical receipt still hashes correctly. Expiry, epoch bounds, key rotation, and supplied revocation facts can change the answer for an otherwise identical request. This article describes the inspected currentness and cleanup helpers, not a distributed revocation service. It assumes the [architecture's](../../architecture.md) distinction between canonical evidence and authority, and the [nominal reference non-claims](../../nominal-authority-references.md). See the [Technical companion](../README.md) for broader lifecycle discussions.

## Currentness is evaluated over explicit facts

`AuthorityGrantCurrentnessInput` contains a context; requested principal, capability, operation, and scope; logical time; grant, minimum, and current epochs; current key references; and revocations. `authority_grant_currentness` first admits selected references into nominal domains. It then gathers independent diagnostics rather than allowing one passing check to override another ([currentness implementation](../../../src/authority/parts/mod/p001/body.rs)).

The principal must equal the context subject, and some capability must match the requested action and scope. A grant epoch below the minimum is stale; one above the current epoch is not yet current. Logical time below `not_before` is too early, while time at or above `expires_at` is expired. If context keys are nonempty, at least one must occur in the supplied current-key set. Finally, any applicable supplied revocation denies the request. Empty diagnostics produce `pass`; otherwise the decision is `fail`.

These rules are deterministic comparisons over input facts. They do not read a wall clock, discover rotated keys, or query a revocation service. The pure law and the shell's responsibility to supply appropriate facts remain distinct. An authentic but old current-key list can still be the wrong input to a present-day authority decision.

The convenience `admit_authority` wrapper chooses some facts itself: requested principal is the context subject, operation is the requested capability, minimum epoch is zero, current epoch is logical time, and current keys are the context's own keys. Those choices mean this wrapper does not independently demonstrate caller identity or external key rotation. Review of a security-sensitive use must examine the caller and the selected entry point, not assume the richer input model is always exercised.

## What the revocation matcher actually sees

`revocation_hits_context` first requires `effective_at <= logical_time`. It then compares the target reference with the context reference, subject reference, delegation references, key references, and canonical subject-bound capability references. The matcher does not branch on `target_kind`, although the constructor recognizes several kinds ([matching implementation](../../../src/authority/parts/mod/p004/body.rs)). This is narrower than a universal interpretation of every supported revocation target.

Token admission has a separate rule set. Its diagnostics reject a revoked issuer or any token delegation present in the proofset's revocation references. Token expiry is checked with `request.at_tick > token.expires_at_tick`, whereas canonical-context expiry uses `logical_time >= expires_at`. Therefore equality at the expiry boundary differs between these helpers ([token diagnostics](../../../src/capability/parts/tokens/p001/body.rs)). This article does not resolve that difference by inventing one shared endpoint convention; callers and reviewers must preserve the actual helper semantics.

## Cleanup is not admission, and replay is not renewal

`cleanup_for_revocation` parses a revocation and filters a supplied assertion collection. It removes recognized two-field `authority-bound-assertion` values whose recorded authority equals the revocation target. Other values, including ones whose authority field cannot be parsed, remain. It emits a cleanup receipt reporting a removed count ([cleanup implementation](../../../src/authority/parts/mod/p001/body.rs)).

Unlike the currentness matcher, this cleanup helper does not test `effective_at` before filtering. Its `logical_time` argument is recorded in the receipt, not used to defer removal. Consequently it should be understood as a direct cleanup operation for a supplied target, not as a scheduler that decides when revocation becomes effective. Broader automatic cleanup described in the [architecture](../../architecture.md) is an intended runtime ownership rule; this finite helper alone is not proof of its complete distributed implementation.

Historical receipt checks are similarly limited. `replay_verify_receipt` parses the context and compares the receipt's recorded context reference. It does not rerun present-time admission. The nominal core's `historical_replay_is_evidence_only` checks the explicit current-authority flag; it does not establish current authority from the existence of historical evidence ([nominal core](../../../crates/molten-core/src/nominal.rs)).

## Worked stale-key and expiry scenario

Suppose an illustrative context expires at logical time 40, carries key K1, and permits the requested operation. At time 39, a caller supplies only current key K2 after rotation. The context can fail with `key-not-current` before expiry. Supplying K1 instead would change that local comparison; it would not prove K1 genuinely remains current outside the supplied model.

At time 40, the context expires even if K1 is supplied and no revocation matches. An otherwise equivalent token checked by the token helper is not expired solely by equality at tick 40; its expiry denial begins above that tick. Now supply a future-effective revocation to the cleanup helper at time 39: the helper can remove matching assertions immediately because it does not perform the currentness matcher's timing test. These examples expose distinct operations rather than proposing an application sequence.

## Verification and review

Suggested cases exercise just-before, equal-to, and just-after expiry; epochs on both sides of the accepted interval; disjoint current-key sets; and revocation targets at their effective boundary. Check cleanup independently with ordinary assertions, recognized bound assertions, and future-effective records. Review how shells obtain and bind currentness facts, and whether delayed work reenters admission instead of replaying old permission. These are proposed checks, not reported test runs.

## Limits and non-claims

No inspected helper proves revocation delivery to every node, instantaneous cancellation of already-issued effects, or distributed key agreement. A cleanup receipt reports the supplied collection's transformation, not global withdrawal. Operational filesystem capability containment remains a separate mechanism and does not automatically disappear because a canonical authority record is revoked ([filesystem authority](../../local-filesystem-authority.md)). The timing and cleanup differences above remain explicitly scoped implementation observations.

## Sources

- [Architecture](../../architecture.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Filesystem authority boundary](../../local-filesystem-authority.md)
- [Currentness, cleanup, and replay helpers](../../../src/authority/parts/mod/p001/body.rs)
- [Revocation target matching](../../../src/authority/parts/mod/p004/body.rs)
- [Token expiry and revocation diagnostics](../../../src/capability/parts/tokens/p001/body.rs)
- [Nominal historical evidence model](../../../crates/molten-core/src/nominal.rs)
