# Key Rotation and Generation Fences

Key rotation changes which persisted private key may serve a purpose; it is not merely publication of another public key. Molten represents that transition with a pure plan, shell persistence, and a public outcome. This article assumes the [cryptographic identity contract](../../fabric-cryptographic-identity.md) and purpose-scoped handle model, and carefully distinguishes the general core state space from the narrower current file adapter. The [Technical companion](../README.md) links related lifecycle discussions.

## The transition names both sides

`KeyRotationRequest` identifies profile, purpose, backend class and reference, old handle and public-key references, old and new generations, policy, activation boundary, overlap choice, and optional revocation evidence. In [rotation planning](../../../crates/molten-core/src/fabric_crypto_identity/rotation.rs), the old-side fields are checked against the supplied current handle. A request with the right generation but the wrong public key or backend is not the same transition.

The generation law is strictly increasing: `new_generation` must exceed `old_generation`. The code does not require consecutive generations. Consequently, a reviewer should reason about monotonic advancement rather than invent a hidden “exactly plus one” invariant. Currentness must permit the operation, and a no-overlap request requires a well-formed revocation-evidence reference.

`complete_key_rotation` checks the generated handle against the requested profile, purpose, backend, and new generation. It also requires `Current` and rejects exposed secret material. The resulting `KeyRotationOutcome` records both public identities and generations together with activation, policy, and revocation references. This function compares supplied values; it does not independently observe durable storage or validate the truth of the referenced revocation evidence.

## The file adapter's narrower operation

The current [capability-file implementation](../../../src/fabric_crypto_identity/file/parts/adapter/p002/body.rs) rejects every rotation request whose overlap is not `None`. It then resolves the persisted current key, plans the transition, generates a fresh secret, constructs the new public handle, completes the pure outcome, and writes the replacement record with restricted permissions. A successful return follows the write call; it is not proof of an unrelated distributed activation protocol.

Signing resolves the purpose-specific key again and supplies its generation and handle reference to `plan_sign`. An old cached handle therefore fails against newly resolved state even if it still names a valid Ed25519 public key. The [signing path](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs) separates cryptographic validity from permission to produce new signatures under the current local generation.

Verification has a different information source. `VerificationInput` supplies signer currentness and generation; ordinary `verify` does not resolve a remote signer's authoritative history from the local file namespace. That makes correct currentness evidence an external prerequisite. A retired signature's mathematical validity is neither erased by rotation nor sufficient for current admission.

## Overlap discrepancy and safe scope of interpretation

The [governing document](../../fabric-cryptographic-identity.md) describes overlap as verification-only and bounded by explicit policy. The inspected [core rotation enum](../../../crates/molten-core/src/fabric_crypto_identity/rotation.rs), however, includes both `VerifyOnly` and `SignAndVerify`, mapping either to `KeyCurrentness::Overlap`. The [core model](../../../crates/molten-core/src/fabric_crypto_identity/model.rs) makes `Overlap` satisfy `permits_signing`, and `plan_sign` uses that predicate. The current file adapter avoids both paths by rejecting overlap entirely.

These are not interchangeable descriptions of implemented production behavior. The intended verification-only contract is broader than the file adapter's implemented no-overlap operation, while the generic core representation admits a broader signing state than that prose suggests. This article neither changes the contract nor blesses overlap signing; it documents the discrepancy so readers do not infer a working bounded-overlap service, expiry mechanism, or retention policy from an enum variant.

## Illustrative stale-operator request

Imagine generation 7 is current. Operator A builds a no-overlap request for generation 8, naming the generation-7 handle and public key. Before operator B's later request is evaluated, A's rotation completes. B's request still names generation 7 but asks for generation 9.

The larger target number does not rescue B's request. Once the adapter resolves generation 8, comparison with B's old generation, handle, and public-key references identifies a stale starting point. B needs a newly authorized transition based on actual current state. This is a sequential reasoning example, not a proof that concurrent writes are serialized: the inspected adapter performs resolution and persistence as separate calls and this article makes no cross-process compare-and-swap claim.

Explicit revocation is another operation. `revoke` checks a current handle, writes a canonical marker under the purpose-specific `.revoked` path, and returns revoked status. Subsequent resolution checks marker presence and rejects use. That durable marker differs from a rotation outcome's old-key currentness and revocation-evidence reference; no-overlap rotation does not call `revoke` on the newly occupied purpose path.

## Review and verification guidance

Suggested review traces old identity, new identity, generation, and policy across planner input, generated handle, persisted record, and restart resolution. The existing scenario `rotation_fences_stale_handle_and_restart_resolves_new_generation` is in [shell lifecycle tests](../../../src/fabric_crypto_identity/parts/tests/p000/body.rs). Core transition cases are in [pure tests](../../../crates/molten-core/src/fabric_crypto_identity/tests.rs). These are suggested verification targets, not tests executed for this article.

Include stale signing and stale rotation requests as separate cases. Also distinguish permission failure before key use, durable revocation-marker refusal, and a verification decision denied by supplied currentness. They exercise different boundaries despite similar user-facing “key unavailable” consequences.

## Limits and non-claims

Generation fencing does not itself establish global ordering, exactly-once rotation, clock synchronization, or remote revocation dissemination. An activation-boundary reference is recorded and syntax-checked, not interpreted here as a wall-clock deadline. [Nominal references](../../nominal-authority-references.md) do not supply that missing temporal or authority meaning. Operational receipts can bind observed local state, but neither a transition record nor successful readback grants membership, signing-policy approval, or deployment readiness.

## Sources

- [Cryptographic identity contract](../../fabric-cryptographic-identity.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Core rotation planning and completion](../../../crates/molten-core/src/fabric_crypto_identity/rotation.rs)
- [Core currentness model](../../../crates/molten-core/src/fabric_crypto_identity/model.rs)
- [File rotation and revocation observations](../../../src/fabric_crypto_identity/file/parts/adapter/p002/body.rs)
- [Signing and explicit revocation](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs)
