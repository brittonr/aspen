# Canonical Signature Domains

A valid signature answers a question about an exact preimage. Molten makes that question explicit by signing a canonical domain record instead of accepting arbitrary ambient bytes. This article assumes familiarity with content references and the [cryptographic identity contract](../../fabric-cryptographic-identity.md). It explains the inspected legacy Molten signing path and distinguishes it from the standalone artifact-auth pilot. Return to the [Technical companion](../README.md) for related boundaries.

## The signed question

`SignatureDomain` names a domain identifier and version, purpose, payload schema and reference, signer-public reference, and verifier-context reference. The canonical wrapper binds additional profile context: both the declared profile reference and the canonical profile admission reference. In the [canonical constructor](../../../src/fabric_crypto_identity/parts/canonical/p000/body.rs), these fields become a `fabric-crypto-signature-domain-v1` Preserves record, from which both canonical bytes and `domain_ref` are derived.

That distinction matters. The payload reference identifies the payload selected by the caller; the domain record identifies the interpretation under which that payload is signed. Reusing payload bytes in two protocols does not entitle either protocol to reuse the other's signing statement. The purpose, schema, version, profile, and verifier context remain part of the preimage even when the payload reference is identical.

The checked [profile template](../../fabric-cryptographic-identity/profile-template.ncl) declares domain ID `molten.crypto.signature` and version `v1`. The inspected Rust `validate_signature_domain` checks token syntax, the domain schema, profile domain-version agreement, purpose admission, and reference syntax; it does not hard-code that domain ID. The template declaration and this generic validator are different enforcement surfaces. This article does not claim that every syntactically valid Rust request has independently passed the Nickel profile contract.

## Canonical identity is not an assertion by the caller

`CanonicalSignatureDomain` contains the structured domain, its Preserves value, bytes, and reference. Carrying all four representations is not sufficient by itself: they must agree. The shell's `require_canonical_domain` reconstructs the complete wrapper and compares it with the supplied value. This guards against a caller mutating bytes while leaving an attractive-looking reference or domain structure intact. The [integrity guards](../../../src/fabric_crypto_identity/file/parts/adapter/p003/body.rs) are therefore part of the signing and verification boundary, not optional presentation checks.

The adapter's `sign` signs the canonical domain bytes after handle admission. Its public outcome contains signature bytes and metadata, including purpose, generation, domain reference, payload reference, signer reference, verifier context, and signature reference. The outcome receives a separate canonical identity. Distinguish the three objects: the referenced payload, the signed domain, and the signature carrier. Their references are not interchangeable.

During verification, the shell validates the supplied canonical carrier, parses the public key and signature, and calls the public key verifier over the expected domain bytes. It then supplies the observed cryptographic result to core admission. The core compares expected purpose, payload, signer, context, profile, and generation with the observation. These are complementary layers: core comparisons cannot establish that Ed25519 verification actually happened, while signature verification alone cannot establish that the caller selected the intended purpose or currentness context. See [verification mechanics](../../../src/fabric_crypto_identity/file/parts/adapter/p001/body.rs) and [core admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs).

## Illustrative context substitution

Consider a valid federation-origin signature over payload reference P in verifier context C1. A consumer wants to use it in context C2 without changing P. If it constructs a canonical expected domain with C2, the signed bytes differ from the C1 preimage and cryptographic verification fails. If it changes only the carrier's metadata, canonical carrier reconstruction must still agree, and the verification request compares that metadata with the expected context. Rehashing a modified public carrier is not a substitute for producing a valid signature over the newly expected domain.

An analogous payload substitution replaces P with Q. It can fail both the metadata comparison and the cryptographic check. Multiple issue observations are not multiple independent cryptographic proofs; they are distinct rejection reasons at the boundary.

A more subtle case uses the same key and payload but a standalone `artifact_auth.statement.v1` preimage. The [governing identity document](../../fabric-cryptographic-identity.md) explicitly requires a separate `CryptographicObservation` for that exact statement. Successful legacy verification cannot be copied into the standalone result. Shared algorithm and shared business meaning do not make two canonical preimages equal.

## Review and verification guidance

Suggested verification changes one semantic field at a time while keeping other values fixed: payload, purpose, signer, verifier context, and version. Separately mutate cached bytes or the carrier reference to exercise reconstruction rather than domain admission. Existing [shell tests](../../../src/fabric_crypto_identity/parts/tests/p000/body.rs) provide production sign/verify scenarios; this article records source inspection, not a newly executed test run.

Reviewers should also locate the code that computes the payload reference from the canonical payload. The identity adapter accepts that reference; it does not fetch and semantically validate an external artifact simply because its reference has the correct grammar. [Nominal reference admission](../../nominal-authority-references.md) similarly preserves wire text and local categories without certifying the referenced content.

## Limits and non-claims

Canonicalization excludes Rust layout, rendered diagnostics, and transport framing from signature identity. It does not prove payload correctness, provenance, membership, or authority. Generation is carried in signature metadata and checked against supplied signer-generation context; the inspected domain constructor does not itself include a generation field. Avoid expanding the signed-preimage claim to every field surrounding a signature. Currentness evidence and policy selection remain separate responsibilities, and no production-readiness conclusion follows from canonical-byte agreement alone.

## Sources

- [Cryptographic identity contract and standalone boundary](../../fabric-cryptographic-identity.md)
- [Nominal authority references](../../nominal-authority-references.md)
- [Production profile template](../../fabric-cryptographic-identity/profile-template.ncl)
- [Canonical records and preimages](../../../src/fabric_crypto_identity/parts/canonical/p000/body.rs)
- [Canonical reconstruction guards](../../../src/fabric_crypto_identity/file/parts/adapter/p003/body.rs)
- [Core verification admission](../../../crates/molten-core/src/fabric_crypto_identity/admission.rs)
