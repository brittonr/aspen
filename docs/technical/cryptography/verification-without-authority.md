# Verification Without Authority

A verified signature establishes a relationship between a public key and an exact statement. It does not establish why that key should be trusted, whether its holder belongs to a federation, or whether a requested effect is permitted. Molten makes this distinction explicit in both its [cryptographic identity contract](../../fabric-cryptographic-identity.md) and [nominal authority reference model](../../nominal-authority-references.md). This article examines where that distinction appears in inspected code. The [Technical companion](../README.md) links the separate authority and evidence topics.

## Separate observation from decision

The shell's `IrohEd25519FileAdapter::verify` parses the supplied public key, checks canonical domain and signature representations, parses the signature, and attempts Ed25519 verification over the expected canonical domain bytes. It derives the actual public-key reference from the parsed key rather than relying solely on carrier metadata. These are concrete [shell observations](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs).

The core receives a `VerificationRequest` containing those observations and the caller's expected domain, signer currentness, signer generation, and policy reference. `evaluate_verification` deterministically checks consistency and returns `Accept` or `Deny` with issues. It does not perform cryptographic I/O or discover a trust root. Crucially, `cryptographic_verification_passed` is an input to this pure function. Calling the core with that field set to true is not evidence that the shell actually verified anything. The [admission implementation](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs) makes the boundary visible rather than hiding it behind a cryptographic-sounding name.

An accepted decision therefore means that the supplied observations and declared context passed this admission procedure. To obtain an operational claim, the consumer must preserve the provenance of those observations, including how currentness was determined. The ordinary verification method takes currentness and generation from `VerificationInput`; it does not query an authoritative remote revocation service.

## Purpose-specific acceptance remains narrow

`admit_federation_verification` and `admit_evidence_verification` consume canonical outcomes. Their shared helper checks the expected purpose and accepted decision kind. The [integration source](../../../src/fabric_crypto_identity/integration.rs) contains no membership enrollment or capability issuance in these helpers. Their names identify a verification gate for a consumer, not an all-encompassing authority decision.

Likewise, the policy reference in the verification request is validated as a reference. That check is not evaluation of the policy's substantive rules. A caller must not turn a content-reference-shaped string into a conclusion that the policy author approved this operation. The nominal reference document makes the parallel point: checked syntax and Rust category separation prevent selected substitutions, but do not prove current authority, freshness, or evidence truth.

The core profile requires explicit non-claims, including no capability authority, no membership, no provenance, no payload correctness, no backend availability, and no algorithm agility. These [model constants](../../../crates/molten-core/src/fabric_crypto_identity/model.rs) are useful review anchors. They describe claims deliberately excluded from the cryptographic boundary, not optional caveats to discard after a successful result.

## Illustrative valid signature, denied effect

Consider a correctly verified federation-origin statement signed by key K under the intended payload schema and verifier context. Suppose K's holder is not admitted by the target federation's membership policy. Cryptographic acceptance can still be internally consistent: the signature really does verify under K. The federation action nevertheless lacks its separate membership prerequisite.

The wrong response would be to broaden `Accept` into “trusted federation participant.” The correct reasoning preserves two propositions: the key verified the statement, and the relevant membership or capability decision remains separate. This is an illustrative boundary scenario, not a claim about a particular uninspected membership implementation.

The same distinction applies to content truth. An evidence-signing key can sign a false assertion, and a verifier can correctly establish that signature. Authenticity of the signer-statement relation does not establish that the stated event occurred or that a lifecycle gate should close.

## Standalone parity does not authorize cutover

The artifact-auth pilot independently reconstructs the standalone statement, checks statement/key/signature carrier identities, and calls `artifact_auth_ed25519::verify_statement`. Its [shell implementation](../../../src/fabric_crypto_identity/artifact_auth.rs) passes that separate observation into the compatibility comparator. It does not reuse the legacy Boolean as proof of a different canonical preimage.

The governing contract retains `legacy_authoritative = true`, `standalone_authority_admitted = false`, and `rollback_available = true`. Accordingly, agreement between legacy and standalone observations is compatibility evidence, not a new authority source. Two failures also need not constitute meaningful parity: unrelated rejection reasons can produce superficially equal decisions while exposing incompatible semantics.

The contract further describes capability-rooted operational receipts and replay against actual persisted key state. Even successful receipt replay remains local operational evidence, not membership, federation, signing-policy, deployment, runtime, or release authority. A receipt's immutability and content binding concern the evidence object; they do not manufacture authorization for the activity it records.

## Review and verification guidance

For each consumer, draw the boundary from external bytes to cryptographic observation, from observation to canonical outcome, and from outcome to the separate policy decision. Identify which component selects the expected key and context and which supplies currentness. A verifier supplied with an attacker-selected key can accurately verify that attacker's signature without establishing the identity the application intended.

Suggested checks include wrong-key verification, stale or revoked currentness, purpose mismatch, tampered carrier identities, and successful standalone verification with authority admission still false. Existing [standalone shell scenarios](../../../src/fabric_crypto_identity/parts/tests/p001/body.rs) and [core scenarios](../../../crates/molten-core/src/fabric_crypto_identity/tests.rs) locate relevant review material. No test execution or production attestation is claimed by this article.

## Limits and non-claims

This explanation does not select trust roots or prescribe a new policy engine. It does not establish transitive provenance, key-holder honesty, global freshness, or whole-system correctness. Canonical verification outcomes are public evidence objects; diagnostic status is an observation surface; neither is interchangeable with an authority grant. The existing governing contracts remain authoritative, including their requirement for a separate reviewed authority-admission change before the standalone pilot can acquire a broader role.

## Sources

- [Cryptographic identity contract and authority non-claims](../../fabric-cryptographic-identity.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Core verification admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs)
- [Required cryptographic non-claims](../../../crates/molten-core/src/fabric_crypto_identity/model.rs)
- [Shell cryptographic observations](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs)
- [Consumer verification gates](../../../src/fabric_crypto_identity/integration.rs)
- [Standalone shell verification](../../../src/fabric_crypto_identity/artifact_auth.rs)
